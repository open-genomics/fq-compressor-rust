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
// =============================================================================

pub(crate) mod block_pipeline;
pub mod compression;
pub mod decompression;
pub mod decompression_classic;

// =============================================================================
// Constants
// =============================================================================

/// Default number of in-flight blocks (for backpressure)
pub const DEFAULT_MAX_IN_FLIGHT_BLOCKS: usize = 8;
