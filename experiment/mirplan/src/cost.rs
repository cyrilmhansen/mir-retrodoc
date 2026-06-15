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
    let bounded = function_cfg_is_acyclic(function);
    let mut counts = CostCounts::default();
    for instruction in function
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
    {
        counts.instructions += 1;
        match &instruction.kind {
            LoweredInstructionKind::Branch { .. } => counts.branches += 1,
            LoweredInstructionKind::Call { .. } => counts.calls += 1,
            LoweredInstructionKind::Trap => counts.traps += 1,
            LoweredInstructionKind::Memory { op } => match op {
                LoweredMemoryOp::Alloc => counts.allocations += 1,
                LoweredMemoryOp::Load => counts.memory_reads += 1,
                LoweredMemoryOp::Store => counts.memory_writes += 1,
                LoweredMemoryOp::Address => counts.memory_addresses += 1,
            },
            LoweredInstructionKind::Return | LoweredInstructionKind::Value => {}
        }
    }

    FunctionCostSummary {
        function: function.ix,
        id: function.id,
        name: function.name.clone(),
        bounded,
        bound_kind: if bounded {
            "acyclic-structural"
        } else {
            "cyclic-unknown"
        },
        counts,
    }
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
