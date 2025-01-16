use std::{error, fmt, io, num, path::PathBuf};

#[derive(Debug)]
pub enum ShellError {
    InvalidExitCode(num::ParseIntError),
    NoArguments,
    TooFewArguments { required: u8, received: usize },
    CommandNotFound(String),
    FileOrDirNotFound(PathBuf),
    EnvVarNotFound(String),
    RedirectionError(String),
    IoError(io::Error),
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExitCode(error) => {
                write!(f, "invalid exit code: {error}")
            }
            Self::NoArguments => {
                write!(f, "arguments are required")
            }
            Self::TooFewArguments { required, received } => {
                write!(
                    f,
                    "too few arguments: required: {required}, received: {received}"
                )
            }
            Self::CommandNotFound(cmd) => {
                write!(f, "{cmd}: not found")
            }
            Self::FileOrDirNotFound(path) => {
                write!(f, "cd: {}: No such file or directory", path.display())
            }
            Self::EnvVarNotFound(var) => {
                write!(f, "${} not found", var.to_uppercase())
            }
            Self::RedirectionError(error) => {
                write!(f, "{}", error)
            }
            Self::IoError(error) => {
                write!(f, "{error}")
            }
        }
    }
}

impl From<io::Error> for ShellError {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}

impl error::Error for ShellError {}
