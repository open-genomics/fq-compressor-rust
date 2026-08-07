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
    pub total_bases: u64,
    pub total_blocks: u32,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub processing_time_ms: u64,
    pub reorder_map_written: bool,
}

impl PipelineStats {
    pub fn compression_ratio(&self) -> f64 {
        if self.input_bytes == 0 {
            return 1.0;
        }
        self.output_bytes as f64 / self.input_bytes as f64
    }

    pub fn throughput_mbps(&self) -> f64 {
        if self.processing_time_ms == 0 {
            return 0.0;
        }
        (self.input_bytes as f64 / (1024.0 * 1024.0)) / (self.processing_time_ms as f64 / 1000.0)
    }
}

// =============================================================================
// Pipeline Control
// =============================================================================

/// Shared state for pipeline cancellation and progress tracking
#[derive(Clone)]
pub struct PipelineControl {
    cancelled: Arc<AtomicBool>,
    reads_processed: Arc<AtomicU64>,
}

impl PipelineControl {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            reads_processed: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn add_reads(&self, count: u64) {
        self.reads_processed.fetch_add(count, Ordering::Relaxed);
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
}
