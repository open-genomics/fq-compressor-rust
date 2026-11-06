// =============================================================================
// fqc - Pipeline compression runner
// =============================================================================

use crate::algo::block_compressor::BlockCodecSettings;
use crate::engine::compression_engine::CompressionOutcome;
use crate::error::{FqcError, Result};
use crate::pipeline::compression::{CompressionPipeline, CompressionPipelineConfig};
use crate::pipeline::DEFAULT_MAX_IN_FLIGHT_BLOCKS;
use crate::plan::compress::CompressionRequest;
use crate::plan::CompressionExecutionMode;
use crate::plan::ReorderMode;

pub(super) fn run(request: &CompressionRequest) -> Result<CompressionOutcome> {
    log::info!("Pipeline compression mode");
    log::info!("pipeline mode applies the archive ingest budget; use --streaming for bounded memory");

    let input = request.input().resolve();

    // The pipeline classifies the complete ingest itself, so no prefix
    // pre-scan is needed here. `codec` deliberately omits the read-length
    // class; it is resolved once exact records are available.
    let pipeline_config = CompressionPipelineConfig {
        num_threads: request.settings().threads,
        max_in_flight_blocks: DEFAULT_MAX_IN_FLIGHT_BLOCKS,
        block_size: request.settings().block_size,
        codec: BlockCodecSettings::from_compression_level(
            request.settings().quality_mode,
            request.settings().id_mode,
            request.settings().level,
        ),
        requested_read_length_class: request.settings().requested_read_length_class,
        scan_all_lengths: request.settings().scan_all_lengths,
        reorder: if input.is_paired {
            ReorderMode::Never
        } else {
            request.settings().reorder
        },
        save_reorder_map: request.settings().reorder.enabled() && !input.is_paired,
        pe_layout: input.archive_layout,
        max_block_bases: request.settings().max_block_bases,
        force_overwrite: request.settings().force_overwrite,
        memory_limit_mb: request.settings().memory_limit_mb,
    };

    let mut pipeline = CompressionPipeline::new(pipeline_config);

    let input_filename = input
        .primary_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    if let Some(ref path2) = input.secondary_path {
        pipeline.run_paired(
            &input.primary_path,
            path2,
            request.output_path(),
            input_filename,
            input.archive_layout,
        )?;
    } else if input.is_interleaved {
        pipeline.run_interleaved(
            &input.primary_path,
            request.output_path(),
            input_filename,
            input.archive_layout,
        )?;
    } else {
        pipeline.run(&input.primary_path, request.output_path(), input_filename)?;
    }

    let detected_read_length_class = pipeline
        .detected_read_length_class()
        .ok_or_else(|| FqcError::Compression("pipeline finished without classifying the input".to_string()))?;
    let stats = pipeline.stats().clone();

    log::info!(
        "Pipeline compression complete! {} blocks, {:.2}x ratio",
        stats.blocks,
        stats.compression_ratio()
    );

    Ok(CompressionOutcome {
        mode: CompressionExecutionMode::Pipeline,
        detected_read_length_class,
        stats,
    })
}
