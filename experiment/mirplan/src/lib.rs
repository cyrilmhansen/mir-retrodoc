mod format;
mod lower;

use mircap::{BlockId, FunctionId, InstructionId, Opcode, SymbolId, TypeKind, ValueId};
use mirspace::{
    BlockIx, EdgeKind, FunctionIx, InstructionIx, OperandRec, ProgramSpace, SymbolIx, ValueIx,
};

pub use format::format_plan;
pub use lower::{
    lower_compile_plan, LoweredBlock, LoweredBlockLabel, LoweredBranchTarget, LoweredFunction,
    LoweredFunctionRef, LoweredInstruction, LoweredInstructionKind, LoweredMemoryOp,
    LoweredProgram, LoweredValue,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilePlan {
    pub module_name: String,
    pub functions: Vec<FunctionPlan>,
    pub data_segments: Vec<DataSegmentPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionPlan {
    pub ix: FunctionIx,
    pub id: FunctionId,
    pub symbol: SymbolId,
    pub name: String,
    pub params: Vec<ValuePlan>,
    pub results: Vec<TypeKind>,
    pub entry: BlockIx,
    pub blocks: Vec<BlockPlan>,
    pub call_sites: Vec<CallSitePlan>,
    pub memory_ops: Vec<MemoryOpPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockPlan {
    pub ix: BlockIx,
    pub id: BlockId,
    pub instructions: Vec<InstructionPlan>,
    pub successors: Vec<BlockEdgePlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionPlan {
    pub ix: InstructionIx,
    pub id: InstructionId,
    pub opcode: Opcode,
    pub results: Vec<ValuePlan>,
    pub operands: Vec<OperandPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValuePlan {
    pub ix: ValueIx,
    pub id: ValueId,
    pub type_kind: TypeKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperandPlan {
    Value(ValuePlan),
    ImmI32(i32),
    ImmU32(u32),
    Block {
        ix: BlockIx,
        id: BlockId,
    },
    Function {
        ix: FunctionIx,
        id: FunctionId,
        name: String,
    },
    Symbol {
        ix: SymbolIx,
        id: SymbolId,
        name: String,
    },
    Type(mircap::TypeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockEdgePlan {
    pub kind: EdgeKind,
    pub target: BlockIx,
    pub target_id: BlockId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallSitePlan {
    pub instruction: InstructionIx,
    pub instruction_id: InstructionId,
    pub callee: FunctionIx,
    pub callee_id: FunctionId,
    pub callee_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryOpPlan {
    pub instruction: InstructionIx,
    pub instruction_id: InstructionId,
    pub opcode: Opcode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataSegmentPlan {
    pub symbol: SymbolId,
    pub name: String,
    pub offset: u32,
    pub length: u32,
}

pub fn build_compile_plan(space: &ProgramSpace) -> CompilePlan {
    let functions = space
        .functions
        .iter()
        .enumerate()
        .map(|(idx, _)| build_function_plan(space, FunctionIx(idx)))
        .collect();

    let data_segments = space
        .data_segments
        .iter()
        .map(|segment| {
            let symbol_ix = space.maps.symbols[&segment.symbol];
            let symbol = &space.symbols[symbol_ix.0];
            DataSegmentPlan {
                symbol: segment.symbol,
                name: symbol.name.clone(),
                offset: segment.offset,
                length: segment.length,
            }
        })
        .collect();

    CompilePlan {
        module_name: space.name.clone(),
        functions,
        data_segments,
    }
}

fn build_function_plan(space: &ProgramSpace, function_ix: FunctionIx) -> FunctionPlan {
    let function = &space.functions[function_ix.0];
    let symbol = &space.symbols[space.maps.symbols[&function.symbol].0];
    let mut call_sites = Vec::new();
    let mut memory_ops = Vec::new();

    let blocks = function
        .blocks
        .iter()
        .map(|&block_ix| {
            let block = &space.blocks[block_ix.0];
            let instructions = block
                .instructions
                .iter()
                .map(|&instruction_ix| {
                    let instruction = &space.instructions[instruction_ix.0];
                    if instruction.opcode == Opcode::Call {
                        if let Some(CallSitePlan {
                            instruction,
                            instruction_id,
                            callee,
                            callee_id,
                            callee_name,
                        }) = call_site(space, instruction_ix)
                        {
                            call_sites.push(CallSitePlan {
                                instruction,
                                instruction_id,
                                callee,
                                callee_id,
                                callee_name,
                            });
                        }
                    }

                    if is_memory_op(instruction.opcode) {
                        memory_ops.push(MemoryOpPlan {
                            instruction: instruction_ix,
                            instruction_id: instruction.id,
                            opcode: instruction.opcode,
                        });
                    }

                    InstructionPlan {
                        ix: instruction_ix,
                        id: instruction.id,
                        opcode: instruction.opcode,
                        results: instruction
                            .results
                            .iter()
                            .map(|&value| value_plan(space, value))
                            .collect(),
                        operands: instruction
                            .operands
                            .iter()
                            .map(|&operand| match space.operands[operand.0] {
                                OperandRec::Value(value) => {
                                    OperandPlan::Value(value_plan(space, value))
                                }
                                OperandRec::ImmI32(value) => OperandPlan::ImmI32(value),
                                OperandRec::ImmU32(value) => OperandPlan::ImmU32(value),
                                OperandRec::Block(block) => OperandPlan::Block {
                                    ix: block,
                                    id: space.blocks[block.0].id,
                                },
                                OperandRec::Function(function) => {
                                    let rec = &space.functions[function.0];
                                    let symbol = &space.symbols[space.maps.symbols[&rec.symbol].0];
                                    OperandPlan::Function {
                                        ix: function,
                                        id: rec.id,
                                        name: symbol.name.clone(),
                                    }
                                }
                                OperandRec::Symbol(symbol) => {
                                    let rec = &space.symbols[symbol.0];
                                    OperandPlan::Symbol {
                                        ix: symbol,
                                        id: rec.id,
                                        name: rec.name.clone(),
                                    }
                                }
                                OperandRec::Type(type_id) => OperandPlan::Type(type_id),
                            })
                            .collect(),
                    }
                })
                .collect();

            let successors = block
                .outgoing
                .iter()
                .map(|&edge_ix| {
                    let edge = &space.edges[edge_ix.0];
                    BlockEdgePlan {
                        kind: edge.kind,
                        target: edge.target,
                        target_id: space.blocks[edge.target.0].id,
                    }
                })
                .collect();

            BlockPlan {
                ix: block_ix,
                id: block.id,
                instructions,
                successors,
            }
        })
        .collect();

    FunctionPlan {
        ix: function_ix,
        id: function.id,
        symbol: function.symbol,
        name: symbol.name.clone(),
        params: function
            .params
            .iter()
            .map(|&value| value_plan(space, value))
            .collect(),
        results: function.results.clone(),
        entry: function.entry,
        blocks,
        call_sites,
        memory_ops,
    }
}

fn value_plan(space: &ProgramSpace, value: ValueIx) -> ValuePlan {
    let rec = &space.values[value.0];
    ValuePlan {
        ix: value,
        id: rec.id,
        type_kind: rec.type_kind,
    }
}

fn call_site(space: &ProgramSpace, instruction_ix: InstructionIx) -> Option<CallSitePlan> {
    let instruction = &space.instructions[instruction_ix.0];
    let first_operand = instruction.operands.first()?;
    let OperandRec::Function(callee) = space.operands[first_operand.0] else {
        return None;
    };
    let rec = &space.functions[callee.0];
    let symbol = &space.symbols[space.maps.symbols[&rec.symbol].0];
    Some(CallSitePlan {
        instruction: instruction_ix,
        instruction_id: instruction.id,
        callee,
        callee_id: rec.id,
        callee_name: symbol.name.clone(),
    })
}

fn is_memory_op(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::Alloc
            | Opcode::LoadI32
            | Opcode::LoadU32
            | Opcode::StoreI32
            | Opcode::StoreU32
            | Opcode::LoadU8
            | Opcode::StoreU8
            | Opcode::AddrAdd
            | Opcode::DataAddr
    )
}
