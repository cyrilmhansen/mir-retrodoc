use crate::ids::{BlockId, FunctionId, InstructionId, SourceSpanId, SymbolId, TypeId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityRef {
    Module,
    Function(FunctionId),
    Block(BlockId),
    Instruction(InstructionId),
    Type(TypeId),
    Symbol(SymbolId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    InvalidFormat,
    UnsupportedVersion,
    DuplicateId,
    MissingReference,
    WrongParent,
    InvalidTerminator,
    TypeMismatch,
    UnsupportedFeature,
    MalformedOperand,
    MalformedFunctionSignature,
    UnresolvedSymbol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError {
    pub kind: ErrorKind,
    pub entity: EntityRef,
    pub source_span: Option<SourceSpanId>,
    pub message: String,
}

impl ValidationError {
    pub fn new(kind: ErrorKind, entity: EntityRef, message: impl Into<String>) -> Self {
        Self { kind, entity, source_span: None, message: message.into() }
    }
}

