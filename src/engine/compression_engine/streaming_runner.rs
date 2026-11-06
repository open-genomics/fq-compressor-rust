// =============================================================================
// fqc - Streaming compression runner
// =============================================================================
//! Bounded-ingest streaming: the parser feeds a block source on demand, the
//! shared block pipeline compresses blocks in parallel and the ordered writer
//! commits them transactionally. Nothing holds the whole input in memory.

use crate::algo::block_compressor::BlockCompressorConfig;
use crate::archive::format::build_flags;
use crate::engine::compression_engine::CompressionOutcome;
use crate::error::Result;
use crate::fastq::parser::{open_fastq, open_fastq_interleaved, open_fastq_paired, open_fastq_stdin};
use crate::pipeline::block_pipeline::{self, BlockPipelineConfig};
use crate::plan::compress::CompressionRequest;
use crate::plan::CompressionExecutionMode;
use crate::stats::OperationStats;
use crate::types::*;
use std::path::Path;
use std::time::Instant;

use super::streaming_support;

/// Streaming prefetches one chunk. The parser is serial, so a deeper queue
/// only multiplies resident blocks without feeding workers faster.
const STREAMING_MAX_IN_FLIGHT_BLOCKS: usize = 1;

/// Block sizing and codec layout resolved before the source starts pulling.
struct StreamingPlan {
    length_class: ReadLengthClass,
    block_size: usize,
    num_workers: usize,
    max_in_flight_blocks: usize,
    codec_config: BlockCompressorConfig,
}

pub(super) fn run(request: &CompressionRequest) -> Result<CompressionOutcome> {
    log::info!("Streaming compression mode");

    let input = request.input().resolve();

    // Inspect input lengths. This pre-scan is part of streaming's parser cost.
    let t_inspect = Instant::now();
    let length_stats = streaming_support::inspect_input_lengths_for_streaming(
        &input.primary_path,
        input.secondary_path.as_deref(),
        input.is_interleaved,
        request.settings().scan_all_lengths,
    )?
    .unwrap_or(LengthStats {
        sample_size: 0,
        avg_length: MEDIUM_READ_THRESHOLD,
        median_length: MEDIUM_READ_THRESHOLD,
        max_length: MEDIUM_READ_THRESHOLD,
    });
    let inspect_ns = t_inspect.elapsed().as_nanos() as u64;

    let effective_length_class =
        crate::plan::effective_read_length_class(request.settings().requested_read_length_class, &length_stats);
    let codec = streaming_support::codec_profile(request.settings(), effective_length_class);
    let compression_plan = crate::plan::plan_compression_layout(crate::plan::CompressionPlanInput {
        requested_block_size: request.settings().block_size,
        avg_length: length_stats.avg_length,
        max_block_bases: request.settings().max_block_bases,
        thread_limit: codec.thread_limit,
        worker_budget_bytes: codec.worker_budget_bytes,
        total_reads: None,
        total_bases: None,
        read_length_class: effective_length_class,
        codec_model_bytes: codec.model_bytes,
        auto_split_long_reads: false,
    });
    let block_size = compression_plan.block_size;
    let num_workers = compression_plan.workers;
    let max_in_flight_blocks = crate::plan::effective_in_flight_blocks(
        block_size,
        &length_stats,
        request.settings().memory_limit_mb,
        STREAMING_MAX_IN_FLIGHT_BLOCKS,
    );
    let plan = StreamingPlan {
        length_class: effective_length_class,
        block_size,
        num_workers,
        max_in_flight_blocks,
        codec_config: codec.config,
    };

    log::info!(
        "Streaming profile: sample={} avg={}bp median={}bp max={}bp, class={}, block_size={}, workers={}, in_flight={}",
        length_stats.sample_size,
        length_stats.avg_length,
        length_stats.median_length,
        length_stats.max_length,
        effective_length_class.as_str(),
        block_size,
        num_workers,
        max_in_flight_blocks
    );

    // Get archive layout for paired/interleaved
    // Route to appropriate streaming handler
    if let Some(path2) = input.secondary_path {
        run_paired(
            &input.primary_path,
            &path2,
            request,
            &plan,
            input.archive_layout,
            inspect_ns,
        )
    } else if input.is_interleaved {
        run_interleaved(&input.primary_path, request, &plan, input.archive_layout, inspect_ns)
    } else {
        run_single(&input.primary_path, request, &plan, inspect_ns)
    }
}

