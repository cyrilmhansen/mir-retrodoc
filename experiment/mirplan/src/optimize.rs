use crate::lower::{
    LoweredBranchTarget, LoweredFunction, LoweredInstructionKind, LoweredOperand, LoweredProgram,
};
use mircap::Opcode;
use mirspace::EdgeKind;
use std::collections::{HashMap, HashSet};

pub fn optimize_program(mut program: LoweredProgram) -> LoweredProgram {
    for func in &mut program.functions {
        optimize_function(func);
    }
    program
}

fn optimize_function(func: &mut LoweredFunction) {
    // Run constant propagation and folding pass
    constant_folding_pass(func);

    // Run dead code elimination pass
    dead_code_elimination_pass(func);
}

fn constant_folding_pass(func: &mut LoweredFunction) {
    for block in &mut func.blocks {
        let mut constants: HashMap<mircap::ids::ValueId, LoweredOperand> = HashMap::new();
        for insn in &mut block.instructions {
            // 1. Constant Propagation: replace input value operands with known constant values
            for op in &mut insn.operands {
                if let LoweredOperand::Value(val) = op {
                    if let Some(c) = constants.get(&val.id) {
                        *op = c.clone();
                    }
                }
            }

            // Re-sync reads vector after propagation
            insn.reads = insn
                .operands
                .iter()
                .filter_map(|op| match op {
                    LoweredOperand::Value(val) => Some(val.clone()),
                    _ => None,
                })
                .collect();

            // 2. Constant Folding: fold arithmetic/logic operations
            let mut is_folded_constant = false;
            let mut folded_val = None;
            match insn.opcode {
                Opcode::AddI32
                | Opcode::SubI32
                | Opcode::MulI32
                | Opcode::AddU32
                | Opcode::SubU32
                | Opcode::MulU32
                | Opcode::EqI32
                | Opcode::NeI32
                | Opcode::LtI32
                | Opcode::EqU32
                | Opcode::NeU32
                | Opcode::LtU32
                | Opcode::LeU32
                | Opcode::GtU32
                | Opcode::GeU32
                | Opcode::AddI64
                | Opcode::SubI64
                | Opcode::MulI64
                | Opcode::EqI64
                | Opcode::NeI64
                | Opcode::LtI64 => {
                    if insn.operands.len() >= 2 {
                        let folded =
                            fold_binary_op(insn.opcode, &insn.operands[0], &insn.operands[1]);
                        if let Some(folded_op) = folded {
                            insn.opcode = match folded_op {
                                LoweredOperand::ImmI32(_) => Opcode::ConstI32,
                                LoweredOperand::ImmU32(_) => Opcode::ConstU32,
                                LoweredOperand::ImmI64(_) => Opcode::ConstI64,
                                _ => unreachable!(),
                            };
                            insn.operands = vec![folded_op.clone()];
                            insn.reads = Vec::new();

                            if let Some(dest) = insn.writes.first() {
                                is_folded_constant = true;
                                folded_val = Some((dest.id, folded_op));
                            }
                        }
                    }
                }
                Opcode::ConstI32 | Opcode::ConstU32 | Opcode::ConstI64 => {
                    if let Some(dest) = insn.writes.first() {
                        is_folded_constant = true;
                        folded_val = Some((dest.id, insn.operands[0].clone()));
                    }
                }
                Opcode::Copy => {
                    if let Some(dest) = insn.writes.first() {
                        if matches!(
                            insn.operands[0],
                            LoweredOperand::ImmI32(_) | LoweredOperand::ImmU32(_) | LoweredOperand::ImmI64(_)
                        ) {
                            is_folded_constant = true;
                            folded_val = Some((dest.id, insn.operands[0].clone()));
                        }
                    }
                }
                _ => {}
            }

            // Invalidate/insert constants
            if is_folded_constant {
                if let Some((dest_id, val)) = folded_val {
                    constants.insert(dest_id, val);
                }
            } else {
                for dest in &insn.writes {
                    constants.remove(&dest.id);
                }
            }
        }
    }

    // 3. Branch Folding Pass: fold BranchIf where condition is constant
    for block in &mut func.blocks {
        let mut folded_branch = None;
        for insn in &mut block.instructions {
            if insn.opcode == Opcode::BranchIf {
                if let LoweredOperand::ImmU32(cond_val) = &insn.operands[0] {
                    let taken_index = if *cond_val != 0 { 1 } else { 2 };
                    let taken_target = insn.operands[taken_index].clone();

                    insn.opcode = Opcode::Branch;
                    insn.operands = vec![taken_target.clone()];
                    insn.reads = Vec::new();

                    let LoweredOperand::Block(target_lbl) = taken_target else {
                        unreachable!()
                    };
                    insn.kind = LoweredInstructionKind::Branch {
                        targets: vec![LoweredBranchTarget {
                            kind: EdgeKind::Unconditional,
                            block: target_lbl.clone(),
                        }],
                    };

                    folded_branch = Some(target_lbl);
                } else if let LoweredOperand::ImmI32(cond_val) = &insn.operands[0] {
                    let taken_index = if *cond_val != 0 { 1 } else { 2 };
                    let taken_target = insn.operands[taken_index].clone();

                    insn.opcode = Opcode::Branch;
                    insn.operands = vec![taken_target.clone()];
                    insn.reads = Vec::new();

                    let LoweredOperand::Block(target_lbl) = taken_target else {
                        unreachable!()
                    };
                    insn.kind = LoweredInstructionKind::Branch {
                        targets: vec![LoweredBranchTarget {
                            kind: EdgeKind::Unconditional,
                            block: target_lbl.clone(),
                        }],
                    };

                    folded_branch = Some(target_lbl);
                }
            }
        }

        if let Some(target_lbl) = folded_branch {
            block.successors = vec![LoweredBranchTarget {
                kind: EdgeKind::Unconditional,
                block: target_lbl,
            }];
        }
    }
}

