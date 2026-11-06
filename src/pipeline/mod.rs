// =============================================================================
// fqc-rust - Pipeline Module
// =============================================================================
// Implements parallel compression/decompression pipelines using channels.
//
// `block_pipeline` is the single block stage for all three compression modes:
// a block source, parallel compressors and one ordered writer. Archive and
// pipeline feed it pre-chunked reads after a full ingest; streaming pulls
// blocks from the parser on demand, so its ingest stays bounded.
//
// 1. Reader/source - pulls the next block (pre-chunked or parsed on demand)
// 2. Compressors (Parallel) - encode blocks, completing out of order
// 3. Writer (Serial) - writes blocks to disk in block order
//
// Key features:
// - Block-level parallelism for compression
// - Bounded channels with backpressure in every mode
// =============================================================================

pub(crate) mod block_pipeline;
pub mod compression;
pub mod decompression;
pub(crate) mod decompression_writer;

// =============================================================================
// Constants
// =============================================================================

/// Default number of in-flight blocks (for backpressure)
pub const DEFAULT_MAX_IN_FLIGHT_BLOCKS: usize = 8;
