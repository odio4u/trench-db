use std::fmt;
use std::io;

/// Errors that can occur when reading, writing, or replaying a WAL file.
#[derive(Debug)]
pub enum WalError {
    /// A generic I/O error propagated from the operating system.
    Io(io::Error),
    /// The payload of a WAL entry is structurally invalid.
    InvalidPayload(String),
    /// The WAL file ended before an entry could be fully read.
    UnexpectedEof,
    /// An entry failed its checksum or length validation.
    CorruptEntry { position: u64, reason: String },
    /// The WAL file header is missing or malformed.
    InvalidHeader { reason: String },
    /// The WAL file version is not supported by this implementation.
    UnsupportedVersion { found: u8, expected: u8 },
    /// The WAL path does not have a parent directory.
    PathMissing,
}

impl fmt::Display for WalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalError::Io(err) => write!(f, "I/O error: {}", err),
            WalError::InvalidPayload(msg) => write!(f, "invalid WAL payload: {}", msg),
            WalError::UnexpectedEof => write!(f, "unexpected end of WAL file"),
            WalError::CorruptEntry { position, reason } => write!(f, "corrupt WAL entry at offset {}: {}", position, reason),
            WalError::InvalidHeader { reason } => write!(f, "invalid WAL header: {}", reason),
            WalError::UnsupportedVersion { found, expected } => write!(f, "unsupported WAL version {} (expected {})", found, expected),
            WalError::PathMissing => write!(f, "WAL path has no parent directory"),
        }
    }
}

impl std::error::Error for WalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WalError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for WalError {
    fn from(value: io::Error) -> Self {
        if value.kind() == io::ErrorKind::UnexpectedEof {
            WalError::UnexpectedEof
        } else {
            WalError::Io(value)
        }
    }
}
