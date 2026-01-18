//! Error types for getattrlistbulk.

use std::fmt;
use std::io;

/// Error type for directory operations.
#[derive(Debug)]
pub enum Error {
    /// Failed to open directory.
    Open(io::Error),
    /// System call failed.
    Syscall(io::Error),
    /// Buffer parsing error.
    Parse(String),
    /// Platform not supported (not macOS).
    NotSupported,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Open(e) => write!(f, "failed to open directory: {}", e),
            Error::Syscall(e) => write!(f, "getattrlistbulk failed: {}", e),
            Error::Parse(msg) => write!(f, "buffer parse error: {}", msg),
            Error::NotSupported => write!(f, "getattrlistbulk is only supported on macOS"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Open(e) | Error::Syscall(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Syscall(e)
    }
}

/// Internal parse error type.
#[derive(Debug)]
pub(crate) enum ParseError {
    /// Buffer is too small for expected data.
    BufferTooSmall,
    /// Invalid offset in attrreference.
    InvalidOffset,
    /// Unexpected end of buffer.
    UnexpectedEnd,
    /// Entry length is zero or invalid.
    InvalidEntryLength,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::BufferTooSmall => write!(f, "buffer too small"),
            ParseError::InvalidOffset => write!(f, "invalid offset in attribute reference"),
            ParseError::UnexpectedEnd => write!(f, "unexpected end of buffer"),
            ParseError::InvalidEntryLength => write!(f, "invalid entry length"),
        }
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(e.to_string())
    }
}
