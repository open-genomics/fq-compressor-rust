// =============================================================================
// fqc-rust - Error Types
// =============================================================================

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FqcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Format error: {0}")]
    Format(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch { expected: u64, actual: u64 },

    #[error("Corrupted block {block_id}: {reason}")]
    CorruptedBlock { block_id: u32, reason: String },

    #[error("Unsupported format version: major={major}")]
    UnsupportedVersion { major: u8 },

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Out of range: {0}")]
    OutOfRange(String),
}

pub type Result<T> = std::result::Result<T, FqcError>;

// =============================================================================
// Exit Code Mapping (matches C++ ErrorCode → CLI exit codes)
// =============================================================================

/// Standard CLI exit codes for fqc
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    /// Successful execution
    Success = 0,
    /// Usage error (invalid arguments, missing files)
    Usage = 1,
    /// I/O error (file not found, permission denied, disk full)
    IoError = 2,
    /// Format error (invalid magic, bad header, corrupted data)
    FormatError = 3,
    /// Checksum or integrity error
    ChecksumError = 4,
    /// Unsupported codec or version
    UnsupportedError = 5,
}

impl FqcError {
    /// Map error to a standard CLI exit code
    pub fn exit_code(&self) -> ExitCode {
        match self {
            FqcError::Io(_) => ExitCode::IoError,
            FqcError::Format(_) => ExitCode::FormatError,
            FqcError::Compression(_) => ExitCode::IoError,
            FqcError::Decompression(_) => ExitCode::FormatError,
            FqcError::InvalidArgument(_) => ExitCode::Usage,
            FqcError::ChecksumMismatch { .. } => ExitCode::ChecksumError,
            FqcError::CorruptedBlock { .. } => ExitCode::ChecksumError,
            FqcError::UnsupportedVersion { .. } => ExitCode::UnsupportedError,
            FqcError::Parse(_) => ExitCode::FormatError,
            FqcError::OutOfRange(_) => ExitCode::Usage,
        }
    }

    /// Get the numeric exit code
    pub fn exit_code_num(&self) -> i32 {
        self.exit_code() as i32
    }
}