/// Run the shared block pipeline over a bounded streaming block source.
fn run_core(
    request: &CompressionRequest,
    plan: &StreamingPlan,
    pe_layout: PeLayout,
    single_end: bool,
    initial_parse_ns: u64,
    next_block: impl FnMut() -> Result<Option<Vec<ReadRecord>>> + Send + 'static,
) -> Result<CompressionOutcome> {
    let flags = build_flags(
        !single_end,
        true,
        request.settings().quality_mode,
        request.settings().id_mode,
        false,
        pe_layout,
        plan.length_class,
        true,
    );
    let input = request.input().resolve();
    let input_filename = input
        .primary_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("stdin");

    let report = block_pipeline::run(
        next_block,
        request.output_path(),
        BlockPipelineConfig {
            num_workers: plan.num_workers,
            max_in_flight_blocks: plan.max_in_flight_blocks,
            force_overwrite: request.settings().force_overwrite,
            flags,
            total_reads: None,
            expected_blocks: None,
            original_filename: input_filename.to_string(),
            reorder_map: None,
            compressor_config: plan.codec_config.clone(),
        },
    )?;
    log::info!(
        "Streaming compression complete! {} blocks written.",
        report.blocks_written
    );

    let input_bytes =
        crate::stats::input_bytes_for(&input.primary_path, input.secondary_path.as_deref(), report.total_bases);
    let stats = OperationStats {
        total_reads: report.total_reads,
        total_bases: report.total_bases,
        input_bytes,
        output_bytes: report.output_bytes,
        blocks: report.blocks_written,
        elapsed_seconds: 0.0,
        parse_ms: report.parse_ms + initial_parse_ns / 1_000_000,
        reorder_ms: 0,
        process_ms: report.process_ms,
        write_ms: report.write_ms,
        ..Default::default()
    };

    Ok(CompressionOutcome {
        mode: CompressionExecutionMode::Streaming,
        detected_read_length_class: plan.length_class,
        stats,
    })
}

fn run_single(
    input_path: &Path,
    request: &CompressionRequest,
    plan: &StreamingPlan,
    inspect_ns: u64,
) -> Result<CompressionOutcome> {
    let mut parser = if input_path == Path::new("-") {
        open_fastq_stdin()
    } else {
        open_fastq(input_path)?
    };
    let block_size = plan.block_size;
    run_core(request, plan, PeLayout::Interleaved, true, inspect_ns, move || {
        let mut buf: Vec<ReadRecord> = Vec::with_capacity(block_size);
        loop {
            match parser.next_record()? {
                Some(rec) => {
                    buf.push(rec);
                    if buf.len() >= block_size {
                        break;
                    }
                }
                None => break,
            }
        }
        Ok(if buf.is_empty() { None } else { Some(buf) })
    })
}

fn run_paired(
    input_path: &Path,
    input2_path: &Path,
    request: &CompressionRequest,
    plan: &StreamingPlan,
    pe_layout: PeLayout,
    inspect_ns: u64,
) -> Result<CompressionOutcome> {
    let mut pe_reader = open_fastq_paired(input_path, input2_path)?;
    let target_pairs = (plan.block_size.max(2)) / 2;
    run_core(request, plan, pe_layout, false, inspect_ns, move || {
        let mut r1_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        let mut r2_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        loop {
            match pe_reader.next_pair()? {
                Some((r1, r2)) => {
                    r1_buf.push(r1);
                    r2_buf.push(r2);
                    if r1_buf.len() >= target_pairs {
                        break;
                    }
                }
                None => break,
            }
        }
        if r1_buf.is_empty() && r2_buf.is_empty() {
            Ok(None)
        } else {
            let block_buf = pe_layout.arrange(r1_buf, r2_buf);
            Ok(if block_buf.is_empty() { None } else { Some(block_buf) })
        }
    })
}

fn run_interleaved(
    input_path: &Path,
    request: &CompressionRequest,
    plan: &StreamingPlan,
    pe_layout: PeLayout,
    inspect_ns: u64,
) -> Result<CompressionOutcome> {
    let mut parser = if input_path == Path::new("-") {
        crate::fastq::parser::InterleavedPeParser::new(open_fastq_stdin())
    } else {
        open_fastq_interleaved(input_path)?
    };
    let target_pairs = (plan.block_size.max(2)) / 2;
    run_core(request, plan, pe_layout, false, inspect_ns, move || {
        let mut r1_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        let mut r2_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        loop {
            match parser.next_pair()? {
                Some((r1, r2)) => {
                    r1_buf.push(r1);
                    r2_buf.push(r2);
                    if r1_buf.len() >= target_pairs {
                        break;
                    }
                }
                None => break,
            }
        }
        if r1_buf.is_empty() && r2_buf.is_empty() {
            Ok(None)
        } else {
            let block_buf = pe_layout.arrange(r1_buf, r2_buf);
            Ok(if block_buf.is_empty() { None } else { Some(block_buf) })
        }
    })
}
