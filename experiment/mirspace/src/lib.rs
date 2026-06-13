pub mod analysis;
pub mod cfg;
pub mod debug;
pub mod error;
pub mod ids;
pub mod import;
pub mod space;
pub mod views;

pub use error::SpaceError;
pub use ids::{
    BlockIx, DataSegmentIx, EdgeIx, FunctionIx, InstructionIx, OperandIx, SymbolIx, ValueIx,
};
pub use analysis::{DefUseIndex, ValueDefUse};
pub use space::{
    BlockRec, DataSegmentRec, EdgeKind, EdgeRec, FunctionRec, IdMaps, InstructionRec, OperandRec,
    ProgramSpace, SymbolRec, ValueRec, ValueRole,
};