fn fold_binary_op(
    opcode: Opcode,
    lhs: &LoweredOperand,
    rhs: &LoweredOperand,
) -> Option<LoweredOperand> {
    match (lhs, rhs) {
        (LoweredOperand::ImmI32(a), LoweredOperand::ImmI32(b)) => match opcode {
            Opcode::AddI32 => Some(LoweredOperand::ImmI32(a.wrapping_add(*b))),
            Opcode::SubI32 => Some(LoweredOperand::ImmI32(a.wrapping_sub(*b))),
            Opcode::MulI32 => Some(LoweredOperand::ImmI32(a.wrapping_mul(*b))),
            Opcode::EqI32 => Some(LoweredOperand::ImmU32(if a == b { 1 } else { 0 })),
            Opcode::NeI32 => Some(LoweredOperand::ImmU32(if a != b { 1 } else { 0 })),
            Opcode::LtI32 => Some(LoweredOperand::ImmU32(if a < b { 1 } else { 0 })),
            _ => None,
        },
        (LoweredOperand::ImmU32(a), LoweredOperand::ImmU32(b)) => match opcode {
            Opcode::AddU32 => Some(LoweredOperand::ImmU32(a.wrapping_add(*b))),
            Opcode::SubU32 => Some(LoweredOperand::ImmU32(a.wrapping_sub(*b))),
            Opcode::MulU32 => Some(LoweredOperand::ImmU32(a.wrapping_mul(*b))),
            Opcode::EqU32 => Some(LoweredOperand::ImmU32(if a == b { 1 } else { 0 })),
            Opcode::NeU32 => Some(LoweredOperand::ImmU32(if a != b { 1 } else { 0 })),
            Opcode::LtU32 => Some(LoweredOperand::ImmU32(if a < b { 1 } else { 0 })),
            Opcode::LeU32 => Some(LoweredOperand::ImmU32(if a <= b { 1 } else { 0 })),
            Opcode::GtU32 => Some(LoweredOperand::ImmU32(if a > b { 1 } else { 0 })),
            Opcode::GeU32 => Some(LoweredOperand::ImmU32(if a >= b { 1 } else { 0 })),
            _ => None,
        },
        (LoweredOperand::ImmI64(a), LoweredOperand::ImmI64(b)) => match opcode {
            Opcode::AddI64 => Some(LoweredOperand::ImmI64(a.wrapping_add(*b))),
            Opcode::SubI64 => Some(LoweredOperand::ImmI64(a.wrapping_sub(*b))),
            Opcode::MulI64 => Some(LoweredOperand::ImmI64(a.wrapping_mul(*b))),
            Opcode::EqI64 => Some(LoweredOperand::ImmI32(if a == b { 1 } else { 0 })),
            Opcode::NeI64 => Some(LoweredOperand::ImmI32(if a != b { 1 } else { 0 })),
            Opcode::LtI64 => Some(LoweredOperand::ImmI32(if a < b { 1 } else { 0 })),
            _ => None,
        },
        _ => None,
    }
}

fn dead_code_elimination_pass(func: &mut LoweredFunction) {
    loop {
        let mut read_values = HashSet::new();
        for block in &func.blocks {
            for insn in &block.instructions {
                for r in &insn.reads {
                    read_values.insert(r.id);
                }
                for op in &insn.operands {
                    if let LoweredOperand::Value(val) = op {
                        read_values.insert(val.id);
                    }
                }
            }
        }

        let mut removed_any = false;
        for block in &mut func.blocks {
            let mut i = 0;
            while i < block.instructions.len() {
                let insn = &block.instructions[i];
                let is_dead = !insn.writes.is_empty()
                    && insn.writes.iter().all(|w| !read_values.contains(&w.id));

                if is_dead && !has_side_effects(insn.opcode) {
                    block.instructions.remove(i);
                    removed_any = true;
                } else {
                    i += 1;
                }
            }
        }

        if !removed_any {
            break;
        }
    }
}

fn has_side_effects(opcode: Opcode) -> bool {
    !matches!(
        opcode,
        Opcode::ConstI32
            | Opcode::ConstU32
            | Opcode::ConstI64
            | Opcode::Copy
            | Opcode::AddI32
            | Opcode::SubI32
            | Opcode::MulI32
            | Opcode::AddU32
            | Opcode::SubU32
            | Opcode::MulU32
            | Opcode::EqI32
            | Opcode::NeI32
            | Opcode::LtI32
            | Opcode::EqU32
            | Opcode::NeU32
            | Opcode::LtU32
            | Opcode::LeU32
            | Opcode::GtU32
            | Opcode::GeU32
            | Opcode::AddI64
            | Opcode::SubI64
            | Opcode::MulI64
            | Opcode::EqI64
            | Opcode::NeI64
            | Opcode::LtI64
    )
}
