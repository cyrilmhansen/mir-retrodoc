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
    pub functions: Vec<FunctionTrace>,
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
    pub blocks: Vec<BlockTrace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockTrace {
    pub block: BlockId,
    pub entries: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct TraceState {
    pub entry_function: Option<FunctionId>,
    pub outcome: TraceOutcome,
    pub executed_instruction_count: u64,
    pub function_calls: BTreeMap<FunctionId, u64>,
    pub block_entries: BTreeMap<BlockId, u64>,
    pub maximum_call_depth_reached: usize,
}

impl Default for TraceState {
    fn default() -> Self {
        Self {
            entry_function: None,
            outcome: TraceOutcome::NotRun,
            executed_instruction_count: 0,
            function_calls: BTreeMap::new(),
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

    pub fn record_block_entry(&mut self, block: BlockId) {
        *self.block_entries.entry(block).or_default() += 1;
    }
}

