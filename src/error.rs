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
