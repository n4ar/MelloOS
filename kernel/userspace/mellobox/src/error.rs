//! Error handling for mellobox utilities

#![allow(dead_code)]

use core::fmt;

/// Result type for mellobox utilities
pub type Result<T> = core::result::Result<T, Error>;

/// Error types for mellobox utilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// File or directory not found
    NotFound,
    /// Permission denied
    PermissionDenied,
    /// Invalid argument
    InvalidArgument,
    /// I/O error
    IoError,
    /// File already exists
    AlreadyExists,
    /// Not a directory
    NotADirectory,
    /// Is a directory
    IsADirectory,
    /// Directory not empty
    DirectoryNotEmpty,
    /// Too many arguments
    TooManyArguments,
    /// Missing required argument
    MissingArgument,
    /// Unknown option
    UnknownOption,
    /// Invalid option argument
    InvalidOptionArgument,
    /// System call failed
    SyscallFailed(isize),
}

impl Error {
    /// Convert errno to Error
    pub fn from_errno(errno: isize) -> Self {
        match -errno {
            2 => Error::NotFound,           // ENOENT
            13 => Error::PermissionDenied,  // EACCES
            17 => Error::AlreadyExists,     // EEXIST
            20 => Error::NotADirectory,     // ENOTDIR
            21 => Error::IsADirectory,      // EISDIR
            22 => Error::InvalidArgument,   // EINVAL
            39 => Error::DirectoryNotEmpty, // ENOTEMPTY
            _ => Error::SyscallFailed(errno),
        }
    }

    /// Get exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            // Usage errors return 2
            Error::InvalidArgument
            | Error::TooManyArguments
            | Error::MissingArgument
            | Error::UnknownOption
            | Error::InvalidOptionArgument => 2,
            // Runtime errors return 1
            _ => 1,
        }
    }

    /// Get error message
    pub fn message(&self) -> &'static str {
        match self {
            Error::NotFound => "No such file or directory",
            Error::PermissionDenied => "Permission denied",
            Error::InvalidArgument => "Invalid argument",
            Error::IoError => "I/O error",
            Error::AlreadyExists => "File exists",
            Error::NotADirectory => "Not a directory",
            Error::IsADirectory => "Is a directory",
            Error::DirectoryNotEmpty => "Directory not empty",
            Error::TooManyArguments => "Too many arguments",
            Error::MissingArgument => "Missing required argument",
            Error::UnknownOption => "Unknown option",
            Error::InvalidOptionArgument => "Invalid option argument",
            Error::SyscallFailed(_) => "System call failed",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SyscallFailed(errno) => write!(f, "{} (errno: {})", self.message(), errno),
            _ => write!(f, "{}", self.message()),
        }
    }
}

/// Print error message to stderr
pub fn print_error(program: &str, error: Error) {
    use crate::syscalls;
    
    let mut buf = [0u8; 256];
    let mut pos = 0;
    
    // Write program name
    for &b in program.as_bytes() {
        if pos >= buf.len() - 1 {
            break;
        }
        buf[pos] = b;
        pos += 1;
    }
    
    // Write ": "
    if pos < buf.len() - 2 {
        buf[pos] = b':';
        pos += 1;
        buf[pos] = b' ';
        pos += 1;
    }
    
    // Write error message
    for &b in error.message().as_bytes() {
        if pos >= buf.len() - 1 {
            break;
        }
        buf[pos] = b;
        pos += 1;
    }
    
    // Write newline
    if pos < buf.len() {
        buf[pos] = b'\n';
        pos += 1;
    }
    
    // Write to stderr (fd 2)
    syscalls::write(2, &buf[..pos]);
}

/// Print usage error to stderr
pub fn print_usage_error(program: &str, message: &str) {
    use crate::syscalls;
    
    let mut buf = [0u8; 256];
    let mut pos = 0;
    
    // Write program name
    for &b in program.as_bytes() {
        if pos >= buf.len() - 1 {
            break;
        }
        buf[pos] = b;
        pos += 1;
    }
    
    // Write ": "
    if pos < buf.len() - 2 {
        buf[pos] = b':';
        pos += 1;
        buf[pos] = b' ';
        pos += 1;
    }
    
    // Write message
    for &b in message.as_bytes() {
        if pos >= buf.len() - 1 {
            break;
        }
        buf[pos] = b;
        pos += 1;
    }
    
    // Write newline
    if pos < buf.len() {
        buf[pos] = b'\n';
        pos += 1;
    }
    
    // Write to stderr (fd 2)
    syscalls::write(2, &buf[..pos]);
}
