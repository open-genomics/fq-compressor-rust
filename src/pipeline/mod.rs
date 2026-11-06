// =============================================================================
// fqc-rust - Pipeline Module
// =============================================================================
// Implements parallel compression/decompression pipelines using channels.
//
// Compression still performs a full ingest: reads are parsed and globally
// analyzed before blocks are built. `block_pipeline` then bounds the number of
// compressed blocks in flight while ordering writes; it is not a bounded-ingest
// streaming path (use `--streaming` for that).
//
// 1. Reader/ingest - parses and (optionally) reorders all reads
// 2. CompressStage (Parallel) - compresses chunks to blocks
// 3. WriterStage (Serial) - writes blocks to disk in order
//
// Key features:
// - Block-level parallelism for compression
// - Bounded in-flight compressed blocks with backpressure during the block phase
// - Progress reporting and cancellation support for decompression
// =============================================================================

pub(crate) mod block_pipeline;
pub mod compression;
pub mod decompression;
pub mod decompression_classic;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

// =============================================================================
// Constants
// =============================================================================

/// Default number of in-flight blocks (for backpressure)
pub const DEFAULT_MAX_IN_FLIGHT_BLOCKS: usize = 8;

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
