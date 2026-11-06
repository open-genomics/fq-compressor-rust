// =============================================================================
// fqc - Memory policy
// =============================================================================
//! Memory budgets and estimates for ingest, encode and decode.
//!
//! The modules are split by operation so a caller does not import decode
//! ceilings just to size a compression worker.

mod budget;
mod decode;
mod encode;
mod system;

pub use budget::{auto_memory_budget, ChunkingStrategy, MemoryBudget};
pub use decode::{
    zstd_decompress_bounded, zstd_default_max_out, DecodeBudget, HARD_MAX_INDEX_ENTRIES, HARD_MAX_TOTAL_READS,
    MIN_DECODE_MEMORY_MB, ZSTD_MAX_EXPANSION,
};
pub use encode::{
    account_archive_ingest, check_archive_ingest, estimate_archive_ingest_bytes, estimate_record_bytes,
    push_archive_record, resolve_compress_limit_bytes, resolve_compress_limit_mb, ARCHIVE_INGEST_PEAK_FACTOR,
    HARD_MAX_COMPRESS_MEMORY_MB, MIN_COMPRESS_MEMORY_MB,
};
pub use system::get_available_memory_mb;
