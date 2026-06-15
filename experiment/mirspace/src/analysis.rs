use crate::ids::{BlockIx, FunctionIx, InstructionIx, ValueIx};
use crate::space::{OperandRec, ProgramSpace};
use mircap::Opcode;
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValueDefUse {
    pub definitions: Vec<InstructionIx>,
    pub uses: Vec<InstructionIx>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefUseIndex {
    values: Vec<ValueDefUse>,
}

impl DefUseIndex {
    pub fn value(&self, value: ValueIx) -> Option<&ValueDefUse> {
        self.values.get(value.0)
    }

    pub fn definitions_of(&self, value: ValueIx) -> &[InstructionIx] {
        self.value(value)
            .map(|entry| entry.definitions.as_slice())
            .unwrap_or(&[])
    }

    pub fn uses_of(&self, value: ValueIx) -> &[InstructionIx] {
        self.value(value)
            .map(|entry| entry.uses.as_slice())
            .unwrap_or(&[])
    }
}

impl ProgramSpace {
    pub fn def_use_index(&self) -> DefUseIndex {
        let mut values = vec![ValueDefUse::default(); self.values.len()];

        for (insn_idx, insn) in self.instructions.iter().enumerate() {
            let insn_ix = InstructionIx(insn_idx);

            for &result in &insn.results {
                values[result.0].definitions.push(insn_ix);
            }

            for &operand in &insn.operands {
                if let OperandRec::Value(value) = self.operands[operand.0] {
                    values[value.0].uses.push(insn_ix);
                }
            }
        }

        DefUseIndex { values }
    }

    pub fn function_effect_summaries(&self) -> Vec<FunctionEffectSummary> {
        self.functions
            .iter()
            .enumerate()
            .map(|(idx, _)| self.function_effect_summary(FunctionIx(idx)))
            .collect()
    }

    pub fn function_effect_summary(&self, function: FunctionIx) -> FunctionEffectSummary {
        let function_rec = &self.functions[function.0];
        let mut summary = FunctionEffectSummary {
            function,
            allocates: false,
            reads_memory: false,
            writes_memory: false,
            may_trap: false,
            calls: Vec::new(),
            acyclic_cfg: self.function_cfg_is_acyclic(function),
            guaranteed_terminates_trivially: false,
            pure_candidate: false,
        };

        let mut has_return = false;
        for &block_ix in &function_rec.blocks {
            let block = &self.blocks[block_ix.0];
            for &insn_ix in &block.instructions {
                let insn = &self.instructions[insn_ix.0];
                match insn.opcode {
                    Opcode::Alloc => {
                        summary.allocates = true;
                        summary.may_trap = true;
                    }
                    Opcode::LoadI32 | Opcode::LoadU32 | Opcode::LoadI64 | Opcode::LoadU8 => {
                        summary.reads_memory = true;
                        summary.may_trap = true;
                    }
                    Opcode::StoreI32 | Opcode::StoreU32 | Opcode::StoreI64 | Opcode::StoreU8 => {
                        summary.writes_memory = true;
                        summary.may_trap = true;
                    }
                    Opcode::AddrAdd | Opcode::DataAddr => {
                        summary.may_trap = true;
                    }
                    Opcode::Call => {
                        if let Some(OperandRec::Function(callee)) = insn
                            .operands
                            .first()
                            .map(|operand| self.operands[operand.0])
                        {
                            summary.calls.push(callee);
                        }
                    }
                    Opcode::Ret => {
                        has_return = true;
                    }
                    Opcode::Trap => {
                        summary.may_trap = true;
                    }
                    Opcode::UnsupportedIndirectCall
                    | Opcode::EqF32
                    | Opcode::NeF32
                    | Opcode::LtF32
                    | Opcode::LeF32
                    | Opcode::GtF32
                    | Opcode::GeF32
                    | Opcode::EqF64
                    | Opcode::NeF64
                    | Opcode::LtF64
                    | Opcode::LeF64
                    | Opcode::GtF64
                    | Opcode::GeF64
                    | Opcode::I32ToF32
                    | Opcode::F32ToI32
                    | Opcode::I32ToF64
                    | Opcode::F64ToI32
                    | Opcode::F32ToF64
                    | Opcode::F64ToF32 => {
                        summary.may_trap = true;
                    }
                    _ => {}
                }
            }
        }

        summary.calls.sort_by_key(|callee| callee.0);
        summary.calls.dedup();
        summary.guaranteed_terminates_trivially =
            summary.acyclic_cfg && has_return && summary.calls.is_empty();
        summary.pure_candidate = !summary.allocates
            && !summary.reads_memory
            && !summary.writes_memory
            && !summary.may_trap
            && summary.calls.is_empty()
            && summary.guaranteed_terminates_trivially;
        summary
    }

    fn function_cfg_is_acyclic(&self, function: FunctionIx) -> bool {
        let function_rec = &self.functions[function.0];
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();

        for &block in &function_rec.blocks {
            if !visited.contains(&block) && self.block_has_cycle(block, &mut visiting, &mut visited)
            {
                return false;
            }
        }
        true
    }

    fn block_has_cycle(
        &self,
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
        for &edge_ix in &self.blocks[block.0].outgoing {
            let target = self.edges[edge_ix.0].target;
            if self.block_has_cycle(target, visiting, visited) {
                return true;
            }
        }
        visiting.remove(&block);
        visited.insert(block);
        false
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionEffectSummary {
    pub function: FunctionIx,
    pub allocates: bool,
    pub reads_memory: bool,
    pub writes_memory: bool,
    pub may_trap: bool,
    pub calls: Vec<FunctionIx>,
    pub acyclic_cfg: bool,
    pub guaranteed_terminates_trivially: bool,
    pub pure_candidate: bool,
}
