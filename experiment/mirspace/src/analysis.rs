use crate::ids::{InstructionIx, ValueIx};
use crate::space::{OperandRec, ProgramSpace};

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
}
