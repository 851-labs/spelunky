use std::fmt;

/// A malformed replay or trace, or an I/O failure while reading one.
///
/// `line` is 1-based and counts every physical line of the input, including the header and blank lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub line: Option<u64>,
    pub message: String,
}

impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Error {
            line: None,
            message: message.into(),
        }
    }

    pub(crate) fn at(line: u64, message: impl Into<String>) -> Self {
        Error {
            line: Some(line),
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "line {line}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::new(format!("I/O error: {e}"))
    }
}
