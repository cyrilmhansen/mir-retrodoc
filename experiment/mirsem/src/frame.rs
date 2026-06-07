use crate::value::Value;
use mircap::{BlockId, FunctionId, ValueId};

#[derive(Clone, Debug)]
pub struct Frame {
    pub function: FunctionId,
    pub current_block: BlockId,
    pub instruction_position: usize,
    pub values: Vec<Option<Value>>,
    pub return_destinations: Vec<ValueId>,
}

impl Frame {
    pub fn new(function: FunctionId, current_block: BlockId, value_count: u32, return_destinations: Vec<ValueId>) -> Self {
        Self {
            function,
            current_block,
            instruction_position: 0,
            values: vec![None; value_count as usize],
            return_destinations,
        }
    }

    pub fn read(&self, value: ValueId) -> Option<Value> {
        self.values.get(value.0 as usize).and_then(Clone::clone)
    }

    pub fn write(&mut self, value: ValueId, runtime_value: Value) -> bool {
        if let Some(slot) = self.values.get_mut(value.0 as usize) {
            *slot = Some(runtime_value);
            true
        } else {
            false
        }
    }
}

