use std::fmt;
use std::io;

/// The operation that failed while managing a file lock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ErrorOperation {
    /// Opening or creating the lock file.
    Open,
    /// Acquiring the file lock.
    Lock,
    /// Releasing the file lock.
    Unlock,
    /// Closing the lock file after use.
    Close,
}

impl fmt::Display for ErrorOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Open => "open lock file",
            Self::Lock => "acquire file lock",
            Self::Unlock => "release file lock",
            Self::Close => "close lock file",
        })
    }
}

/// An error produced while opening, acquiring, or releasing a file lock.
#[derive(Debug)]
pub struct Error {
    operation: ErrorOperation,
    source: io::Error,
}

impl Error {
    pub(crate) fn new(operation: ErrorOperation, source: io::Error) -> Self {
        Self { operation, source }
    }

    /// Returns the operation that failed.
    pub fn operation(&self) -> ErrorOperation {
        self.operation
    }

    /// Returns the portable category of the underlying operating-system error.
    pub fn kind(&self) -> io::ErrorKind {
        self.source.kind()
    }

    /// Returns the operating-system error code, when one is available.
    pub fn raw_os_error(&self) -> Option<i32> {
        self.source.raw_os_error()
    }

    /// Returns the underlying I/O error.
    pub fn io_error(&self) -> &io::Error {
        &self.source
    }

    /// Consumes this error and returns the underlying I/O error.
    pub fn into_io_error(self) -> io::Error {
        self.source
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to {}: {}", self.operation, self.source)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// A result returned by file-locking operations.
pub type Result<T> = std::result::Result<T, Error>;
