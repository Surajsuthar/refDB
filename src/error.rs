use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum Error {
    IO(String),
    Abort,
    InvalidData(String),
    InvalidInput(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Abort => write!(f, "Operation abort"),
            Error::IO(msg) => write!(f, "IO error : {msg}"),
            Error::InvalidData(msg) => write!(f, "Invalid Data : {msg}"),
            Error::InvalidInput(msg) => write!(f, "Invalid input : {msg}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl<T> From<Error> for Result<T> {
    fn from(error: Error) -> Self {
        Err(error)
    }
}
