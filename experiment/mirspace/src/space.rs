use crate::ids::{BlockIx, EdgeIx, FunctionIx, InstructionIx, OperandIx, SymbolIx, ValueIx};
use mircap::ids::{BlockId, FunctionId, InstructionId, SymbolId, TypeId, ValueId};
use mircap::image::{Opcode, SymbolKind, TypeKind};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ValueRole {
    Parameter,
    Local,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum EdgeKind {
    Unconditional,
    TrueBranch,
    FalseBranch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionRec {
    pub id: FunctionId,
    pub symbol: SymbolId,
    pub params: Vec<ValueIx>,
    pub results: Vec<TypeKind>,
    pub blocks: Vec<BlockIx>,
    pub entry: BlockIx,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockRec {
    pub id: BlockId,
    pub parent: FunctionIx,
    pub instructions: Vec<InstructionIx>,
    pub outgoing: Vec<EdgeIx>,
    pub incoming: Vec<EdgeIx>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionRec {
    pub id: InstructionId,
    pub parent: BlockIx,
    pub opcode: Opcode,
    pub results: Vec<ValueIx>,
    pub operands: Vec<OperandIx>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueRec {
    pub id: ValueId,
    pub parent: FunctionIx,
    pub type_kind: TypeKind,
    pub role: ValueRole,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum OperandRec {
    Value(ValueIx),
    ImmI32(i32),
    ImmU32(u32),
    ImmI64(i64),
    Block(BlockIx),
    Function(FunctionIx),
    Symbol(SymbolIx),
    Type(TypeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgeRec {
    pub source: BlockIx,
    pub target: BlockIx,
    pub kind: EdgeKind,
    pub terminator: InstructionIx,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataSegmentRec {
    pub symbol: SymbolId,
    pub offset: u32,
    pub length: u32,
    pub bytes: Vec<u8>,
    pub zero_fill: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolRec {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
}

#[derive(Clone, Debug, Default)]
pub struct IdMaps {
    pub functions: HashMap<FunctionId, FunctionIx>,
    pub blocks: HashMap<BlockId, BlockIx>,
    pub instructions: HashMap<InstructionId, InstructionIx>,
    pub values: HashMap<(FunctionId, ValueId), ValueIx>,
    pub symbols: HashMap<SymbolId, SymbolIx>,
}

#[derive(Clone, Debug)]
pub struct ProgramSpace {
    pub name: String,
    pub functions: Vec<FunctionRec>,
    pub blocks: Vec<BlockRec>,
    pub instructions: Vec<InstructionRec>,
    pub operands: Vec<OperandRec>,
    pub values: Vec<ValueRec>,
    pub edges: Vec<EdgeRec>,
    pub data_segments: Vec<DataSegmentRec>,
    pub symbols: Vec<SymbolRec>,
    pub maps: IdMaps,
}
