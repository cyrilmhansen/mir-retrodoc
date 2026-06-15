use crate::profile::ExecutionProfile;
use crate::trap::ExecutionTrap;
use crate::value::Value;
use mircap::{BlockId, FunctionId};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceSnapshot {
    pub module_id: u32,
    pub module_name: String,
    pub entry_function: FunctionId,
    pub outcome: TraceOutcome,
    pub executed_instruction_count: u64,
    pub branch_count: u64,
    pub call_instruction_count: u64,
    pub address_instruction_count: u64,
    pub memory_read_count: u64,
    pub memory_write_count: u64,
    pub return_count: u64,
    pub trap_count: u64,
    pub functions: Vec<FunctionTrace>,
    pub call_edges: Vec<CallEdgeTrace>,
    pub maximum_call_depth_reached: usize,
    pub memory_profile: ExecutionProfile,
    pub allocation_count: u64,
    pub allocated_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceOutcome {
    NotRun,
    Returned(Vec<Value>),
    Trapped(ExecutionTrap),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionTrace {
    pub function: FunctionId,
    pub calls: u64,
    pub executed_instructions: u64,
    pub branches: u64,
    pub call_instructions: u64,
    pub address_instructions: u64,
    pub allocations: u64,
    pub memory_reads: u64,
    pub memory_writes: u64,
    pub returns: u64,
    pub traps: u64,
    pub blocks: Vec<BlockTrace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallEdgeTrace {
    pub caller: FunctionId,
    pub callee: FunctionId,
    pub calls: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockTrace {
    pub block: BlockId,
    pub entries: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct TraceState {
    pub entry_function: Option<FunctionId>,
    pub current_function: Option<FunctionId>,
    pub outcome: TraceOutcome,
    pub executed_instruction_count: u64,
    pub branch_count: u64,
    pub call_instruction_count: u64,
    pub address_instruction_count: u64,
    pub memory_read_count: u64,
    pub memory_write_count: u64,
    pub return_count: u64,
    pub trap_count: u64,
    pub function_calls: BTreeMap<FunctionId, u64>,
    pub function_instruction_counts: BTreeMap<FunctionId, u64>,
    pub function_branch_counts: BTreeMap<FunctionId, u64>,
    pub function_call_instruction_counts: BTreeMap<FunctionId, u64>,
    pub function_address_instruction_counts: BTreeMap<FunctionId, u64>,
    pub function_allocations: BTreeMap<FunctionId, u64>,
    pub function_memory_reads: BTreeMap<FunctionId, u64>,
    pub function_memory_writes: BTreeMap<FunctionId, u64>,
    pub function_returns: BTreeMap<FunctionId, u64>,
    pub function_traps: BTreeMap<FunctionId, u64>,
    pub call_edges: BTreeMap<(FunctionId, FunctionId), u64>,
    pub block_entries: BTreeMap<BlockId, u64>,
    pub maximum_call_depth_reached: usize,
}

impl Default for TraceState {
    fn default() -> Self {
        Self {
            entry_function: None,
            current_function: None,
            outcome: TraceOutcome::NotRun,
            executed_instruction_count: 0,
            branch_count: 0,
            call_instruction_count: 0,
            address_instruction_count: 0,
            memory_read_count: 0,
            memory_write_count: 0,
            return_count: 0,
            trap_count: 0,
            function_calls: BTreeMap::new(),
            function_instruction_counts: BTreeMap::new(),
            function_branch_counts: BTreeMap::new(),
            function_call_instruction_counts: BTreeMap::new(),
            function_address_instruction_counts: BTreeMap::new(),
            function_allocations: BTreeMap::new(),
            function_memory_reads: BTreeMap::new(),
            function_memory_writes: BTreeMap::new(),
            function_returns: BTreeMap::new(),
            function_traps: BTreeMap::new(),
            call_edges: BTreeMap::new(),
            block_entries: BTreeMap::new(),
            maximum_call_depth_reached: 0,
        }
    }
}

impl TraceState {
    pub fn record_function_call(&mut self, function: FunctionId, depth: usize) {
        *self.function_calls.entry(function).or_default() += 1;
        self.maximum_call_depth_reached = self.maximum_call_depth_reached.max(depth);
    }

    pub fn record_call_edge(&mut self, caller: FunctionId, callee: FunctionId) {
        *self.call_edges.entry((caller, callee)).or_default() += 1;
    }

    pub fn record_block_entry(&mut self, block: BlockId) {
        *self.block_entries.entry(block).or_default() += 1;
    }

    pub fn record_instruction(&mut self, function: FunctionId) {
        *self
            .function_instruction_counts
            .entry(function)
            .or_default() += 1;
    }

    pub fn record_branch(&mut self, function: FunctionId) {
        self.branch_count += 1;
        *self.function_branch_counts.entry(function).or_default() += 1;
    }

    pub fn record_call_instruction(&mut self, function: FunctionId) {
        self.call_instruction_count += 1;
        *self
            .function_call_instruction_counts
            .entry(function)
            .or_default() += 1;
    }

    pub fn record_address_instruction(&mut self, function: FunctionId) {
        self.address_instruction_count += 1;
        *self
            .function_address_instruction_counts
            .entry(function)
            .or_default() += 1;
    }

    pub fn record_allocation(&mut self, function: FunctionId) {
        *self.function_allocations.entry(function).or_default() += 1;
    }

    pub fn record_memory_read(&mut self, function: FunctionId) {
        self.memory_read_count += 1;
        *self.function_memory_reads.entry(function).or_default() += 1;
    }

    pub fn record_memory_write(&mut self, function: FunctionId) {
        self.memory_write_count += 1;
        *self.function_memory_writes.entry(function).or_default() += 1;
    }

    pub fn record_return(&mut self, function: FunctionId) {
        self.return_count += 1;
        *self.function_returns.entry(function).or_default() += 1;
    }

    pub fn record_trap(&mut self, function: FunctionId) {
        self.trap_count += 1;
        *self.function_traps.entry(function).or_default() += 1;
    }
}
