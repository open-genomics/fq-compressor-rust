// =============================================================================
// fqc - Validated compression requests
// =============================================================================
//! Normalized compression request types and the single validation gate.

use crate::error::{FqcError, Result};
use crate::plan::{CompressionExecutionMode, ReorderMode, MAX_BLOCK_SIZE, MIN_BLOCK_SIZE};
use crate::types::{CompressionLevel, IdMode, PeLayout, QualityMode, ReadLengthClass};
use std::path::{Path, PathBuf};

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
    pub primary_path: PathBuf,
    pub secondary_path: Option<PathBuf>,
    pub is_paired: bool,
    pub is_interleaved: bool,
    pub archive_layout: PeLayout,
}

impl CompressionInputTopology {
    /// Resolve topology-specific details once so callers do not duplicate branching.
    pub fn resolve(&self) -> CompressionInputResolution {
        match self {
            Self::SingleFile { input_path } => CompressionInputResolution {
                primary_path: input_path.clone(),
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
                primary_path: input_path_r1.clone(),
                secondary_path: Some(input_path_r2.clone()),
                is_paired: true,
                is_interleaved: false,
                archive_layout: *archive_layout,
            },
            Self::InterleavedFile {
                input_path,
                archive_layout,
            } => CompressionInputResolution {
                primary_path: input_path.clone(),
                secondary_path: None,
                is_paired: true,
                is_interleaved: true,
                archive_layout: *archive_layout,
            },
            Self::Stdin { archive_layout } => {
                let is_paired = archive_layout.is_some();
                CompressionInputResolution {
                    primary_path: PathBuf::from("-"),
                    secondary_path: None,
                    is_paired,
                    is_interleaved: is_paired,
                    archive_layout: archive_layout.unwrap_or(PeLayout::Interleaved),
                }
            }
        }
    }
}

/// Compression parameters shared by every execution mode.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionSettings {
    /// Compression level (1-9).
    pub level: CompressionLevel,
    /// Quality encoding mode.
    pub quality_mode: QualityMode,
    /// ID encoding mode.
    pub id_mode: IdMode,
    /// Read reordering policy.
    pub reorder: ReorderMode,
    /// Requested read length class (None = auto-detect).
    pub requested_read_length_class: Option<ReadLengthClass>,
    /// Number of threads (0 = auto).
    pub threads: usize,
    /// Memory limit in MB (0 = auto).
    pub memory_limit_mb: usize,
    /// Force overwrite existing output.
    pub force_overwrite: bool,
    /// Block size override (0 = auto based on read length class).
    pub block_size: usize,
    /// Max block size in bases (0 = auto).
    pub max_block_bases: usize,
    /// Scan all reads for length detection (slower but more accurate).
    pub scan_all_lengths: bool,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            level: crate::types::DEFAULT_COMPRESSION_LEVEL,
            quality_mode: QualityMode::Lossless,
            id_mode: IdMode::Tokenize,
            reorder: ReorderMode::Auto,
            requested_read_length_class: None,
            threads: 0,
            memory_limit_mb: 0,
            force_overwrite: false,
            block_size: 0,
            max_block_bases: 0,
            scan_all_lengths: false,
        }
    }
}

/// Normalized compression request.
///
/// Fields are private so callers go through [`CompressionRequest::new`] and
/// [`CompressionRequest::with_settings`]; execution validates once through
/// [`ValidatedCompressPlan`].
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionRequest {
    mode: CompressionExecutionMode,
    input: CompressionInputTopology,
    output_path: PathBuf,
    settings: CompressionSettings,
}

impl CompressionRequest {
    pub fn new(
        mode: CompressionExecutionMode,
        input: CompressionInputTopology,
        output_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            mode,
            input,
            output_path: output_path.into(),
            settings: CompressionSettings::default(),
        }
    }

    pub fn with_settings(mut self, settings: CompressionSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn mode(&self) -> CompressionExecutionMode {
        self.mode
    }

    pub fn input(&self) -> &CompressionInputTopology {
        &self.input
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }

    pub fn settings(&self) -> &CompressionSettings {
        &self.settings
    }

    /// Validate the request exactly once before execution.
    ///
    /// This is the single gate used by `CompressionEngine::run`; CLI-only
    /// concerns such as progress printing are intentionally not part of it.
    pub fn validate(&self) -> Result<()> {
        let level = self.settings.level;
        if !(crate::types::MIN_COMPRESSION_LEVEL..=crate::types::MAX_COMPRESSION_LEVEL).contains(&level) {
            return Err(FqcError::InvalidArgument(format!(
                "Compression level must be {}-{}",
                crate::types::MIN_COMPRESSION_LEVEL,
                crate::types::MAX_COMPRESSION_LEVEL
            )));
        }
        if self.settings.block_size > MAX_BLOCK_SIZE {
            return Err(FqcError::InvalidArgument(format!(
                "Block size {} is too large (max {})",
                self.settings.block_size, MAX_BLOCK_SIZE
            )));
        }
        if self.mode == CompressionExecutionMode::Pipeline
            && self.settings.block_size > 0
            && self.settings.block_size < MIN_BLOCK_SIZE
        {
            return Err(FqcError::InvalidArgument(format!(
                "Pipeline block size {} is too small (min {})",
                self.settings.block_size, MIN_BLOCK_SIZE
            )));
        }
        if self.mode == CompressionExecutionMode::Streaming && self.settings.reorder == ReorderMode::Always {
            return Err(FqcError::InvalidArgument(
                "streaming mode does not support forced reordering".to_string(),
            ));
        }
        if self.output_path.as_os_str().is_empty() || self.output_path.as_os_str() == "-" {
            return Err(FqcError::InvalidArgument(
                "compression output must be a file path".to_string(),
            ));
        }
        match &self.input {
            CompressionInputTopology::SingleFile { input_path } => {
                if input_path.as_os_str().is_empty() {
                    return Err(FqcError::InvalidArgument("input path is empty".to_string()));
                }
            }
            CompressionInputTopology::PairedFiles {
                input_path_r1,
                input_path_r2,
                ..
            } => {
                if input_path_r1.as_os_str().is_empty() || input_path_r2.as_os_str().is_empty() {
                    return Err(FqcError::InvalidArgument("paired input path is empty".to_string()));
                }
            }
            CompressionInputTopology::InterleavedFile { input_path, .. } => {
                if input_path.as_os_str().is_empty() {
                    return Err(FqcError::InvalidArgument("input path is empty".to_string()));
                }
            }
            CompressionInputTopology::Stdin { .. } => {}
        }
        Ok(())
    }
}

/// A request that passed [`CompressionRequest::validate`].
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedCompressPlan {
    request: CompressionRequest,
}

impl ValidatedCompressPlan {
    pub fn new(request: CompressionRequest) -> Result<Self> {
        request.validate()?;
        Ok(Self { request })
    }

    pub fn request(&self) -> &CompressionRequest {
        &self.request
    }

    pub fn mode(&self) -> CompressionExecutionMode {
        self.request.mode()
    }
}
