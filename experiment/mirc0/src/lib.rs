//! Baseline correctness-oriented compiler from MIR-F0 to simple, portable C.
//!
//! `mirc0` is used to compare compiled execution against reference interpreter `mirsem`.

pub mod error;
pub mod c_emit;
pub mod compile;
pub mod runtime_c;
pub mod pretty;

pub use error::CompileError;
pub use compile::compile;
