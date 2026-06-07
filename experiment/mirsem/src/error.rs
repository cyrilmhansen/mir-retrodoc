use crate::trap::ExecutionTrap;

#[derive(Debug)]
pub enum ExecutionError {
    Validation(Vec<mircap::ValidationError>),
    UnsupportedMirF0(String),
    Trap(ExecutionTrap),
    Internal(String),
}

pub type RunError = ExecutionError;

impl From<ExecutionTrap> for ExecutionError {
    fn from(value: ExecutionTrap) -> Self {
        ExecutionError::Trap(value)
    }
}

