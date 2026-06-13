//! Experimental MIR-F0 immutable module image.
//!
//! `mircap` is not upstream MIR. It is a small MIR-inspired module-image
//! prototype with stable IDs, explicit blocks, and conservative validation.

pub mod capnp_serde;
pub mod error;
pub mod ids;
pub mod image;
pub mod loader;
pub mod pretty;
pub mod validate;

pub mod mircap_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema/mircap_capnp.rs"));
}

pub use error::{ErrorKind, ValidationError};
pub use ids::{BlockId, FunctionId, InstructionId, SourceSpanId, SymbolId, TypeId, ValueId};
pub use image::{
    Block, DataSegment, Function, Header, Instruction, Module, ModuleImage, Opcode, Operand,
    Symbol, SymbolKind, TypeDef, TypeKind,
};
pub use loader::LoadError;
pub use validate::{Validate, ValidationReport};
