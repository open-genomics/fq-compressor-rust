// =============================================================================
// fqc - Pipeline compression runner
// =============================================================================

use crate::engine::compression_engine::CompressionOutcome;
use crate::error::Result;
use crate::pipeline::compression::{CompressionPipeline, CompressionPipelineConfig};
use crate::plan::compress::CompressionRequest;
use crate::plan::CompressionExecutionMode;
use crate::plan::ReorderMode;
use crate::types::*;

use super::common;

pub(super) fn run(request: &CompressionRequest) -> Result<CompressionOutcome> {
    log::info!("Pipeline compression mode");

    let input = request.input().resolve();

    // Inspect input lengths
    let length_stats = common::inspect_input_lengths_for_streaming(
        &input.primary_path,
        input.secondary_path.as_deref(),
        input.is_interleaved,
        request.settings().scan_all_lengths,
    )?
    .unwrap_or(LengthStats {
        sample_size: 0,
        avg_length: 150,
        median_length: 150,
        max_length: 150,
    });

    let effective_length_class =
        common::effective_length_class(request.settings().requested_read_length_class, &length_stats);
    let effective_threads = common::codec_thread_limit(request.settings().threads, request.settings().memory_limit_mb);
    // This is only a sizing hint for the bounded channel; the pipeline
    // recomputes the real block/worker plan once it knows exact totals.
    let block_size_hint = common::effective_block_size(
        request.settings().block_size,
        effective_length_class,
        &length_stats,
        request.settings().max_block_bases,
        effective_threads,
        None,
        None,
        true,
    );
    let max_in_flight_blocks =
        common::effective_in_flight_blocks(block_size_hint, &length_stats, request.settings().memory_limit_mb);

    log::info!("pipeline mode applies the archive ingest budget; use --streaming for bounded memory");

    log::info!(
        "Pipeline profile: sample={} avg={}bp median={}bp max={}bp, class={}, block_size_hint={}, in_flight_blocks={}",
        length_stats.sample_size,
        length_stats.avg_length,
        length_stats.median_length,
        length_stats.max_length,
        effective_length_class.as_str(),
        block_size_hint,
        max_in_flight_blocks
    );

    // Get archive layout for paired/interleaved
    // Create pipeline config. Keep the explicit user block override only:
    // an auto value is resolved by the pipeline after exact totals arrive.
    let pipeline_config = CompressionPipelineConfig {
        num_threads: effective_threads,
        max_in_flight_blocks,
        block_size: request.settings().block_size,
        read_length_class: effective_length_class,
        quality_mode: request.settings().quality_mode,
        id_mode: request.settings().id_mode,
        compression_level: request.settings().level,
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

    let stats = pipeline.stats().clone();

    log::info!(
        "Pipeline compression complete! {} blocks, {:.2}x ratio",
        stats.blocks,
        stats.compression_ratio()
    );

    Ok(CompressionOutcome {
        mode: CompressionExecutionMode::Pipeline,
        detected_read_length_class: effective_length_class,
        stats,
    })
}
