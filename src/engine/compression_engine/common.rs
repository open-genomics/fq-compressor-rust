// =============================================================================
// fqc - Shared compression planning and input helpers
// =============================================================================

use crate::algo::block_compressor::{BlockCodecSettings, BlockCompressorConfig};
use crate::error::Result;
use crate::fastq::parser::{open_fastq, open_fastq_interleaved, open_fastq_paired, open_fastq_stdin};
use crate::plan::compress::CompressionSettings;
use crate::plan::DEFAULT_LENGTH_SAMPLE_READS;
use crate::types::*;
use std::path::Path;

/// Compression settings resolved once for codec construction and layout planning.
///
/// Keeping this mapping in one place stops archive, pipeline and streaming from
/// drifting on how CLI settings become codec configuration and worker budgets.
pub(super) struct CodecProfile {
    pub config: BlockCompressorConfig,
    pub thread_limit: usize,
    pub worker_budget_bytes: u64,
    pub model_bytes: usize,
}

pub(super) fn codec_profile(settings: &CompressionSettings, read_length_class: ReadLengthClass) -> CodecProfile {
    let config = BlockCodecSettings::from_compression_level(settings.quality_mode, settings.id_mode, settings.level)
        .with_read_length_class(read_length_class);
    let model_bytes = config.to_quality_config().estimated_model_bytes();
    CodecProfile {
        config,
        thread_limit: crate::plan::codec_thread_limit(settings.threads, settings.memory_limit_mb),
        worker_budget_bytes: crate::plan::worker_budget_bytes(settings.memory_limit_mb),
        model_bytes,
    }
}

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

/// Inspect read lengths before streaming so block sizing fits the input.
///
/// Reads at most `scan_all ? all : DEFAULT_LENGTH_SAMPLE_READS` lengths and
/// never buffers full records; stdin has no seekable pre-scan and returns
/// `None` so callers fall back to their default class.
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
            lengths.push(r1.sequence.len());
            lengths.push(r2.sequence.len());
            if lengths.len() >= sample_limit {
                break;
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
