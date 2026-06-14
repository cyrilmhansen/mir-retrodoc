//! Baseline correctness-oriented compiler from MIR-F0 to simple, portable C.
//!
//! `mirc0` is used to compare compiled execution against reference interpreter `mirsem`.

pub mod c_emit;
pub mod compile;
pub mod compile_lowered;
pub mod error;
pub mod pretty;
pub mod runtime_c;

pub use compile::compile;
pub use compile_lowered::{compile_lowered, C11Backend};
pub use error::CompileError;
