use std::fmt;

#[derive(Debug)]
pub enum Error {
    ScriptError(String),
    AssertError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ScriptError(e) => write!(f, "Script error: {}", e),
            Error::AssertError(e) => write!(f, "Assert error: {}", e),
        }
    }
}

impl std::error::Error for Error {}
