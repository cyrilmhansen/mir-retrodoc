use core::fmt;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum CompileError {
    Validation(Vec<mircap::ValidationError>),
    UnsupportedOpcode(mircap::Opcode),
    UnsupportedType(mircap::TypeKind),
    MultipleResultsNotSupported,
    EntryFunctionNotFound(String),
    InvalidEntrySignature(String),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Validation(errs) => {
                write!(f, "Validation errors ({}):", errs.len())?;
                for err in errs {
                    write!(f, "\n  - {:?}", err)?;
                }
                Ok(())
            }
            CompileError::UnsupportedOpcode(op) => {
                write!(f, "Unsupported opcode: {:?}", op)
            }
            CompileError::UnsupportedType(ty) => {
                write!(f, "Unsupported type: {:?}", ty)
            }
            CompileError::MultipleResultsNotSupported => {
                write!(f, "Multiple results are not supported in mirc0 v0")
            }
            CompileError::EntryFunctionNotFound(name) => {
                write!(f, "Entry function not found: {}", name)
            }
            CompileError::InvalidEntrySignature(reason) => {
                write!(f, "Invalid entry function signature: {}", reason)
            }
        }
    }
}

impl Error for CompileError {}
