// =============================================================================
// fqc - Validated decompression requests
// =============================================================================
//! Normalized decompression request types and the single validation gate.

use crate::error::{FqcError, Result};
use std::path::{Path, PathBuf};

/// Decompression parameters shared by the threaded and sequential paths.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecompressSettings {
    pub range_start: u64,
    pub range_end: u64,
    pub header_only: bool,
    pub original_order: bool,
    pub skip_corrupted: bool,
    pub corrupted_placeholder: Option<String>,
    pub split_pe: bool,
    pub threads: usize,
    pub force_overwrite: bool,
    /// Prefer the threaded pipeline for plain decompression.
    pub use_pipeline: bool,
    /// Memory limit in MB (`0` = automatic, still finite).
    pub memory_limit_mb: usize,
}

/// Normalized decompression request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecompressRequest {
    input_path: PathBuf,
    output_path: PathBuf,
    settings: DecompressSettings,
}

impl DecompressRequest {
    pub fn new(input_path: impl Into<PathBuf>, output_path: impl Into<PathBuf>) -> Self {
        Self {
            input_path: input_path.into(),
            output_path: output_path.into(),
            settings: DecompressSettings::default(),
        }
    }

    pub fn with_settings(mut self, settings: DecompressSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn input_path(&self) -> &Path {
        &self.input_path
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }

    pub fn settings(&self) -> &DecompressSettings {
        &self.settings
    }

    /// Validate structural options. Input existence remains an app-level check.
    pub fn validate(&self) -> Result<()> {
        if self.input_path.as_os_str().is_empty() {
            return Err(FqcError::InvalidArgument("input path is empty".to_string()));
        }
        if self.output_path.as_os_str().is_empty() {
            return Err(FqcError::InvalidArgument("output path is empty".to_string()));
        }
        if self.settings.split_pe && self.output_path.as_os_str() == "-" {
            return Err(FqcError::InvalidArgument(
                "--split-pe cannot be used with stdout output".to_string(),
            ));
        }
        if self.settings.range_start > 0
            && self.settings.range_end > 0
            && self.settings.range_start > self.settings.range_end
        {
            return Err(FqcError::InvalidArgument(format!(
                "Invalid read range: start {} is greater than end {}",
                self.settings.range_start, self.settings.range_end
            )));
        }
        Ok(())
    }
}

/// A request that passed [`DecompressRequest::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDecompressPlan {
    request: DecompressRequest,
}

impl ValidatedDecompressPlan {
    pub fn new(request: DecompressRequest) -> Result<Self> {
        request.validate()?;
        Ok(Self { request })
    }

    pub fn request(&self) -> &DecompressRequest {
        &self.request
    }
}
