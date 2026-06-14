use mircap::ValueId;
use mirplan::LoweredFunction;
use std::collections::HashMap;

pub struct StackFrame {
    pub frame_size: i32,
    pub slots: HashMap<ValueId, i32>,
    pub ra_offset: i32,
    pub fp_offset: i32,
}

impl StackFrame {
    pub fn new(function: &LoweredFunction) -> Self {
        let mut slots = HashMap::new();
        let mut offset = -8; // First 8 bytes are reserved for saved ra and fp

        // Assign a stack slot offset for all virtual values written to in this function
        // (including parameters)
        // Ensure parameters get their slots first
        for param in &function.params {
            offset -= 4;
            slots.insert(param.id, offset);
        }

        // Assign slots to all other values written by instructions
        for block in &function.blocks {
            for instruction in &block.instructions {
                for write in &instruction.writes {
                    if !slots.contains_key(&write.id) {
                        offset -= 4;
                        slots.insert(write.id, offset);
                    }
                }
            }
        }

        // Align frame size to 16 bytes
        let total_needed = -offset;
        let frame_size = (total_needed + 15) & !15;

        // FP is saved at fp - 8, RA is saved at fp - 4
        let fp_offset = -8;
        let ra_offset = -4;

        Self {
            frame_size,
            slots,
            ra_offset,
            fp_offset,
        }
    }

    pub fn offset_of(&self, val: ValueId) -> i32 {
        *self.slots.get(&val).unwrap_or(&0)
    }
}
