use crate::ids::{BlockIx, FunctionIx, InstructionIx, OperandIx};
use crate::space::{BlockRec, FunctionRec, InstructionRec, ProgramSpace, ValueRec};
use mircap::ids::{BlockId, FunctionId, InstructionId, ValueId};

impl ProgramSpace {
    pub fn function_by_id(&self, id: FunctionId) -> Option<&FunctionRec> {
        let ix = self.maps.functions.get(&id)?;
        self.functions.get(ix.0)
    }

    pub fn block_by_id(&self, id: BlockId) -> Option<&BlockRec> {
        let ix = self.maps.blocks.get(&id)?;
        self.blocks.get(ix.0)
    }

    pub fn instruction_by_id(&self, id: InstructionId) -> Option<&InstructionRec> {
        let ix = self.maps.instructions.get(&id)?;
        self.instructions.get(ix.0)
    }

    pub fn value_by_id(&self, func_id: FunctionId, val_id: ValueId) -> Option<&ValueRec> {
        let ix = self.maps.values.get(&(func_id, val_id))?;
        self.values.get(ix.0)
    }

    pub fn successors(&self, block: BlockIx) -> Vec<BlockIx> {
        let block_rec = &self.blocks[block.0];
        block_rec
            .outgoing
            .iter()
            .map(|&edge_ix| {
                let edge = &self.edges[edge_ix.0];
                edge.target
            })
            .collect()
    }

    pub fn predecessors(&self, block: BlockIx) -> Vec<BlockIx> {
        let block_rec = &self.blocks[block.0];
        block_rec
            .incoming
            .iter()
            .map(|&edge_ix| {
                let edge = &self.edges[edge_ix.0];
                edge.source
            })
            .collect()
    }

    pub fn function_blocks(&self, function: FunctionIx) -> &[BlockIx] {
        &self.functions[function.0].blocks
    }

    pub fn block_instructions(&self, block: BlockIx) -> &[InstructionIx] {
        &self.blocks[block.0].instructions
    }

    pub fn instruction_operands(&self, instruction: InstructionIx) -> &[OperandIx] {
        &self.instructions[instruction.0].operands
    }
}
