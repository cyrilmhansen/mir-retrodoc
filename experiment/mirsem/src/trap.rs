use mircap::{BlockId, FunctionId, InstructionId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionTrap {
    StackOverflow {
        max_depth: usize,
    },
    FuelExhausted {
        max_instructions: u64,
    },
    ExplicitTrap {
        instruction: InstructionId,
    },
    UnsupportedInstruction {
        instruction: InstructionId,
        opcode: String,
    },
    UnsupportedType {
        function: FunctionId,
    },
    InvalidBlock {
        function: FunctionId,
        block: BlockId,
    },
    InvalidInstruction {
        instruction: InstructionId,
    },
    CallArityMismatch {
        instruction: InstructionId,
    },
    ReturnArityMismatch {
        instruction: InstructionId,
    },
    UninitializedValue {
        function: FunctionId,
        value: u32,
    },
    OutOfMemory {
        requested: u32,
        align: u32,
    },
    HeapStackCollision {
        requested: u32,
        align: u32,
    },
    OutOfBoundsLoad {
        addr: u32,
        size: u32,
    },
    OutOfBoundsStore {
        addr: u32,
        size: u32,
    },
    MisalignedLoad {
        addr: u32,
        align: u32,
    },
    MisalignedStore {
        addr: u32,
        align: u32,
    },
    AddressOverflow {
        base: u32,
        offset: u32,
    },
}
