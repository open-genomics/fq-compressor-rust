// =============================================================================
// fqc - Shared compression planning and input helpers
// =============================================================================

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use crate::block::EncodedBlock;
use crate::error::Result;
use crate::fastq::parser::{open_fastq, open_fastq_interleaved, open_fastq_paired, open_fastq_stdin};
use crate::types::*;
use rayon::prelude::*;
use std::path::Path;

pub(super) const DEFAULT_LENGTH_SAMPLE_READS: usize = 4_096;

pub(super) fn input_bytes_for(primary: &Path, secondary: Option<&Path>, fallback: u64) -> u64 {
    let mut paths = vec![primary];
    if let Some(path) = secondary {
        paths.push(path);
    }
    crate::stats::input_bytes_for_paths(&paths, fallback)
}

pub(super) fn read_all_records(input_path: &Path, memory_limit_mb: usize) -> Result<Vec<ReadRecord>> {
    if input_path == Path::new("-") {
        open_fastq_stdin().collect_all_within_archive_budget(memory_limit_mb)
    } else {
        open_fastq(input_path)?.collect_all_within_archive_budget(memory_limit_mb)
    }
}

pub(super) fn read_all_paired_records(
    input_path: &Path,
    input2_path: &Path,
    pe_layout: PeLayout,
    memory_limit_mb: usize,
) -> Result<Vec<ReadRecord>> {
    open_fastq_paired(input_path, input2_path)?.collect_pairs_within_archive_budget(pe_layout, memory_limit_mb)
}

pub(super) fn read_all_interleaved_records(
    input_path: &Path,
    pe_layout: PeLayout,
    memory_limit_mb: usize,
) -> Result<Vec<ReadRecord>> {
    let mut parser = if input_path == Path::new("-") {
        crate::fastq::parser::InterleavedPeParser::new(open_fastq_stdin())
    } else {
        open_fastq_interleaved(input_path)?
    };
    parser.collect_pairs_within_archive_budget(pe_layout, memory_limit_mb)
}

pub(super) fn length_stats_from_records_sampled(records: &[ReadRecord], scan_all: bool) -> LengthStats {
    if scan_all || records.len() <= DEFAULT_LENGTH_SAMPLE_READS {
        let lengths: Vec<usize> = records.iter().map(|r| r.sequence.len()).collect();
        return LengthStats::from_lengths(&lengths);
    }

    let sample_size = DEFAULT_LENGTH_SAMPLE_READS.min(records.len());
    let lengths: Vec<usize> = (0..sample_size)
        .map(|i| {
            let idx = i * records.len() / sample_size;
            records[idx].sequence.len()
        })
        .collect();
    LengthStats::from_lengths(&lengths)
}

pub(super) fn effective_length_class(requested: Option<ReadLengthClass>, stats: &LengthStats) -> ReadLengthClass {
    requested.unwrap_or_else(|| crate::types::classify_read_length(stats.median_length, stats.max_length))
}

pub(super) fn codec_thread_limit(threads: usize, memory_limit_mb: usize) -> usize {
    crate::plan::codec_thread_limit(threads, memory_limit_mb)
}

pub(super) fn compress_blocks(
    block_read_sets: &[(u32, Vec<ReadRecord>)],
    max_workers: usize,
    block_config: &BlockCompressorConfig,
) -> (Vec<Result<EncodedBlock>>, u64) {
    let process_ns = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let compress_one = |(block_id, reads): &(u32, Vec<ReadRecord>)| {
        let t = std::time::Instant::now();
        let mut compressor = BlockCompressor::new(block_config.clone());
        let result = compressor.compress(reads, *block_id);
        process_ns.fetch_add(t.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
        result
    };

    let compressed = if max_workers <= 1 {
        block_read_sets.iter().map(compress_one).collect()
    } else {
        match rayon::ThreadPoolBuilder::new().num_threads(max_workers).build() {
            Ok(pool) => pool.install(|| block_read_sets.par_iter().map(compress_one).collect()),
            Err(error) => {
                log::warn!("Failed to build worker pool ({error}); using global pool");
                block_read_sets.par_iter().map(compress_one).collect()
            }
        }
    };

    let process_ms = process_ns.load(std::sync::atomic::Ordering::Relaxed) / 1_000_000;
    (compressed, process_ms)
}

pub(super) fn effective_block_size(
    requested_block_size: usize,
    class: ReadLengthClass,
    stats: &LengthStats,
    max_block_bases: usize,
    threads: usize,
    total_reads: Option<u64>,
    total_bases: Option<u64>,
    auto_split_long_reads: bool,
) -> usize {
    crate::types::adaptive_block_size_with_policy(
        requested_block_size,
        class,
        stats.avg_length,
        max_block_bases,
        threads,
        total_reads,
        total_bases,
        auto_split_long_reads,
    )
}

// =============================================================================
// Streaming mode helpers
// =============================================================================

/// Convergence core for all streaming topologies. Pulls pre-batched blocks
/// (in archive order) from `next_block` and writes them without ever
/// holding the full read set in memory. Single-end and paired adapters own
/// the accumulation/arrangement and hand the core complete blocks.
pub(super) fn inspect_input_lengths_for_streaming(
    input_path: &Path,
    input2_path: Option<&Path>,
    is_interleaved: bool,
    scan_all: bool,
) -> Result<Option<LengthStats>> {
    if input_path == Path::new("-") {
        return Ok(None);
    }

    let sample_limit = if scan_all {
        usize::MAX
    } else {
        DEFAULT_LENGTH_SAMPLE_READS
    };
    let mut lengths = Vec::new();

    if let Some(path2) = input2_path {
        let mut reader = open_fastq_paired(input_path, path2)?;
        while let Some((r1, r2)) = reader.next_pair()? {
            lengths.push(r1.sequence.len());
            lengths.push(r2.sequence.len());
            if lengths.len() >= sample_limit {
                break;
            }
        }
    } else if is_interleaved {
        let mut parser = open_fastq_interleaved(input_path)?;
        while let Some((r1, r2)) = parser.next_pair()? {
            if scan_all || lengths.len() < sample_limit {
                lengths.push(r1.sequence.len());
                lengths.push(r2.sequence.len());
            }
        }
    } else {
        let mut parser = open_fastq(input_path)?;
        while let Some(record) = parser.next_record()? {
            lengths.push(record.sequence.len());
            if lengths.len() >= sample_limit {
                break;
            }
        }
    }

    if lengths.is_empty() {
        Ok(None)
    } else {
        Ok(Some(LengthStats::from_lengths(&lengths)))
    }
}

pub(super) fn effective_in_flight_blocks(block_size: usize, stats: &LengthStats, memory_limit_mb: usize) -> usize {
    use crate::memory::auto_memory_budget;
    use crate::pipeline::DEFAULT_MAX_IN_FLIGHT_BLOCKS;

    let budget = auto_memory_budget(memory_limit_mb);
    let bytes_per_read = stats.avg_length.max(1).saturating_mul(3).saturating_add(80);
    let chunk_bytes = block_size.saturating_mul(bytes_per_read);
    if chunk_bytes == 0 {
        return DEFAULT_MAX_IN_FLIGHT_BLOCKS;
    }

    budget
        .block_buffer_bytes()
        .saturating_div(chunk_bytes)
        .clamp(1, DEFAULT_MAX_IN_FLIGHT_BLOCKS)
}
