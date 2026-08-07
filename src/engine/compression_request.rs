// =============================================================================
// fqc-rust - Compression Request
// =============================================================================
//! Normalized input types for compression operations.
//!
//! This module provides a clean abstraction layer for compression requests,
//! separating execution mode, input topology, and compression parameters.

use crate::types::{CompressionLevel, IdMode, PeLayout, QualityMode, ReadLengthClass};
use std::path::PathBuf;

/// Execution mode for compression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionExecutionMode {
    /// Archive mode: full-featured compression with reordering support
    Archive,
    /// Streaming mode: single-pass compression without reordering
    Streaming,
    /// Pipeline mode: concurrent block compression with in-flight buffering
    Pipeline,
}

/// Input topology for compression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompressionInputTopology {
    /// Single FASTQ file (unpaired)
    SingleFile { input_path: PathBuf },
    /// Paired FASTQ files (R1 and R2)
    PairedFiles {
        input_path_r1: PathBuf,
        input_path_r2: PathBuf,
        archive_layout: PeLayout,
    },
    /// Interleaved FASTQ file (paired reads interleaved in one file)
    InterleavedFile {
        input_path: PathBuf,
        archive_layout: PeLayout,
    },
    /// Read from stdin
    Stdin { archive_layout: Option<PeLayout> },
}

/// Resolved input properties used by compression execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionInputResolution {
    pub primary_path: String,
    pub secondary_path: Option<String>,
    pub is_paired: bool,
    pub is_interleaved: bool,
    pub archive_layout: PeLayout,
}

impl CompressionInputTopology {
    /// Resolve topology-specific details once so callers do not duplicate branching.
    pub fn resolve(&self) -> CompressionInputResolution {
        match self {
            Self::SingleFile { input_path } => CompressionInputResolution {
                primary_path: input_path.to_string_lossy().to_string(),
                secondary_path: None,
                is_paired: false,
                is_interleaved: false,
                archive_layout: PeLayout::Interleaved,
            },
            Self::PairedFiles {
                input_path_r1,
                input_path_r2,
                archive_layout,
            } => CompressionInputResolution {
                primary_path: input_path_r1.to_string_lossy().to_string(),
                secondary_path: Some(input_path_r2.to_string_lossy().to_string()),
                is_paired: true,
                is_interleaved: false,
                archive_layout: *archive_layout,
            },
            Self::InterleavedFile {
                input_path,
                archive_layout,
            } => CompressionInputResolution {
                primary_path: input_path.to_string_lossy().to_string(),
                secondary_path: None,
                is_paired: true,
                is_interleaved: true,
                archive_layout: *archive_layout,
            },
            Self::Stdin { archive_layout } => {
                let is_paired = archive_layout.is_some();
                CompressionInputResolution {
                    primary_path: "-".to_string(),
                    secondary_path: None,
                    is_paired,
                    is_interleaved: is_paired,
                    archive_layout: archive_layout.unwrap_or(PeLayout::Interleaved),
                }
            }
        }
    }
}

/// Normalized compression request.
///
/// This type represents a validated, normalized compression request
/// with all input parameters resolved and ready for execution.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionRequest {
    /// Execution mode
    pub mode: CompressionExecutionMode,
    /// Input topology
    pub input: CompressionInputTopology,
    /// Output archive path
    pub output_path: PathBuf,
    /// Compression level (1-9)
    pub level: CompressionLevel,
    /// Quality encoding mode
    pub quality_mode: QualityMode,
    /// ID encoding mode
    pub id_mode: IdMode,
    /// Enable global read reordering (short single-end reads)
    pub enable_reorder: bool,
    /// Requested read length class (None = auto-detect)
    pub requested_read_length_class: Option<ReadLengthClass>,
    /// Number of threads (0 = auto)
    pub threads: usize,
    /// Memory limit in MB (0 = auto)
    pub memory_limit_mb: usize,
    /// Force overwrite existing output
    pub force_overwrite: bool,
    /// Show progress during compression
    pub show_progress: bool,
    /// Block size override (0 = auto based on read length class)
    pub block_size: usize,
    /// Max block size in bases (0 = auto)
    pub max_block_bases: usize,
    /// Scan all reads for length detection (slower but more accurate)
    pub scan_all_lengths: bool,
}

impl CompressionRequest {
    /// Create a default request for testing purposes.
    ///
    /// This method provides a minimal valid request with sensible defaults
    /// for use in unit tests and integration tests.
    pub fn for_tests() -> Self {
        Self {
            mode: CompressionExecutionMode::Archive,
            input: CompressionInputTopology::SingleFile {
                input_path: "tests/data/test_se.fastq".into(),
            },
            output_path: "tests/output/test.fqc".into(),
            level: crate::types::DEFAULT_COMPRESSION_LEVEL,
            quality_mode: QualityMode::Lossless,
            id_mode: IdMode::Exact,
            enable_reorder: true,
            requested_read_length_class: None,
            threads: 1,
            memory_limit_mb: 0,
            force_overwrite: false,
            show_progress: false,
            block_size: 0,
            max_block_bases: 0,
            scan_all_lengths: false,
        }
    }
}
