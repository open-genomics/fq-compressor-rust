// =============================================================================
// fqc - Block and worker layout planning
// =============================================================================
//! Resolve block size and codec worker count together.
//!
//! Planning is deliberately outside `BlockCompressorConfig`: codec
//! configuration describes how to encode, while this module decides how many
//! blocks and workers fit the thread and memory budget.

use crate::types::{adaptive_block_size_with_policy, LengthStats, ReadLengthClass};

/// Inputs for the shared block/worker planner used by all compression modes.
///
/// Archive and pipeline pass known totals and auto-split long reads; streaming
/// passes unknown totals and keeps its historical long-read block size.
#[derive(Debug, Clone, Copy)]
pub struct CompressionPlanInput {
    pub requested_block_size: usize,
    pub avg_length: usize,
    pub max_block_bases: usize,
    pub thread_limit: usize,
    pub worker_budget_bytes: u64,
    pub total_reads: Option<u64>,
    pub total_bases: Option<u64>,
    pub read_length_class: ReadLengthClass,
    /// Resident quality model plus fixed worker overhead, excluding records.
    pub codec_model_bytes: usize,
    /// Apply the archive/pipeline long-read base ceiling. Streaming keeps its
    /// historical block size and passes `false`.
    pub auto_split_long_reads: bool,
}

/// Resolved block size and codec worker count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressionPlan {
    pub block_size: usize,
    pub workers: usize,
}

/// Conservative peak estimate for one codec worker.
///
/// `codec_model_bytes` is the resident quality/ID model; record and Zstd
/// buffers are counted separately.
pub fn estimated_worker_bytes(codec_model_bytes: usize, reads: usize, avg_length: usize) -> u64 {
    let record_bytes = reads.saturating_mul(avg_length.saturating_mul(2).saturating_add(128));
    codec_model_bytes
        .saturating_add(record_bytes.saturating_mul(2))
        .saturating_add(16 * 1024 * 1024) as u64
}

/// Codec workers that fit the block-level worker budget.
fn memory_limited_worker_count(
    thread_limit: usize,
    worker_budget_bytes: u64,
    codec_model_bytes: usize,
    block_size: usize,
    avg_length: usize,
) -> usize {
    let worker_bytes = estimated_worker_bytes(codec_model_bytes, block_size, avg_length).max(1);
    usize::try_from(worker_budget_bytes / worker_bytes)
        .unwrap_or(usize::MAX)
        .max(1)
        .min(thread_limit.max(1))
}

fn memory_limited_workers(input: CompressionPlanInput, block_size: usize, block_count: usize) -> usize {
    memory_limited_worker_count(
        input.thread_limit,
        input.worker_budget_bytes,
        input.codec_model_bytes,
        block_size,
        input.avg_length,
    )
    .min(block_count.max(1))
}

/// Resolve block size and worker count together.
///
/// A larger block raises per-worker memory and can reduce the worker cap,
/// which raises the target block size again. Iterate the monotone relation to
/// a small fixed point instead of sizing blocks from the requested thread
/// count while later running fewer workers.
pub fn plan_compression_layout(input: CompressionPlanInput) -> CompressionPlan {
    let thread_limit = input.thread_limit.max(1);
    let mut workers = thread_limit;
    let mut block_size = adaptive_block_size_with_policy(
        input.requested_block_size,
        input.read_length_class,
        input.avg_length,
        input.max_block_bases,
        workers,
        input.total_reads,
        input.total_bases,
        input.auto_split_long_reads,
    )
    .max(1);

    for _ in 0..8 {
        let block_count = input
            .total_reads
            .map(|reads| usize::try_from(reads).unwrap_or(usize::MAX).div_ceil(block_size).max(1))
            .unwrap_or_else(|| thread_limit.saturating_mul(2).max(1));
        let next_workers = memory_limited_workers(input, block_size, block_count);
        let next_block_size = adaptive_block_size_with_policy(
            input.requested_block_size,
            input.read_length_class,
            input.avg_length,
            input.max_block_bases,
            next_workers,
            input.total_reads,
            input.total_bases,
            input.auto_split_long_reads,
        )
        .max(1);
        if next_workers == workers && next_block_size == block_size {
            break;
        }
        workers = next_workers;
        block_size = next_block_size;
    }

    CompressionPlan { block_size, workers }
}

/// Available codec workers before applying the auto-mode memory guard.
pub fn effective_worker_count(threads: usize) -> usize {
    if threads == 0 {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
    } else {
        threads
    }
    .max(1)
}

/// Codec worker limit shared by archive, pipeline and streaming.
pub fn codec_thread_limit(threads: usize, memory_limit_mb: usize) -> usize {
    let workers = effective_worker_count(threads);
    if threads == 0 && memory_limit_mb == 0 {
        workers.min(crate::types::DEFAULT_AUTO_CODEC_WORKERS)
    } else {
        workers
    }
}

/// Budget available to codec workers after reserving reader/writer headroom.
pub fn worker_budget_bytes(memory_limit_mb: usize) -> u64 {
    crate::memory::resolve_compress_limit_bytes(memory_limit_mb) / 3 * 2
}

/// Exact average read length, never zero.
pub fn avg_read_length(total_bases: u64, total_reads: u64) -> usize {
    usize::try_from(total_bases / total_reads.max(1))
        .unwrap_or(usize::MAX)
        .max(1)
}

/// Bound the number of compressed blocks in flight by the block buffer budget.
///
/// `max_in_flight` is the caller's configured ceiling (pipeline tests use small
/// values to force out-of-order completion); memory pressure can only lower it.
pub fn effective_in_flight_blocks(
    block_size: usize,
    stats: &LengthStats,
    memory_limit_mb: usize,
    max_in_flight: usize,
) -> usize {
    let max_in_flight = max_in_flight.max(1);
    let budget = crate::memory::auto_memory_budget(memory_limit_mb);
    let bytes_per_read = stats.avg_length.max(1).saturating_mul(3).saturating_add(80);
    let chunk_bytes = block_size.saturating_mul(bytes_per_read);
    if chunk_bytes == 0 {
        return max_in_flight;
    }
    budget
        .block_buffer_bytes()
        .saturating_div(chunk_bytes)
        .clamp(1, max_in_flight)
}
