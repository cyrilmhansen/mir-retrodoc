use std::fmt;

#[derive(Debug)]
pub enum CliError {
    Io(std::io::Error),
    Load(mircap::LoadError),
    Validation(Vec<mircap::ValidationError>),
    Capnp(capnp::Error),
    Run(mirsem::RunError),
    Compile(mirc0::CompileError),
    Generic(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Io(err) => write!(f, "IO Error: {}", err),
            CliError::Load(err) => match err {
                mircap::LoadError::InvalidUtf8 => write!(f, "Invalid UTF-8 input"),
                mircap::LoadError::InvalidLine { line, message } => {
                    write!(f, "Load Error at line {}: {}", line, message)
                }
            },
            CliError::Validation(errors) => {
                writeln!(f, "Validation failed with {} errors:", errors.len())?;
                for err in errors {
                    writeln!(f, "  {:?} at {:?}: {}", err.kind, err.entity, err.message)?;
                }
                Ok(())
            }
            CliError::Capnp(err) => write!(f, "Cap'n Proto Error: {}", err),
            CliError::Run(err) => write!(f, "Execution Error: {:?}", err),
            CliError::Compile(err) => write!(f, "Compilation Error: {:?}", err),
            CliError::Generic(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::Io(err)
    }
}

impl From<mircap::LoadError> for CliError {
    fn from(err: mircap::LoadError) -> Self {
        CliError::Load(err)
    }
}

impl From<Vec<mircap::ValidationError>> for CliError {
    fn from(errors: Vec<mircap::ValidationError>) -> Self {
        CliError::Validation(errors)
    }
}

impl From<capnp::Error> for CliError {
    fn from(err: capnp::Error) -> Self {
        CliError::Capnp(err)
    }
}

impl From<mirsem::RunError> for CliError {
    fn from(err: mirsem::RunError) -> Self {
        CliError::Run(err)
    }
}

impl From<mirc0::CompileError> for CliError {
    fn from(err: mirc0::CompileError) -> Self {
        CliError::Compile(err)
    }
}

impl From<String> for CliError {
    fn from(msg: String) -> Self {
        CliError::Generic(msg)
    }
}

impl From<&str> for CliError {
    fn from(msg: &str) -> Self {
        CliError::Generic(msg.to_string())
    }
}
