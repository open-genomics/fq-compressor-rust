// =============================================================================
// fqc - Archive ingest and compression budgets
// =============================================================================

use super::decode::{HARD_MAX_DECODE_MEMORY_MB, MIN_DECODE_MEMORY_MB};
use super::system::get_available_memory_mb;
use crate::error::{FqcError, Result};
use crate::types::ReadRecord;

// =============================================================================
// Compress archive ingest budget (`--memory-limit` on archive compress)
// =============================================================================

/// Minimum user-visible compress budget (matches decode so `--memory-limit 16` is legal).
pub const MIN_COMPRESS_MEMORY_MB: usize = MIN_DECODE_MEMORY_MB;
/// Absolute ceiling even when the user/OS reports more RAM.
pub const HARD_MAX_COMPRESS_MEMORY_MB: usize = HARD_MAX_DECODE_MEMORY_MB;
/// Extra copies in archive mode (sequence clone + block extract).
pub const ARCHIVE_INGEST_PEAK_FACTOR: u64 = 2;
const ARCHIVE_RECORD_OVERHEAD: u64 = 128;

/// Resolve compress `--memory-limit` MB (`0` = automatic = 75% of available, still capped).
pub fn resolve_compress_limit_mb(user_limit_mb: usize) -> usize {
    let limit_mb = if user_limit_mb == 0 {
        ((get_available_memory_mb() as f64) * 0.75) as usize
    } else {
        user_limit_mb
    };
    limit_mb.clamp(MIN_COMPRESS_MEMORY_MB, HARD_MAX_COMPRESS_MEMORY_MB)
}

pub fn resolve_compress_limit_bytes(user_limit_mb: usize) -> u64 {
    (resolve_compress_limit_mb(user_limit_mb) as u64).saturating_mul(1024 * 1024)
}

pub fn estimate_record_bytes(record: &ReadRecord) -> u64 {
    (record.id.len() as u64)
        .saturating_add(record.comment.len() as u64)
        .saturating_add(record.sequence.len() as u64)
        .saturating_add(record.quality.len() as u64)
        .saturating_add(ARCHIVE_RECORD_OVERHEAD)
}

/// Held-record estimate for `num_reads` of `avg_length` (seq+qual + overhead).
pub fn estimate_archive_ingest_bytes(num_reads: usize, avg_length: usize) -> u64 {
    let per_read = (avg_length as u64)
        .saturating_mul(2)
        .saturating_add(ARCHIVE_RECORD_OVERHEAD);
    (num_reads as u64)
        .saturating_mul(per_read)
        .saturating_mul(ARCHIVE_INGEST_PEAK_FACTOR)
}

/// Compare a synthetic ingest peak against the resolved compress budget.
pub fn check_archive_ingest(num_reads: usize, avg_length: usize, user_limit_mb: usize) -> Result<()> {
    let allowed = resolve_compress_limit_bytes(user_limit_mb);
    let declared = estimate_archive_ingest_bytes(num_reads, avg_length);
    if declared > allowed {
        return Err(FqcError::ResourceLimit {
            location: "archive ingest peak estimate (use --streaming)".to_string(),
            declared,
            allowed,
        });
    }
    Ok(())
}

/// Add one held record to the running archive ingest estimate and fail if peak exceeds the budget.
pub fn account_archive_ingest(held_bytes: &mut u64, record: &ReadRecord, limit_bytes: u64) -> Result<()> {
    *held_bytes = held_bytes.saturating_add(estimate_record_bytes(record));
    let peak = held_bytes.saturating_mul(ARCHIVE_INGEST_PEAK_FACTOR);
    if peak > limit_bytes {
        return Err(FqcError::ResourceLimit {
            location: "archive ingest peak estimate (use --streaming)".to_string(),
            declared: peak,
            allowed: limit_bytes,
        });
    }
    Ok(())
}

pub fn push_archive_record(
    records: &mut Vec<ReadRecord>,
    rec: ReadRecord,
    held_bytes: &mut u64,
    limit_bytes: u64,
) -> Result<()> {
    account_archive_ingest(held_bytes, &rec, limit_bytes)?;
    records.push(rec);
    Ok(())
}
