use crate::ids::{BlockId, FunctionId, InstructionId, SourceSpanId, SymbolId, TypeId, ValueId};
use crate::loader::{self, LoadError};
use crate::validate::{Validate, ValidationReport};

pub const FORMAT_SCHEMA_NAME: &str = "mircap";
pub const FORMAT_VERSION: u32 = 0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    pub schema_name: String,
    pub format_version: u32,
    pub producer_name: String,
    pub producer_version: String,
    pub source_language: Option<String>,
    pub target_assumptions: Option<String>,
    pub feature_flags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Module {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeKind {
    Void,
    I32,
    U32,
    Addr32,
    UnsupportedI64,
    UnsupportedFloat,
    UnsupportedLongDouble,
    UnsupportedAggregate,
    UnsupportedVarargs,
    UnsupportedHostCAbi,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeDef {
    pub id: TypeId,
    pub kind: TypeKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Function,
    Data,
    RuntimeHelper,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    pub id: FunctionId,
    pub symbol: SymbolId,
    pub params: Vec<TypeId>,
    pub results: Vec<TypeId>,
    pub value_count: u32,
    pub value_types: Vec<TypeId>,
    pub blocks: Vec<BlockId>,
    pub flags: u32,
    pub source_span: Option<SourceSpanId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub id: BlockId,
    pub parent: FunctionId,
    pub instructions: Vec<InstructionId>,
    pub terminator: InstructionId,
    pub source_span: Option<SourceSpanId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Opcode {
    ConstI32,
    ConstU32,
    Copy,
    AddI32,
    SubI32,
    MulI32,
    EqI32,
    NeI32,
    LtI32,
    Branch,
    BranchIf,
    Call,
    Ret,
    Trap,
    Alloc,
    LoadI32,
    LoadU32,
    StoreI32,
    StoreU32,
    AddrAdd,
    UnsupportedI64,
    UnsupportedIndirectCall,
}

impl Opcode {
    pub fn is_terminator(self) -> bool {
        matches!(self, Opcode::Branch | Opcode::BranchIf | Opcode::Ret | Opcode::Trap)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operand {
    Value(ValueId),
    ImmI32(i32),
    ImmU32(u32),
    Block(BlockId),
    Function(FunctionId),
    Symbol(SymbolId),
    Type(TypeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instruction {
    pub id: InstructionId,
    pub opcode: Opcode,
    pub results: Vec<ValueId>,
    pub operands: Vec<Operand>,
    pub source_span: Option<SourceSpanId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    pub id: SourceSpanId,
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metadata {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataSegment {
    pub symbol: SymbolId,
    pub offset: u32,
    pub bytes: Vec<u8>,
    pub zero_fill: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleImage {
    pub header: Header,
    pub module: Module,
    pub types: Vec<TypeDef>,
    pub symbols: Vec<Symbol>,
    pub functions: Vec<Function>,
    pub data_segments: Vec<DataSegment>,
    pub blocks: Vec<Block>,
    pub instructions: Vec<Instruction>,
    pub source_spans: Vec<SourceSpan>,
    pub metadata: Vec<Metadata>,
}

impl ModuleImage {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
        loader::from_bytes(bytes)
    }

    pub fn from_text(text: &str) -> Result<Self, LoadError> {
        loader::from_text(text)
    }

    pub fn validate(&self) -> Result<ValidationReport, Vec<crate::ValidationError>> {
        Validate::validate(self)
    }

    pub fn function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.iter().find(|f| f.id == id)
    }

    pub fn block(&self, id: BlockId) -> Option<&Block> {
        self.blocks.iter().find(|b| b.id == id)
    }

    pub fn instruction(&self, id: InstructionId) -> Option<&Instruction> {
        self.instructions.iter().find(|i| i.id == id)
    }

    pub fn symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.id == id)
    }

    pub fn type_def(&self, id: TypeId) -> Option<&TypeDef> {
        self.types.iter().find(|t| t.id == id)
    }

    pub fn type_kind(&self, id: TypeId) -> Option<TypeKind> {
        self.type_def(id).map(|ty| ty.kind)
    }
}
