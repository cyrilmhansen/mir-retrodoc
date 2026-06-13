use crate::error::SpaceError;
use crate::ids::{BlockIx, EdgeIx};
use crate::space::{EdgeKind, EdgeRec, OperandRec, ProgramSpace};
use mircap::image::Opcode;

impl ProgramSpace {
    pub(crate) fn build_cfg(&mut self) -> Result<(), SpaceError> {
        let mut edges = Vec::new();

        let mut block_outgoing = vec![Vec::new(); self.blocks.len()];
        let mut block_incoming = vec![Vec::new(); self.blocks.len()];

        for b_idx in 0..self.blocks.len() {
            let block_ix = BlockIx(b_idx);
            let block_rec = &self.blocks[b_idx];

            if block_rec.instructions.is_empty() {
                return Err(SpaceError::Inconsistency(format!(
                    "Block {} has no instructions",
                    block_rec.id.0
                )));
            }

            let term_ix = *block_rec.instructions.last().unwrap();
            let term_rec = &self.instructions[term_ix.0];

            match term_rec.opcode {
                Opcode::Branch => {
                    if term_rec.operands.is_empty() {
                        return Err(SpaceError::Inconsistency(format!(
                            "Branch instruction {} in block {} has no operands",
                            term_rec.id.0, block_rec.id.0
                        )));
                    }
                    let op_ix = term_rec.operands[0];
                    let OperandRec::Block(target_block_ix) = self.operands[op_ix.0] else {
                        return Err(SpaceError::Inconsistency(format!(
                            "Invalid branch target operand in block {}",
                            block_rec.id.0
                        )));
                    };

                    let edge_ix = EdgeIx(edges.len());
                    edges.push(EdgeRec {
                        source: block_ix,
                        target: target_block_ix,
                        kind: EdgeKind::Unconditional,
                        terminator: term_ix,
                    });
                    block_outgoing[block_ix.0].push(edge_ix);
                    block_incoming[target_block_ix.0].push(edge_ix);
                }
                Opcode::BranchIf => {
                    if term_rec.operands.len() < 3 {
                        return Err(SpaceError::Inconsistency(format!(
                            "BranchIf instruction {} in block {} has fewer than 3 operands",
                            term_rec.id.0, block_rec.id.0
                        )));
                    }

                    let op_true_ix = term_rec.operands[1];
                    let OperandRec::Block(true_block_ix) = self.operands[op_true_ix.0] else {
                        return Err(SpaceError::Inconsistency(format!(
                            "Invalid branch_if true target operand in block {}",
                            block_rec.id.0
                        )));
                    };

                    let op_false_ix = term_rec.operands[2];
                    let OperandRec::Block(false_block_ix) = self.operands[op_false_ix.0] else {
                        return Err(SpaceError::Inconsistency(format!(
                            "Invalid branch_if false target operand in block {}",
                            block_rec.id.0
                        )));
                    };

                    let edge_true_ix = EdgeIx(edges.len());
                    edges.push(EdgeRec {
                        source: block_ix,
                        target: true_block_ix,
                        kind: EdgeKind::TrueBranch,
                        terminator: term_ix,
                    });
                    block_outgoing[block_ix.0].push(edge_true_ix);
                    block_incoming[true_block_ix.0].push(edge_true_ix);

                    let edge_false_ix = EdgeIx(edges.len());
                    edges.push(EdgeRec {
                        source: block_ix,
                        target: false_block_ix,
                        kind: EdgeKind::FalseBranch,
                        terminator: term_ix,
                    });
                    block_outgoing[block_ix.0].push(edge_false_ix);
                    block_incoming[false_block_ix.0].push(edge_false_ix);
                }
                Opcode::Ret | Opcode::Trap => {
                    // Terminators with no outgoing edges
                }
                _ => {
                    return Err(SpaceError::Inconsistency(format!(
                        "Non-terminator opcode {:?} at the end of block {}",
                        term_rec.opcode, block_rec.id.0
                    )));
                }
            }
        }

        self.edges = edges;
        for b_idx in 0..self.blocks.len() {
            self.blocks[b_idx].outgoing = block_outgoing[b_idx].clone();
            self.blocks[b_idx].incoming = block_incoming[b_idx].clone();
        }

        Ok(())
    }
}
