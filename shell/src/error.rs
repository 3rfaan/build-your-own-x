use std::{
    error, fmt, io, num,
    path::{self, PathBuf},
};

#[derive(Debug)]
pub enum ShellError {
    CommandNotFound(String),
    EnvVarNotFound(String),
    FileOrDirNotFound(PathBuf),
    HomeDirPathError(path::StripPrefixError),
    InvalidExitCode(num::ParseIntError),
    IoError(io::Error),
    NoArguments,
    RedirectionError(io::Error),
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandNotFound(cmd) => {
                write!(f, "{cmd}: not found")
            }
            Self::EnvVarNotFound(var) => {
                write!(f, "${} not found", var.to_uppercase())
            }
            Self::FileOrDirNotFound(path) => {
                write!(f, "cd: {}: No such file or directory", path.display())
            }
            Self::HomeDirPathError(error) => {
                write!(f, "could not strip '~' prefix from path: {error}")
            }
            Self::InvalidExitCode(error) => {
                write!(f, "invalid exit code: {error}")
            }
            Self::IoError(error) => {
                write!(f, "{error}")
            }
            Self::NoArguments => {
                write!(f, "arguments are required")
            }
            Self::RedirectionError(error) => {
                write!(f, "{}", error)
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
