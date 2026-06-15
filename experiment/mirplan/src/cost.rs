use crate::{LoweredFunction, LoweredInstructionKind, LoweredMemoryOp, LoweredProgram};
use mircap::FunctionId;
use mirspace::{BlockIx, FunctionIx};
use std::collections::HashSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramCostSummary {
    pub module_name: String,
    pub bounded: bool,
    pub totals: CostCounts,
    pub functions: Vec<FunctionCostSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionCostSummary {
    pub function: FunctionIx,
    pub id: FunctionId,
    pub name: String,
    pub bounded: bool,
    pub bound_kind: &'static str,
    pub counts: CostCounts,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CostCounts {
    pub instructions: u64,
    pub branches: u64,
    pub calls: u64,
    pub memory_reads: u64,
    pub memory_writes: u64,
    pub memory_addresses: u64,
    pub allocations: u64,
    pub traps: u64,
}

impl CostCounts {
    fn add_assign(&mut self, other: &CostCounts) {
        self.instructions += other.instructions;
        self.branches += other.branches;
        self.calls += other.calls;
        self.memory_reads += other.memory_reads;
        self.memory_writes += other.memory_writes;
        self.memory_addresses += other.memory_addresses;
        self.allocations += other.allocations;
        self.traps += other.traps;
    }
}

pub fn summarize_cost(program: &LoweredProgram) -> ProgramCostSummary {
    let functions = program
        .functions
        .iter()
        .map(summarize_function_cost)
        .collect::<Vec<_>>();
    let mut totals = CostCounts::default();
    for function in &functions {
        totals.add_assign(&function.counts);
    }
    ProgramCostSummary {
        module_name: program.module_name.clone(),
        bounded: functions.iter().all(|function| function.bounded),
        totals,
        functions,
    }
}

fn summarize_function_cost(function: &LoweredFunction) -> FunctionCostSummary {
    let mut bounded = function_cfg_is_acyclic(function);
    let mut bound_kind = if bounded {
        "acyclic-structural"
    } else {
        "cyclic-unknown"
    };

    let mut block_freqs = std::collections::HashMap::new();

    if !bounded {
        if let Some(loops) = try_prove_all_counted_loops(function) {
            bounded = true;
            bound_kind = "cyclic-counted-loop";
            for block in &function.blocks {
                let mut freq = 1u64;
                for &(header_ix, latch_ix, trip_count) in &loops {
                    if block.label.ix.0 >= header_ix.0 && block.label.ix.0 <= latch_ix.0 {
                        if block.label.ix == header_ix {
                            freq *= trip_count + 1;
                        } else {
                            freq *= trip_count;
                        }
                    }
                }
                block_freqs.insert(block.label.ix, freq);
            }
        }
    }

    let mut counts = CostCounts::default();
    for block in &function.blocks {
        let freq = *block_freqs.get(&block.label.ix).unwrap_or(&1);
        for instruction in &block.instructions {
            counts.instructions += freq;
            match &instruction.kind {
                LoweredInstructionKind::Branch { .. } => counts.branches += freq,
                LoweredInstructionKind::Call { .. } => counts.calls += freq,
                LoweredInstructionKind::Trap => counts.traps += freq,
                LoweredInstructionKind::Memory { op } => match op {
                    LoweredMemoryOp::Alloc => counts.allocations += freq,
                    LoweredMemoryOp::Load => counts.memory_reads += freq,
                    LoweredMemoryOp::Store => counts.memory_writes += freq,
                    LoweredMemoryOp::Address => counts.memory_addresses += freq,
                },
                LoweredInstructionKind::Return | LoweredInstructionKind::Value => {}
            }
        }
    }

    FunctionCostSummary {
        function: function.ix,
        id: function.id,
        name: function.name.clone(),
        bounded,
        bound_kind,
        counts,
    }
}

fn try_prove_all_counted_loops(function: &LoweredFunction) -> Option<Vec<(BlockIx, BlockIx, u64)>> {
    let mut backedges = Vec::new();
    for block in &function.blocks {
        for succ in &block.successors {
            if succ.block.ix.0 <= block.label.ix.0 {
                backedges.push((block.label.ix, succ.block.ix));
            }
        }
    }

    let mut loops = Vec::new();
    for &(latch_ix, header_ix) in &backedges {
        if let Some(trip_count) = try_prove_counted_loop_for_backedge(function, latch_ix, header_ix)
        {
            loops.push((header_ix, latch_ix, trip_count));
        } else {
            return None;
        }
    }
    Some(loops)
}

fn try_prove_counted_loop_for_backedge(
    function: &LoweredFunction,
    latch_ix: BlockIx,
    header_ix: BlockIx,
) -> Option<u64> {
    let header = function.blocks.iter().find(|b| b.label.ix == header_ix)?;
    let latch = function.blocks.iter().find(|b| b.label.ix == latch_ix)?;

    if header.successors.len() != 2 {
        return None;
    }

    let branch_insn = header.instructions.last()?;
    let (cond_val, true_target, false_target) = match &branch_insn.kind {
        crate::LoweredInstructionKind::Branch { targets, .. } if targets.len() == 2 => {
            let cond_val = branch_insn.reads.get(0)?;
            (cond_val, targets[0].block.ix, targets[1].block.ix)
        }
        _ => {
            return None;
        }
    };

    let true_in_loop = true_target.0 >= header_ix.0 && true_target.0 <= latch_ix.0;
    let false_in_loop = false_target.0 >= header_ix.0 && false_target.0 <= latch_ix.0;

    let body_ix = if true_in_loop && !false_in_loop {
        true_target
    } else if false_in_loop && !true_in_loop {
        false_target
    } else {
        return None;
    };

    let cond_insn = match header
        .instructions
        .iter()
        .find(|i| i.writes.contains(cond_val))
    {
        Some(i) => i,
        None => {
            return None;
        }
    };
    if cond_insn.reads.len() != 2 {
        return None;
    }
    let counter_val = &cond_insn.reads[0];
    let limit_val = &cond_insn.reads[1];

    let mut counter_init = None;
    let mut limit_init = None;
    for b in &function.blocks {
        if b.label.ix == header_ix || b.label.ix == body_ix {
            continue;
        }
        for i in &b.instructions {
            if i.writes.contains(counter_val) {
                counter_init = Some(i);
            }
            if i.writes.contains(limit_val) {
                limit_init = Some(i);
            }
        }
    }

    fn extract_const_u64(insn: &crate::lower::LoweredInstruction) -> Option<u64> {
        for op in &insn.operands {
            match op {
                crate::lower::LoweredOperand::ImmI32(v) => return Some(*v as i64 as u64),
                crate::lower::LoweredOperand::ImmU32(v) => return Some(*v as u64),
                crate::lower::LoweredOperand::ImmI64(v) => return Some(*v as u64),
                _ => {}
            }
        }
        None
    }

    let start_val = counter_init.and_then(extract_const_u64).unwrap_or(0);
    let limit = match limit_init.and_then(extract_const_u64) {
        Some(v) => v,
        None => {
            return None;
        }
    };

    let mut increment_insn = None;
    for i in &latch.instructions {
        if i.writes.contains(counter_val) {
            increment_insn = Some(i);
        }
    }
    let increment_insn = match increment_insn {
        Some(i) => i,
        None => {
            return None;
        }
    };

    let step_val = match increment_insn.reads.iter().find(|&v| v != counter_val) {
        Some(v) => v,
        None => {
            return None;
        }
    };

    let mut step_init = None;
    for b in &function.blocks {
        if b.label.ix == header_ix || b.label.ix == body_ix {
            continue;
        }
        for i in &b.instructions {
            if i.writes.contains(step_val) {
                step_init = Some(i);
            }
        }
    }
    let step = step_init.and_then(extract_const_u64).unwrap_or(1);

    if step == 0 {
        return None;
    }

    let trip_count = if start_val < limit {
        (limit - start_val + step - 1) / step
    } else {
        0
    };

    Some(trip_count)
}

fn function_cfg_is_acyclic(function: &LoweredFunction) -> bool {
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    for block in &function.blocks {
        if !visited.contains(&block.label.ix)
            && block_has_cycle(function, block.label.ix, &mut visiting, &mut visited)
        {
            return false;
        }
    }
    true
}

fn block_has_cycle(
    function: &LoweredFunction,
    block: BlockIx,
    visiting: &mut HashSet<BlockIx>,
    visited: &mut HashSet<BlockIx>,
) -> bool {
    if visiting.contains(&block) {
        return true;
    }
    if visited.contains(&block) {
        return false;
    }

    visiting.insert(block);
    if let Some(block_rec) = function
        .blocks
        .iter()
        .find(|block_rec| block_rec.label.ix == block)
    {
        for target in &block_rec.successors {
            if block_has_cycle(function, target.block.ix, visiting, visited) {
                return true;
            }
        }
    }
    visiting.remove(&block);
    visited.insert(block);
    false
}
