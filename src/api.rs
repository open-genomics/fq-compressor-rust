// =============================================================================
// fqc - Stable operation facade
// =============================================================================
//! Small, binary-first facade over the four user operations.
//!
//! The rest of the module tree is public for integration tests and for
//! advanced callers, but it is internal API: names and layout may change
//! between minor releases. Prefer these functions and the re-exported request
//! types when embedding `fqc`.

use std::path::Path;

pub use crate::archive::reader::ArchiveInfo;
use crate::engine::compression_engine::CompressionEngine;
pub use crate::engine::compression_engine::CompressionOutcome;
use crate::error::Result;
use crate::pipeline::decompression::{DecompressionConfig, DecompressionPipeline};
use crate::pipeline::decompression_classic::SequentialDecompressor;
use crate::plan::compress::CompressionRequest;
pub use crate::plan::compress::{
    CompressionInputTopology, CompressionRequest as CompressRequest, CompressionSettings,
    CompressionSettings as CompressSettings,
};
use crate::plan::decompress::ValidatedDecompressPlan;
pub use crate::plan::decompress::{DecompressRequest, DecompressSettings};
pub use crate::plan::{CompressionExecutionMode, ReorderMode};
pub use crate::stats::OperationStats;
use crate::verify::verify_archive;
pub use crate::verify::{VerifyOptions, VerifyReport};

/// Compress one input according to a validated request.
pub fn compress(request: CompressionRequest) -> Result<CompressionOutcome> {
    CompressionEngine::new().run(request)
}

/// Decompress one archive, choosing the threaded or sequential path.
pub fn decompress(request: DecompressRequest) -> Result<OperationStats> {
    let plan = ValidatedDecompressPlan::new(request)?;
    let request = plan.request();
    let settings = request.settings();
    let config = DecompressionConfig {
        num_threads: settings.threads,
        range_start: settings.range_start,
        range_end: settings.range_end,
        original_order: settings.original_order,
        header_only: settings.header_only,
        skip_corrupted: settings.skip_corrupted,
        corrupted_placeholder: settings.corrupted_placeholder.clone(),
        force_overwrite: settings.force_overwrite,
        split_pe: settings.split_pe,
        memory_limit_mb: settings.memory_limit_mb,
        ..Default::default()
    };
    let input = request.input_path().to_string_lossy();
    let output = request.output_path().to_string_lossy();

    if settings.use_pipeline && !settings.original_order && !settings.split_pe {
        let mut pipeline = DecompressionPipeline::new(config);
        pipeline.run(&input, &output)?;
        Ok(pipeline.stats().clone())
    } else {
        let mut sequential = SequentialDecompressor::new(config);
        sequential.run(&input, &output)
    }
}

/// Read archive metadata without decoding blocks.
pub fn info(path: impl AsRef<Path>) -> Result<ArchiveInfo> {
    let path = path.as_ref().to_string_lossy();
    Ok(crate::archive::reader::FqcReader::open(&path)?.info())
}

/// Verify an archive and return counts instead of printing a summary.
pub fn verify(options: &VerifyOptions) -> Result<VerifyReport> {
    verify_archive(options)
}
