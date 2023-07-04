use std::{fmt::Display, num::ParseIntError};




#[derive(Debug)]
pub enum Error { //poor error handling :/ - might fix
    IoError(std::io::Error),
    ParseHeaderError(String),
    UnknownContentType(String),
    SerdeJsonError(serde_json::Error)
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "{}", err),
            Self::ParseHeaderError(str) => write!(f, "{}", str),
            Self::UnknownContentType(str) => write!(f, "{}", str),
            Self::SerdeJsonError(err) => write!(f, "{}", err)
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        //only use of ParseIntError is when parsing asar header
        Error::ParseHeaderError(format!("ParseIntError: {}", err))
    }
}

impl From<serde_json::Error> for Error{
    fn from(err: serde_json::Error) -> Self {
        Error::SerdeJsonError(err)
    }
}

impl std::error::Error for Error{/* todo */}