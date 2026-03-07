// =============================================================================
// fqc-rust - Pipeline Module
// =============================================================================
// Implements parallel compression/decompression pipelines using channels.
//
// The pipeline follows the Pigz model:
// 1. ReaderStage (Serial) - Reads FASTQ and produces chunks of reads
// 2. CompressStage (Parallel) - Compresses chunks to blocks
// 3. WriterStage (Serial) - Writes blocks to disk in order
//
// Key features:
// - Block-level parallelism for compression
// - Memory-bounded operation via bounded channels (backpressure)
// - Progress reporting and cancellation support
// =============================================================================

pub mod compression;
pub mod decompression;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::types::*;

// =============================================================================
// Constants
// =============================================================================

/// Default number of in-flight blocks (for backpressure)
pub const DEFAULT_MAX_IN_FLIGHT_BLOCKS: usize = 8;

/// Default input buffer size (bytes)
pub const DEFAULT_INPUT_BUFFER_SIZE: usize = 64 * 1024 * 1024; // 64MB

/// Default output buffer size (bytes)
pub const DEFAULT_OUTPUT_BUFFER_SIZE: usize = 32 * 1024 * 1024; // 32MB

/// Minimum block size (reads)
pub const MIN_BLOCK_SIZE: usize = 100;

/// Maximum block size (reads)
pub const MAX_BLOCK_SIZE: usize = 1_000_000;

// =============================================================================
// Pipeline Statistics
// =============================================================================

/// Statistics collected during pipeline execution
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    pub total_reads: u64,
    pub total_blocks: u32,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub processing_time_ms: u64,
    pub peak_memory_bytes: usize,
    pub threads_used: usize,
}

impl PipelineStats {
    pub fn compression_ratio(&self) -> f64 {
        if self.input_bytes == 0 { return 1.0; }
        self.output_bytes as f64 / self.input_bytes as f64
    }

    pub fn bits_per_base(&self) -> f64 {
        if self.input_bytes == 0 { return 0.0; }
        (self.output_bytes as f64 * 8.0) / (self.input_bytes as f64 * 0.5)
    }

    pub fn throughput_mbps(&self) -> f64 {
        if self.processing_time_ms == 0 { return 0.0; }
        (self.input_bytes as f64 / (1024.0 * 1024.0)) /
            (self.processing_time_ms as f64 / 1000.0)
    }
}

// =============================================================================
// Progress Info
// =============================================================================

/// Progress information for callbacks
#[derive(Debug, Clone, Default)]
pub struct ProgressInfo {
    pub reads_processed: u64,
    pub total_reads: u64,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub current_block: u32,
    pub elapsed_ms: u64,
}

impl ProgressInfo {
    pub fn ratio(&self) -> f64 {
        if self.total_reads > 0 {
            return self.reads_processed as f64 / self.total_reads as f64;
        }
        if self.total_bytes > 0 {
            return self.bytes_processed as f64 / self.total_bytes as f64;
        }
        0.0
    }

    pub fn estimated_remaining_ms(&self) -> u64 {
        let r = self.ratio();
        if r <= 0.0 || r >= 1.0 { return 0; }
        ((self.elapsed_ms as f64) * (1.0 - r) / r) as u64
    }
}

/// Progress callback type: returns true to continue, false to cancel
pub type ProgressCallback = Box<dyn Fn(&ProgressInfo) -> bool + Send + Sync>;

// =============================================================================
// Pipeline Control
// =============================================================================

/// Shared state for pipeline cancellation and progress tracking
#[derive(Clone)]
pub struct PipelineControl {
    cancelled: Arc<AtomicBool>,
    reads_processed: Arc<AtomicU64>,
    bytes_processed: Arc<AtomicU64>,
}

impl PipelineControl {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            reads_processed: Arc::new(AtomicU64::new(0)),
            bytes_processed: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn add_reads(&self, count: u64) {
        self.reads_processed.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_bytes(&self, count: u64) {
        self.bytes_processed.fetch_add(count, Ordering::Relaxed);
    }

    pub fn reads_processed(&self) -> u64 {
        self.reads_processed.load(Ordering::Relaxed)
    }

    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed.load(Ordering::Relaxed)
    }
}

impl Default for PipelineControl {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ReadChunk - data passed between stages
// =============================================================================

/// A chunk of reads to be processed
pub struct ReadChunk {
    pub reads: Vec<ReadRecord>,
    pub chunk_id: u32,
    pub start_read_id: u64,
    pub is_last: bool,
}

impl ReadChunk {
    pub fn size(&self) -> usize {
        self.reads.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reads.is_empty()
    }

    /// Estimate memory usage of this chunk in bytes
    pub fn estimated_memory(&self) -> usize {
        self.reads.iter()
            .map(|r| r.id.len() + r.sequence.len() + r.quality.len() + 80)
            .sum()
    }
}
