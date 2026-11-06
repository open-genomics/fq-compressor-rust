// =============================================================================
// fqc - Streaming compression runner
// =============================================================================

use crate::algo::block_compressor::BlockCompressor;
use crate::archive::format::build_flags;
use crate::archive::writer::{begin_fqc_writer, write_global_header};
use crate::engine::compression_engine::CompressionOutcome;
use crate::error::Result;
use crate::fastq::parser::{open_fastq, open_fastq_interleaved, open_fastq_paired, open_fastq_stdin};
use crate::plan::compress::CompressionRequest;
use crate::plan::CompressionExecutionMode;
use crate::stats::OperationStats;
use crate::types::*;
use std::path::Path;
use std::time::Instant;

use super::common;

pub(super) fn run(request: &CompressionRequest) -> Result<CompressionOutcome> {
    log::info!("Streaming compression mode");

    let input = request.input().resolve();

    // Inspect input lengths. This pre-scan is part of streaming's parser cost.
    let t_inspect = Instant::now();
    let length_stats = common::inspect_input_lengths_for_streaming(
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
    let block_size = common::effective_block_size(
        request.settings().block_size,
        effective_length_class,
        &length_stats,
        request.settings().max_block_bases,
        request.settings().threads,
        None,
        None,
        false,
    );

    log::info!(
        "Streaming profile: sample={} avg={}bp median={}bp max={}bp, class={}, block_size={}",
        length_stats.sample_size,
        length_stats.avg_length,
        length_stats.median_length,
        length_stats.max_length,
        effective_length_class.as_str(),
        block_size
    );

    // Get archive layout for paired/interleaved
    // Route to appropriate streaming handler
    if let Some(path2) = input.secondary_path {
        run_paired(
            &input.primary_path,
            &path2,
            request,
            effective_length_class,
            block_size,
            input.archive_layout,
            inspect_ns,
        )
    } else if input.is_interleaved {
        run_interleaved(
            &input.primary_path,
            request,
            effective_length_class,
            block_size,
            input.archive_layout,
            inspect_ns,
        )
    } else {
        run_single(
            &input.primary_path,
            request,
            effective_length_class,
            block_size,
            inspect_ns,
        )
    }
}

/// Execute streaming compression: pull one block at a time through a single
/// compressor and the ordered writer, so memory is bounded by the block size.
fn run_core(
    request: &CompressionRequest,
    effective_length_class: ReadLengthClass,
    pe_layout: PeLayout,
    single_end: bool,
    initial_parse_ns: u64,
    mut next_block: impl FnMut() -> Result<Option<Vec<ReadRecord>>>,
) -> Result<CompressionOutcome> {
    let t_setup = Instant::now();
    let (mut writer, output_tx) = begin_fqc_writer(request.output_path(), request.settings().force_overwrite)?;

    let flags = build_flags(
        !single_end,
        true,
        request.settings().quality_mode,
        request.settings().id_mode,
        false,
        pe_layout,
        effective_length_class,
        true,
    );
    let input = request.input().resolve();
    let input_filename = input
        .primary_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("stdin");
    write_global_header(&mut writer, flags, 0, input_filename)?;
    let mut write_ns = t_setup.elapsed().as_nanos() as u64;

    let block_config = common::codec_profile(request.settings(), effective_length_class).config;
    let mut compressor = BlockCompressor::new(block_config);

    let mut block_id = 0u32;
    let mut archive_id_start = 0u64;
    let mut total_reads = 0u64;
    let mut total_bases = 0u64;
    let mut blocks_written = 0;
    let mut parse_ns = initial_parse_ns;
    let mut process_ns = 0u64;

    loop {
        let t_parse = Instant::now();
        let next = next_block()?;
        parse_ns += t_parse.elapsed().as_nanos() as u64;
        let Some(block_reads) = next else {
            break;
        };

        total_reads += block_reads.len() as u64;
        total_bases += block_reads.iter().map(|r| r.sequence.len() as u64).sum::<u64>();

        let t_process = Instant::now();
        let compressed = compressor.compress(&block_reads, block_id)?;
        process_ns += t_process.elapsed().as_nanos() as u64;

        let t_write = Instant::now();
        writer.write_block_with_id(&compressed, archive_id_start)?;
        write_ns += t_write.elapsed().as_nanos() as u64;
        archive_id_start += block_reads.len() as u64;
        blocks_written += 1;
        block_id += 1;
    }

    let t_finish = Instant::now();
    writer.patch_total_read_count(total_reads)?;
    let output_bytes = writer.finalize()?;
    output_tx.commit()?;
    write_ns += t_finish.elapsed().as_nanos() as u64;
    log::info!("Streaming compression complete! {} blocks written.", blocks_written);

    let input_bytes = common::input_bytes_for(&input.primary_path, input.secondary_path.as_deref(), total_bases);
    let stats = OperationStats {
        total_reads,
        total_bases,
        input_bytes,
        output_bytes,
        blocks: blocks_written as u64,
        elapsed_seconds: 0.0,
        parse_ms: parse_ns / 1_000_000,
        reorder_ms: 0,
        process_ms: process_ns / 1_000_000,
        write_ms: write_ns / 1_000_000,
        ..Default::default()
    };

    Ok(CompressionOutcome {
        mode: CompressionExecutionMode::Streaming,
        detected_read_length_class: effective_length_class,
        stats,
    })
}

fn run_single(
    input_path: &Path,
    request: &CompressionRequest,
    effective_length_class: ReadLengthClass,
    block_size: usize,
    inspect_ns: u64,
) -> Result<CompressionOutcome> {
    let mut parser = if input_path == Path::new("-") {
        open_fastq_stdin()
    } else {
        open_fastq(input_path)?
    };
    run_core(
        request,
        effective_length_class,
        PeLayout::Interleaved,
        true,
        inspect_ns,
        move || {
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
        },
    )
}

fn run_paired(
    input_path: &Path,
    input2_path: &Path,
    request: &CompressionRequest,
    effective_length_class: ReadLengthClass,
    block_size: usize,
    pe_layout: PeLayout,
    inspect_ns: u64,
) -> Result<CompressionOutcome> {
    let mut pe_reader = open_fastq_paired(input_path, input2_path)?;
    let target_pairs = (block_size.max(2)) / 2;
    run_core(
        request,
        effective_length_class,
        pe_layout,
        false,
        inspect_ns,
        move || {
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
        },
    )
}

fn run_interleaved(
    input_path: &Path,
    request: &CompressionRequest,
    effective_length_class: ReadLengthClass,
    block_size: usize,
    pe_layout: PeLayout,
    inspect_ns: u64,
) -> Result<CompressionOutcome> {
    let mut parser = if input_path == Path::new("-") {
        crate::fastq::parser::InterleavedPeParser::new(open_fastq_stdin())
    } else {
        open_fastq_interleaved(input_path)?
    };
    let target_pairs = (block_size.max(2)) / 2;
    run_core(
        request,
        effective_length_class,
        pe_layout,
        false,
        inspect_ns,
        move || {
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
        },
    )
}
