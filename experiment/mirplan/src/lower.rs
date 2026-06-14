use crate::{BlockEdgePlan, CompilePlan, DataSegmentPlan, FunctionPlan, OperandPlan, ValuePlan};
use mircap::{BlockId, FunctionId, InstructionId, Opcode, SymbolId, TypeId, TypeKind, ValueId};
use mirspace::{BlockIx, EdgeKind, FunctionIx, InstructionIx, SymbolIx, ValueIx};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredProgram {
    pub module_name: String,
    pub data_segments: Vec<DataSegmentPlan>,
    pub functions: Vec<LoweredFunction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredFunction {
    pub ix: FunctionIx,
    pub id: FunctionId,
    pub name: String,
    pub params: Vec<LoweredValue>,
    pub results: Vec<TypeKind>,
    pub entry: LoweredBlockLabel,
    pub blocks: Vec<LoweredBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredBlock {
    pub label: LoweredBlockLabel,
    pub instructions: Vec<LoweredInstruction>,
    pub successors: Vec<LoweredBranchTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredInstruction {
    pub ix: InstructionIx,
    pub id: InstructionId,
    pub opcode: Opcode,
    pub kind: LoweredInstructionKind,
    pub writes: Vec<LoweredValue>,
    pub reads: Vec<LoweredValue>,
    pub operands: Vec<LoweredOperand>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoweredInstructionKind {
    Value,
    Branch { targets: Vec<LoweredBranchTarget> },
    Call { callee: LoweredFunctionRef },
    Return,
    Trap,
    Memory { op: LoweredMemoryOp },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredValue {
    pub ix: ValueIx,
    pub id: ValueId,
    pub type_kind: TypeKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredBlockLabel {
    pub ix: BlockIx,
    pub id: BlockId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredBranchTarget {
    pub kind: EdgeKind,
    pub block: LoweredBlockLabel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredFunctionRef {
    pub ix: FunctionIx,
    pub id: FunctionId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoweredOperand {
    Value(LoweredValue),
    ImmI32(i32),
    ImmU32(u32),
    Block(LoweredBlockLabel),
    Function(LoweredFunctionRef),
    Symbol {
        ix: SymbolIx,
        id: SymbolId,
        name: String,
    },
    Type(TypeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoweredMemoryOp {
    Alloc,
    Load,
    Store,
    Address,
}

pub fn lower_compile_plan(plan: &CompilePlan) -> LoweredProgram {
    LoweredProgram {
        module_name: plan.module_name.clone(),
        data_segments: plan.data_segments.clone(),
        functions: plan.functions.iter().map(lower_function).collect(),
    }
}

fn lower_function(function: &FunctionPlan) -> LoweredFunction {
    LoweredFunction {
        ix: function.ix,
        id: function.id,
        name: function.name.clone(),
        params: function.params.iter().map(lower_value).collect(),
        results: function.results.clone(),
        entry: LoweredBlockLabel {
            ix: function.entry,
            id: function
                .blocks
                .iter()
                .find(|block| block.ix == function.entry)
                .map(|block| block.id)
                .expect("compile plan entry block must exist"),
        },
        blocks: function.blocks.iter().map(lower_block).collect(),
    }
}

fn lower_block(block: &crate::BlockPlan) -> LoweredBlock {
    let successors = block.successors.iter().map(lower_branch_target).collect();
    LoweredBlock {
        label: LoweredBlockLabel {
            ix: block.ix,
            id: block.id,
        },
        instructions: block.instructions.iter().map(lower_instruction).collect(),
        successors,
    }
}

fn lower_instruction(instruction: &crate::InstructionPlan) -> LoweredInstruction {
    let reads = instruction
        .operands
        .iter()
        .filter_map(|operand| match operand {
            OperandPlan::Value(value) => Some(lower_value(value)),
            _ => None,
        })
        .collect();
    let writes = instruction.results.iter().map(lower_value).collect();

    LoweredInstruction {
        ix: instruction.ix,
        id: instruction.id,
        opcode: instruction.opcode,
        kind: lower_instruction_kind(instruction),
        writes,
        reads,
        operands: instruction.operands.iter().map(lower_operand).collect(),
    }
}

fn lower_instruction_kind(instruction: &crate::InstructionPlan) -> LoweredInstructionKind {
    match instruction.opcode {
        Opcode::Branch | Opcode::BranchIf => LoweredInstructionKind::Branch {
            targets: branch_targets_from_operands(&instruction.operands),
        },
        Opcode::Call => LoweredInstructionKind::Call {
            callee: callee_from_operands(&instruction.operands),
        },
        Opcode::Ret => LoweredInstructionKind::Return,
        Opcode::Trap => LoweredInstructionKind::Trap,
        opcode if memory_op(opcode).is_some() => LoweredInstructionKind::Memory {
            op: memory_op(opcode).expect("memory op checked above"),
        },
        _ => LoweredInstructionKind::Value,
    }
}

fn branch_targets_from_operands(operands: &[OperandPlan]) -> Vec<LoweredBranchTarget> {
    let block_operands = operands
        .iter()
        .filter_map(|operand| match operand {
            OperandPlan::Block { ix, id } => Some(LoweredBlockLabel { ix: *ix, id: *id }),
            _ => None,
        })
        .collect::<Vec<_>>();

    match block_operands.as_slice() {
        [target] => vec![LoweredBranchTarget {
            kind: EdgeKind::Unconditional,
            block: target.clone(),
        }],
        [true_target, false_target] => vec![
            LoweredBranchTarget {
                kind: EdgeKind::TrueBranch,
                block: true_target.clone(),
            },
            LoweredBranchTarget {
                kind: EdgeKind::FalseBranch,
                block: false_target.clone(),
            },
        ],
        _ => Vec::new(),
    }
}

fn callee_from_operands(operands: &[OperandPlan]) -> LoweredFunctionRef {
    operands
        .iter()
        .find_map(|operand| match operand {
            OperandPlan::Function { ix, id, name } => Some(LoweredFunctionRef {
                ix: *ix,
                id: *id,
                name: name.clone(),
            }),
            _ => None,
        })
        .expect("compile plan call instruction must have a direct callee")
}

fn lower_operand(operand: &OperandPlan) -> LoweredOperand {
    match operand {
        OperandPlan::Value(value) => LoweredOperand::Value(lower_value(value)),
        OperandPlan::ImmI32(value) => LoweredOperand::ImmI32(*value),
        OperandPlan::ImmU32(value) => LoweredOperand::ImmU32(*value),
        OperandPlan::Block { ix, id } => {
            LoweredOperand::Block(LoweredBlockLabel { ix: *ix, id: *id })
        }
        OperandPlan::Function { ix, id, name } => LoweredOperand::Function(LoweredFunctionRef {
            ix: *ix,
            id: *id,
            name: name.clone(),
        }),
        OperandPlan::Symbol { ix, id, name } => LoweredOperand::Symbol {
            ix: *ix,
            id: *id,
            name: name.clone(),
        },
        OperandPlan::Type(type_id) => LoweredOperand::Type(*type_id),
    }
}

fn lower_branch_target(target: &BlockEdgePlan) -> LoweredBranchTarget {
    LoweredBranchTarget {
        kind: target.kind,
        block: LoweredBlockLabel {
            ix: target.target,
            id: target.target_id,
        },
    }
}

fn lower_value(value: &ValuePlan) -> LoweredValue {
    LoweredValue {
        ix: value.ix,
        id: value.id,
        type_kind: value.type_kind,
    }
}

fn memory_op(opcode: Opcode) -> Option<LoweredMemoryOp> {
    match opcode {
        Opcode::Alloc => Some(LoweredMemoryOp::Alloc),
        Opcode::LoadI32 | Opcode::LoadU32 | Opcode::LoadU8 => Some(LoweredMemoryOp::Load),
        Opcode::StoreI32 | Opcode::StoreU32 | Opcode::StoreU8 => Some(LoweredMemoryOp::Store),
        Opcode::AddrAdd | Opcode::DataAddr => Some(LoweredMemoryOp::Address),
        _ => None,
    }
}

pub trait Backend {
    type Output;
    type Error: std::error::Error + Send + Sync + 'static;

    fn compile(&self, program: &LoweredProgram) -> Result<Self::Output, Self::Error>;
}
