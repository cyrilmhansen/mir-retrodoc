//! Strict MIR-F0 reference evaluator.
//!
//! `mirsem` consumes `mircap::ModuleImage` and executes only the currently
//! validated MIR-F0 subset. It is an oracle for future compiler work, not a
//! production interpreter.

pub mod error;
pub mod frame;
pub mod memory;
pub mod profile;
pub mod runner;
pub mod trace;
pub mod trap;
pub mod value;

pub use error::{ExecutionError, RunError};
pub use memory::LinearMemory;
pub use profile::ExecutionProfile;
pub use runner::{ExecutionResult, Runner};
pub use trace::{CallEdgeTrace, FunctionTrace, TraceSnapshot};
pub use trap::ExecutionTrap;
pub use value::Value;
