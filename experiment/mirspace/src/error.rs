use std::fmt;

#[derive(Debug)]
pub enum SpaceError {
    Validation(Vec<mircap::ValidationError>),
    MircapLoad(mircap::LoadError),
    Inconsistency(String),
}

impl fmt::Display for SpaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpaceError::Validation(errors) => {
                writeln!(
                    f,
                    "Validation failed during space construction with {} errors:",
                    errors.len()
                )?;
                for err in errors {
                    writeln!(f, "  {:?} at {:?}: {}", err.kind, err.entity, err.message)?;
                }
                Ok(())
            }
            SpaceError::MircapLoad(err) => write!(f, "Mircap Load Error: {:?}", err),
            SpaceError::Inconsistency(msg) => write!(f, "Inconsistency Error: {}", msg),
        }
    }
}

impl std::error::Error for SpaceError {}

impl From<Vec<mircap::ValidationError>> for SpaceError {
    fn from(errors: Vec<mircap::ValidationError>) -> Self {
        SpaceError::Validation(errors)
    }
}

impl From<mircap::LoadError> for SpaceError {
    fn from(err: mircap::LoadError) -> Self {
        SpaceError::MircapLoad(err)
    }
}
