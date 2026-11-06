// =============================================================================
// fqc - Archive compression runner
// =============================================================================

use crate::algo::block_compressor::BlockCompressorConfig;
use crate::algo::global_analyzer::{GlobalAnalyzer, GlobalAnalyzerConfig};
use crate::archive::format::build_flags;
use crate::archive::writer::{begin_fqc_writer, write_global_header};
use crate::engine::compression_engine::CompressionOutcome;
use crate::error::{FqcError, Result};
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

    // Resolve block size and codec workers together before global analysis.
    let block_config = std::sync::Arc::new(BlockCompressorConfig {
        read_length_class: effective_length_class,
        quality_mode: request.settings().quality_mode,
        id_mode: request.settings().id_mode,
        zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(request.settings().level),
        ..Default::default()
    });
    let thread_limit = common::codec_thread_limit(request.settings().threads, request.settings().memory_limit_mb);
    let worker_budget = crate::plan::worker_budget_bytes(request.settings().memory_limit_mb);
    let avg_length = crate::plan::avg_read_length(total_bases, total_reads);
    let plan = plan_compression_layout(CompressionPlanInput {
        requested_block_size: request.settings().block_size,
        avg_length,
        max_block_bases: request.settings().max_block_bases,
        thread_limit,
        worker_budget_bytes: worker_budget,
        total_reads: Some(total_reads),
        total_bases: Some(total_bases),
        read_length_class: effective_length_class,
        codec_model_bytes: block_config.to_quality_config().estimated_model_bytes(),
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
    let sequences: Vec<String> = records.iter().map(|r| r.sequence.clone()).collect();

    let analyzer_config = GlobalAnalyzerConfig {
        reads_per_block: block_size,
        reorder,
        read_length_class: Some(effective_length_class),
        ..Default::default()
    };

    let analyzer = GlobalAnalyzer::new(analyzer_config);
    let analysis = analyzer.analyze(&sequences)?;

    log::info!(
        "Analysis: {} blocks, reordering={}",
        analysis.num_blocks,
        analysis.reordering_performed
    );

    // Phase 2: Write FQC archive (temp file; commit after finalize)
    let (mut writer, output_tx) = begin_fqc_writer(request.output_path(), request.settings().force_overwrite)?;

    // Build flags
    let flags = build_flags(
        input.is_paired,
        !analysis.reordering_performed,
        request.settings().quality_mode,
        request.settings().id_mode,
        analysis.reordering_performed,
        input.archive_layout,
        effective_length_class,
        false, // not streaming
    );

    let input_filename = input
        .primary_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    write_global_header(&mut writer, flags, records.len() as u64, input_filename)?;

    // Extract block read sets
    // Reordered block build consumes reads by move (mem::take) instead of
    // cloning each record: identical order, same bytes, fewer allocations.
    let mut records = records;
    let block_read_sets: Vec<(u32, Vec<ReadRecord>)> = analysis
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
                Some((boundary.block_id, block_reads))
            }
        })
        .collect();
    let reorder_ms = t_reorder.elapsed().as_millis() as u64;

    // Parallel block compression
    let num_blocks = block_read_sets.len();
    log::info!(
        "Compressing {} blocks{}...",
        num_blocks,
        if num_blocks > 1 { " in parallel" } else { "" }
    );

    let max_workers = plan.workers.min(num_blocks.max(1));
    if max_workers < num_blocks {
        log::info!("Compression worker cap: {} ({} blocks)", max_workers, num_blocks);
    }
    let (compressed_blocks, process_ms) = common::compress_blocks(&block_read_sets, max_workers, block_config.as_ref());

    // Sequential write (file I/O must be ordered)
    let t_write = std::time::Instant::now();
    let mut archive_id_start = 0u64;
    let mut blocks_written = 0;

    for (i, result) in compressed_blocks.into_iter().enumerate() {
        let compressed = result?;
        let num_reads = compressed.read_count as u64;

        writer.write_block_with_id(&compressed, archive_id_start)?;
        archive_id_start += num_reads;

        blocks_written += 1;

        log::debug!(
            "Block {} written: {} reads, {} bytes",
            i,
            num_reads,
            compressed.total_compressed_size()
        );
    }

    // Write reorder map if applicable
    let reorder_map_written = analysis.reordering_performed && !analysis.forward_map.is_empty();
    if reorder_map_written {
        writer.write_reorder_map(&analysis.forward_map, &analysis.reverse_map)?;
        log::info!("Reorder map written: {} reads", analysis.forward_map.len());
    }

    // Finalize then atomically replace the destination. `finalize` returns
    // the actual archive size, including headers, index and footer.
    let output_bytes = writer.finalize()?;
    output_tx.commit()?;
    let write_ms = t_write.elapsed().as_millis() as u64;

    log::info!("Compression complete! {} blocks written.", blocks_written);

    let input_bytes = common::input_bytes_for(&input.primary_path, input.secondary_path.as_deref(), total_bases);
    let stats = OperationStats {
        total_reads,
        total_bases,
        input_bytes,
        output_bytes,
        blocks: blocks_written as u64,
        elapsed_seconds: 0.0,
        parse_ms,
        reorder_ms,
        process_ms,
        write_ms,
        reorder_map_written,
        ..Default::default()
    };

    Ok(CompressionOutcome {
        mode: CompressionExecutionMode::Archive,
        detected_read_length_class: effective_length_class,
        stats,
    })
}
