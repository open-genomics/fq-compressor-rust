// =============================================================================
// Compressor Traits — Seams for Block Compression
// =============================================================================
//! Traits defining seams for stream compressors.
//!
//! Each stream type (sequence, quality, ID, aux) has a compressor trait.
//! `BlockCompressor` coordinates these traits, enabling:
//! - Codec selection at runtime based on `ReadLengthClass`
//! - Easy addition of new codecs without modifying `BlockCompressor`
//! - Testable compression logic with mock implementations

use crate::error::Result;
use crate::types::ReadRecord;

// =============================================================================
// SequenceCompressor
// =============================================================================

/// Compressor for DNA sequences.
///
/// Implementations:
/// - `AbcCompressor` for short reads (≤511 bp)
/// - `ZstdSequenceCompressor` for medium/long reads
pub trait SequenceCompressor: Send + Sync {
    /// Compress sequences from reads.
    fn compress(&self, reads: &[ReadRecord]) -> Result<Vec<u8>>;

    /// Decompress sequences.
    ///
    /// # Arguments
    /// * `data` — Compressed data
    /// * `read_count` — Number of reads to decompress
    /// * `uniform_length` — If non-zero, all reads have this length
    /// * `lengths` — Individual lengths if non-uniform
    fn decompress(&self, data: &[u8], read_count: u32, uniform_length: u32, lengths: &[u32]) -> Result<Vec<String>>;

    /// Codec identifier for this compressor.
    ///
    /// Used in block headers to identify which codec was used.
    fn codec_id(&self) -> u8;
}

// =============================================================================
// QualityCompressor
// =============================================================================

/// Compressor for quality scores.
///
/// Implementations:
/// - `ScmQualityCompressor` using Statistical Compression Model
/// - `DiscardQualityCompressor` for lossy mode (empty output)
pub trait QualityCompressor: Send + Sync {
    /// Compress quality scores from reads.
    fn compress(&mut self, reads: &[ReadRecord]) -> Result<Vec<u8>>;

    /// Decompress quality scores.
    ///
    /// # Arguments
    /// * `data` — Compressed data
    /// * `read_count` — Number of reads to decompress
    /// * `uniform_length` — If non-zero, all reads have this length
    /// * `lengths` — Individual lengths if non-uniform
    fn decompress(&mut self, data: &[u8], read_count: u32, uniform_length: u32, lengths: &[u32])
        -> Result<Vec<String>>;

    /// Codec identifier for this compressor.
    fn codec_id(&self) -> u8;
}

// =============================================================================
// IdCompressor
// =============================================================================

/// Compressor for read IDs.
///
/// Implementations:
/// - `DeltaZstdIdCompressor` using delta encoding + Zstd
/// - `DiscardIdCompressor` for discard mode (generates IDs from prefix)
pub trait IdCompressor: Send + Sync {
    /// Compress read IDs.
    fn compress(&self, reads: &[ReadRecord]) -> Result<Vec<u8>>;

    /// Decompress read IDs.
    ///
    /// # Arguments
    /// * `data` — Compressed data
    /// * `read_count` — Number of IDs to decompress
    fn decompress(&self, data: &[u8], read_count: u32) -> Result<Vec<String>>;

    /// Codec identifier for this compressor.
    fn codec_id(&self) -> u8;
}

// =============================================================================
// AuxCompressor
// =============================================================================

/// Compressor for auxiliary data (read lengths).
///
/// Implementations:
/// - `DeltaVarintAuxCompressor` using delta encoding + varint + Zstd
pub trait AuxCompressor: Send + Sync {
    /// Compress auxiliary data (lengths).
    ///
    /// Returns compressed data and the uniform length if all reads have same length.
    fn compress(&self, reads: &[ReadRecord]) -> Result<(Vec<u8>, u32)>;

    /// Decompress auxiliary data.
    fn decompress(&self, data: &[u8], read_count: u32) -> Result<Vec<u32>>;

    /// Codec identifier for this compressor.
    fn codec_id(&self) -> u8;
}
