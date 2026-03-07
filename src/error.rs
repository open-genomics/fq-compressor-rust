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

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

pub type Result<T> = std::result::Result<T, FqcError>;

// =============================================================================
// Error Context (matches C++ ErrorContext)
// =============================================================================

/// Additional context information for errors.
/// Provides detailed information about where and why an error occurred.
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    /// File path associated with the error (if applicable)
    pub file_path: Option<String>,
    /// Block ID where the error occurred (if applicable)
    pub block_id: Option<u32>,
    /// Read ID where the error occurred (if applicable)
    pub read_id: Option<u64>,
    /// Byte offset in file where error occurred (if applicable)
    pub byte_offset: Option<u64>,
}

impl ErrorContext {
    pub fn new() -> Self { Self::default() }

    pub fn with_file(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into()); self
    }

    pub fn with_block(mut self, id: u32) -> Self {
        self.block_id = Some(id); self
    }

    pub fn with_read(mut self, id: u64) -> Self {
        self.read_id = Some(id); self
    }

    pub fn with_offset(mut self, offset: u64) -> Self {
        self.byte_offset = Some(offset); self
    }
}

impl std::fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if let Some(ref p) = self.file_path { parts.push(format!("file={}", p)); }
        if let Some(b) = self.block_id { parts.push(format!("block={}", b)); }
        if let Some(r) = self.read_id { parts.push(format!("read={}", r)); }
        if let Some(o) = self.byte_offset { parts.push(format!("offset={}", o)); }
        if parts.is_empty() {
            write!(f, "<no context>")
        } else {
            write!(f, "[{}]", parts.join(", "))
        }
    }
}

impl FqcError {
    /// Wrap this error with additional context information.
    /// Returns a Format variant containing the original message plus context.
    pub fn with_context(self, ctx: &ErrorContext) -> Self {
        let msg = format!("{} {}", self, ctx);
        match self {
            FqcError::Io(_) => FqcError::Io(std::io::Error::other(msg)),
            FqcError::Format(_) => FqcError::Format(msg),
            FqcError::Compression(_) => FqcError::Compression(msg),
            FqcError::Decompression(_) => FqcError::Decompression(msg),
            FqcError::InvalidArgument(_) => FqcError::InvalidArgument(msg),
            FqcError::Parse(_) => FqcError::Parse(msg),
            FqcError::OutOfRange(_) => FqcError::OutOfRange(msg),
            FqcError::UnsupportedFormat(_) => FqcError::UnsupportedFormat(msg),
            other => other, // ChecksumMismatch, CorruptedBlock, UnsupportedVersion keep structured fields
        }
    }
}

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
            FqcError::UnsupportedFormat(_) => ExitCode::UnsupportedError,
        }
    }

    /// Get the numeric exit code
    pub fn exit_code_num(&self) -> i32 {
        self.exit_code() as i32
    }
}
