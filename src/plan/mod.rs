// =============================================================================
// fqc - Validated execution plans
// =============================================================================
//! Shared execution plans, validation vocabulary and block limits.
//!
//! This module owns the stable enums and limits so CLI, engine and pipeline
//! agree on one source. Request validation lives with the request types.

pub mod block_plan;
pub mod compress;
pub mod decompress;
pub mod length;

pub use block_plan::{
    avg_read_length, codec_thread_limit, effective_in_flight_blocks, effective_worker_count,
    memory_limited_worker_count, plan_compression_layout, worker_budget_bytes, CompressionPlan, CompressionPlanInput,
};
pub use length::{effective_read_length_class, length_stats_from_records_sampled, DEFAULT_LENGTH_SAMPLE_READS};

/// Execution mode for compression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionExecutionMode {
    /// Archive mode: full-featured compression with reordering support.
    Archive,
    /// Streaming mode: single-pass compression without reordering.
    Streaming,
    /// Pipeline mode: concurrent block compression with in-flight buffering.
    Pipeline,
}

/// Minimum explicit block size accepted by pipeline compression.
pub const MIN_BLOCK_SIZE: usize = 100;

/// Maximum explicit block size accepted by every compression mode.
pub const MAX_BLOCK_SIZE: usize = 1_000_000;

/// Read reordering policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReorderMode {
    /// Let the analyzer skip reordering when the input has no reorder value.
    #[default]
    Auto,
    /// Always reorder eligible short single-end input.
    Always,
    /// Never reorder.
    Never,
}

impl ReorderMode {
    pub fn enabled(self) -> bool {
        !matches!(self, Self::Never)
    }

    pub fn auto(self) -> bool {
        matches!(self, Self::Auto)
    }
}
