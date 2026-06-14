use crate::register::Register;
use mircap::ValueId;
use mirplan::{LoweredFunction, LoweredOperand};
use std::collections::HashMap;

pub struct StackFrame {
    pub frame_size: i32,
    pub slots: HashMap<ValueId, i32>,
    pub ra_offset: i32,
    pub fp_offset: i32,
    pub registers: HashMap<ValueId, Register>,
    pub used_saved_regs: Vec<Register>,
    pub saved_reg_offsets: HashMap<Register, i32>,
    pub val_types: HashMap<ValueId, mircap::TypeKind>,
}

impl StackFrame {
    pub fn new(function: &LoweredFunction) -> Self {
        // 1. Assign sequential indices to instructions and find block boundaries
        let mut insn_count = 0;
        let mut block_ranges = HashMap::new(); // BlockId -> (start_idx, end_idx)

        for block in &function.blocks {
            let start = insn_count;
            insn_count += block.instructions.len();
            let end = if insn_count > start {
                insn_count - 1
            } else {
                start
            };
            block_ranges.insert(block.label.id, (start, end));
        }

        // 2. Compute liveness intervals
        let mut start_idx = HashMap::new();
        let mut end_idx = HashMap::new();

        // Parameters are live from the start
        for param in &function.params {
            start_idx.insert(param.id, 0);
            end_idx.insert(param.id, 0);
        }

        let mut current_idx = 0;
        for block in &function.blocks {
            for insn in &block.instructions {
                for write in &insn.writes {
                    start_idx.entry(write.id).or_insert(current_idx);
                }
                for read in &insn.reads {
                    start_idx.entry(read.id).or_insert(current_idx);
                    let e = end_idx.entry(read.id).or_insert(current_idx);
                    if current_idx > *e {
                        *e = current_idx;
                    }
                }
                for op in &insn.operands {
                    if let LoweredOperand::Value(val) = op {
                        start_idx.entry(val.id).or_insert(current_idx);
                        let e = end_idx.entry(val.id).or_insert(current_idx);
                        if current_idx > *e {
                            *e = current_idx;
                        }
                    }
                }
                current_idx += 1;
            }
        }

        // 3. Extend intervals across back-edges (loops)
        for block in &function.blocks {
            let &(_, end_curr) = block_ranges.get(&block.label.id).unwrap_or(&(0, 0));
            for successor in &block.successors {
                let &(start_succ, _) = block_ranges.get(&successor.block.id).unwrap_or(&(0, 0));
                // If back-edge
                if start_succ < end_curr {
                    for (&val_id, &start) in &start_idx {
                        if start <= end_curr {
                            if let Some(end) = end_idx.get_mut(&val_id) {
                                if *end >= start_succ {
                                    if end_curr > *end {
                                        *end = end_curr;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Linear Scan Register Allocation
        // Available registers pool: s1 to s11
        let all_regs = vec![
            Register::S1,
            Register::S2,
            Register::S3,
            Register::S4,
            Register::S5,
            Register::S6,
            Register::S7,
            Register::S8,
            Register::S9,
            Register::S10,
            Register::S11,
        ];
        let mut free_registers = all_regs.clone();
        free_registers.reverse(); // So we pop S1 first

        let mut val_types = HashMap::new();
        for param in &function.params {
            val_types.insert(param.id, param.type_kind);
        }
        for block in &function.blocks {
            for insn in &block.instructions {
                for write in &insn.writes {
                    val_types.insert(write.id, write.type_kind);
                }
            }
        }

        // Collect and sort intervals by start time, filtering out i64 values
        let mut intervals: Vec<(ValueId, usize, usize)> = start_idx
            .keys()
            .filter(|&&id| val_types.get(&id) != Some(&mircap::TypeKind::I64))
            .map(|&id| {
                let start = *start_idx.get(&id).unwrap();
                let end = *end_idx.get(&id).unwrap_or(&start);
                (id, start, end)
            })
            .collect();
        intervals.sort_by_key(|&(_, start, _)| start);

        let mut registers = HashMap::new();
        let mut spilled_vars = Vec::new();
        for &id in start_idx.keys() {
            if val_types.get(&id) == Some(&mircap::TypeKind::I64) {
                spilled_vars.push(id);
            }
        }
        let mut active: Vec<(ValueId, usize, usize, Register)> = Vec::new();

        for (val_id, start, end) in intervals {
            // Expire old intervals
            active.retain(|&(_, _, active_end, reg)| {
                if active_end < start {
                    free_registers.push(reg);
                    false
                } else {
                    true
                }
            });

            // Allocate
            if let Some(reg) = free_registers.pop() {
                registers.insert(val_id, reg);
                active.push((val_id, start, end, reg));
                active.sort_by_key(|&(_, _, active_end, _)| active_end);
            } else {
                // Spill the one that ends furthest in the future
                let mut spill_idx = None;
                let mut max_end = end;
                for (i, &(_, _, active_end, _)) in active.iter().enumerate() {
                    if active_end > max_end {
                        max_end = active_end;
                        spill_idx = Some(i);
                    }
                }

                if let Some(idx) = spill_idx {
                    let (spilled_id, _, _, reg) = active.remove(idx);
                    registers.remove(&spilled_id);
                    spilled_vars.push(spilled_id);

                    registers.insert(val_id, reg);
                    active.push((val_id, start, end, reg));
                    active.sort_by_key(|&(_, _, active_end, _)| active_end);
                } else {
                    spilled_vars.push(val_id);
                }
            }
        }

        // 5. Layout Stack Frame
        // Collect used saved registers
        let mut used_saved_regs: Vec<Register> = registers.values().cloned().collect();
        used_saved_regs.sort_by_key(|&r| r as usize);
        used_saved_regs.dedup();

        let mut saved_reg_offsets = HashMap::new();
        let mut offset = -8; // Saved RA at -4, FP at -8

        for &reg in &used_saved_regs {
            offset -= 4;
            saved_reg_offsets.insert(reg, offset);
        }

        // Allocate slots for spilled variables
        let mut slots = HashMap::new();
        for val_id in spilled_vars {
            let is_i64 = val_types.get(&val_id) == Some(&mircap::TypeKind::I64);
            if is_i64 {
                offset -= 8;
                slots.insert(val_id, offset);
            } else {
                offset -= 4;
                slots.insert(val_id, offset);
            }
        }

        // Ensure parameters that were spilled (or not allocated) also have slots.
        for param in &function.params {
            if !registers.contains_key(&param.id) && !slots.contains_key(&param.id) {
                let is_i64 = param.type_kind == mircap::TypeKind::I64;
                if is_i64 {
                    offset -= 8;
                    slots.insert(param.id, offset);
                } else {
                    offset -= 4;
                    slots.insert(param.id, offset);
                }
            }
        }

        // Frame size aligned to 16 bytes
        let total_needed = -offset;
        let frame_size = (total_needed + 15) & !15;

        Self {
            frame_size,
            slots,
            ra_offset: -4,
            fp_offset: -8,
            registers,
            used_saved_regs,
            saved_reg_offsets,
            val_types,
        }
    }

    pub fn offset_of(&self, val: ValueId) -> i32 {
        *self.slots.get(&val).unwrap_or(&0)
    }
}
