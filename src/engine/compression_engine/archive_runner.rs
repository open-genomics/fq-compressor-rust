// =============================================================================
// fqc - Archive compression runner
// =============================================================================

use crate::algo::global_analyzer::{GlobalAnalyzer, GlobalAnalyzerConfig};
use crate::archive::format::build_flags;
use crate::engine::compression_engine::CompressionOutcome;
use crate::error::{FqcError, Result};
use crate::pipeline::block_pipeline::{self, BlockPipelineConfig};
use crate::plan::compress::CompressionRequest;
use crate::plan::CompressionExecutionMode;
use crate::plan::{plan_compression_layout, CompressionPlanInput};
use crate::stats::OperationStats;
use crate::types::*;

use super::common;

pub(super) fn run(request: &CompressionRequest) -> Result<CompressionOutcome> {
    let input = request.input().resolve();

    let t_parse = std::time::Instant::now();
    log::info!("Reading input file: {}", input.primary_path.display());
    let records = if let Some(path2) = input.secondary_path.as_deref() {
        common::read_all_paired_records(
            &input.primary_path,
            path2,
            input.archive_layout,
            request.settings().memory_limit_mb,
        )?
    } else if input.is_interleaved {
        common::read_all_interleaved_records(
            &input.primary_path,
            input.archive_layout,
            request.settings().memory_limit_mb,
        )?
    } else {
        common::read_all_records(&input.primary_path, request.settings().memory_limit_mb)?
    };

    if records.is_empty() {
        return Err(FqcError::InvalidArgument(
            "Input file contains no FASTQ records".to_string(),
        ));
    }

    let parse_ms = t_parse.elapsed().as_millis() as u64;

    let total_bases: u64 = records.iter().map(|r| r.sequence.len() as u64).sum();
    let total_reads = records.len() as u64;
    log::info!("Loaded {} reads ({} bases)", records.len(), total_bases);

    // Detect read length class
    let length_stats = common::length_stats_from_records_sampled(&records, request.settings().scan_all_lengths);
    let effective_length_class =
        common::effective_length_class(request.settings().requested_read_length_class, &length_stats);

    // Resolve codec settings and worker budgets once before global analysis.
    let codec = common::codec_profile(request.settings(), effective_length_class);
    let avg_length = crate::plan::avg_read_length(total_bases, total_reads);
    let plan = plan_compression_layout(CompressionPlanInput {
        requested_block_size: request.settings().block_size,
        avg_length,
        max_block_bases: request.settings().max_block_bases,
        thread_limit: codec.thread_limit,
        worker_budget_bytes: codec.worker_budget_bytes,
        total_reads: Some(total_reads),
        total_bases: Some(total_bases),
        read_length_class: effective_length_class,
        codec_model_bytes: codec.model_bytes,
    });
    let block_size = plan.block_size;
    let reorder = if input.is_paired || effective_length_class != ReadLengthClass::Short {
        crate::plan::ReorderMode::Never
    } else {
        request.settings().reorder
    };

    log::info!("Read length class: {}", effective_length_class.as_str());
    log::info!(
        "Length detection: sample={} avg={}bp median={}bp max={}bp",
        length_stats.sample_size,
        length_stats.avg_length,
        length_stats.median_length,
        length_stats.max_length
    );
    log::info!("Block size: {}", block_size);
    log::info!("Reordering: {}", reorder.enabled());

    // Phase 1: Global analysis (reordering)
    let t_reorder = std::time::Instant::now();

    let analyzer_config = GlobalAnalyzerConfig {
        reads_per_block: block_size,
        reorder,
        read_length_class: Some(effective_length_class),
        ..Default::default()
    };

    let analyzer = GlobalAnalyzer::new(analyzer_config);
    let analysis = analyzer.analyze(&records)?;

    log::info!(
        "Analysis: {} blocks, reordering={}",
        analysis.num_blocks,
        analysis.reordering_performed
    );

    // Build flags before moving the analysis maps into the block pipeline.
    let reorder_map_written = analysis.reordering_performed && !analysis.forward_map.is_empty();
    let flags = build_flags(
        input.is_paired,
        !analysis.reordering_performed,
        request.settings().quality_mode,
        request.settings().id_mode,
        reorder_map_written,
        input.archive_layout,
        effective_length_class,
        false, // not streaming
    );

    let input_filename = input
        .primary_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Extract block read sets. Reordered block build consumes reads by move
    // (mem::take) instead of cloning each record: identical order, same bytes,
    // fewer allocations.
    let mut records = records;
    let block_chunks: Vec<Vec<ReadRecord>> = analysis
        .block_boundaries
        .iter()
        .filter_map(|boundary| {
            let start = boundary.archive_id_start as usize;
            let end = boundary.archive_id_end as usize;

            let block_reads: Vec<ReadRecord> = if analysis.reordering_performed && !analysis.reverse_map.is_empty() {
                (start..end)
                    .filter_map(|archive_id| {
                        analysis
                            .reverse_map
                            .get(archive_id)
                            .and_then(|&orig_id| records.get_mut(orig_id as usize).map(std::mem::take))
                    })
                    .collect()
            } else {
                (start..end.min(records.len()))
                    .map(|i| std::mem::take(&mut records[i]))
                    .collect()
            };

            if block_reads.is_empty() {
                None
            } else {
                Some(block_reads)
            }
        })
        .collect();
    let reorder_ms = t_reorder.elapsed().as_millis() as u64;

    let num_blocks = block_chunks.len();
    log::info!(
        "Compressing {} blocks{}...",
        num_blocks,
        if num_blocks > 1 { " in parallel" } else { "" }
    );

    let max_workers = plan.workers.min(num_blocks.max(1));
    if max_workers < num_blocks {
        log::info!("Compression worker cap: {} ({} blocks)", max_workers, num_blocks);
    }
    let max_in_flight_blocks =
        common::effective_in_flight_blocks(block_size, &length_stats, request.settings().memory_limit_mb);
    let reorder_map = if reorder_map_written {
        Some((analysis.forward_map, analysis.reverse_map))
    } else {
        None
    };

    let report = block_pipeline::run(
        block_chunks,
        request.output_path(),
        BlockPipelineConfig {
            num_workers: max_workers,
            max_in_flight_blocks,
            force_overwrite: request.settings().force_overwrite,
            flags,
            total_reads,
            original_filename: input_filename.to_string(),
            reorder_map,
            compressor_config: codec.config,
        },
    )?;

    if reorder_map_written {
        log::info!("Reorder map written: {} reads", total_reads);
    }
    log::info!("Compression complete! {} blocks written.", report.blocks_written);

    let input_bytes = common::input_bytes_for(&input.primary_path, input.secondary_path.as_deref(), total_bases);
    let stats = OperationStats {
        total_reads,
        total_bases,
        input_bytes,
        output_bytes: report.output_bytes,
        blocks: report.blocks_written,
        elapsed_seconds: 0.0,
        parse_ms,
        reorder_ms,
        process_ms: report.process_ms,
        write_ms: report.write_ms,
        reorder_map_written,
        ..Default::default()
    };

    Ok(CompressionOutcome {
        mode: CompressionExecutionMode::Archive,
        detected_read_length_class: effective_length_class,
        stats,
    })
}
