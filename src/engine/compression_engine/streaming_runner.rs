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

use super::common;

pub(super) fn run(request: &CompressionRequest) -> Result<CompressionOutcome> {
    log::info!("Streaming compression mode");

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
        avg_length: MEDIUM_READ_THRESHOLD,
        median_length: MEDIUM_READ_THRESHOLD,
        max_length: MEDIUM_READ_THRESHOLD,
    });

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
        )
    } else if input.is_interleaved {
        run_interleaved(
            &input.primary_path,
            request,
            effective_length_class,
            block_size,
            input.archive_layout,
        )
    } else {
        run_single(&input.primary_path, request, effective_length_class, block_size)
    }
}

/// Execute pipeline mode compression.
///
/// This method implements pipeline compression:
/// 1. 3-stage pipeline: Reader → Compressor → Writer
/// 2. Parallel compression with bounded channels
/// 3. Optional reordering for single-end reads
fn run_core(
    request: &CompressionRequest,
    effective_length_class: ReadLengthClass,
    pe_layout: PeLayout,
    single_end: bool,
    mut next_block: impl FnMut() -> Result<Option<Vec<ReadRecord>>>,
) -> Result<CompressionOutcome> {
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

    let block_config = common::codec_profile(request.settings(), effective_length_class).config;
    let mut compressor = BlockCompressor::new(block_config);

    let mut block_id = 0u32;
    let mut archive_id_start = 0u64;
    let mut total_reads = 0u64;
    let mut total_bases = 0u64;
    let mut blocks_written = 0;

    while let Some(block_reads) = next_block()? {
        total_reads += block_reads.len() as u64;
        total_bases += block_reads.iter().map(|r| r.sequence.len() as u64).sum::<u64>();
        let compressed = compressor.compress(&block_reads, block_id)?;
        writer.write_block_with_id(&compressed, archive_id_start)?;
        archive_id_start += block_reads.len() as u64;
        blocks_written += 1;
        block_id += 1;
    }

    writer.patch_total_read_count(total_reads)?;
    let output_bytes = writer.finalize()?;
    output_tx.commit()?;
    log::info!("Streaming compression complete! {} blocks written.", blocks_written);

    let input_bytes = common::input_bytes_for(&input.primary_path, input.secondary_path.as_deref(), total_bases);
    let stats = OperationStats {
        total_reads,
        total_bases,
        input_bytes,
        output_bytes,
        blocks: blocks_written as u64,
        elapsed_seconds: 0.0,
        parse_ms: 0,
        reorder_ms: 0,
        process_ms: 0,
        write_ms: 0,
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
) -> Result<CompressionOutcome> {
    let mut pe_reader = open_fastq_paired(input_path, input2_path)?;
    let target_pairs = (block_size.max(2)) / 2;
    run_core(request, effective_length_class, pe_layout, false, move || {
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
    effective_length_class: ReadLengthClass,
    block_size: usize,
    pe_layout: PeLayout,
) -> Result<CompressionOutcome> {
    let mut parser = if input_path == Path::new("-") {
        crate::fastq::parser::InterleavedPeParser::new(open_fastq_stdin())
    } else {
        open_fastq_interleaved(input_path)?
    };
    let target_pairs = (block_size.max(2)) / 2;
    run_core(request, effective_length_class, pe_layout, false, move || {
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
