// =============================================================================
// fqc - Decode / verify resource policy
// =============================================================================

use super::system::get_available_memory_mb;
use crate::error::{FqcError, Result};
use std::io::Read;

// =============================================================================
// DecodeBudget — operation-scoped limits for decompress / verify
// =============================================================================

/// Minimum user-visible decode budget (tests and tiny fixtures).
pub const MIN_DECODE_MEMORY_MB: usize = 16;
/// Absolute ceiling even when the user/OS reports more RAM.
pub const HARD_MAX_DECODE_MEMORY_MB: usize = 512 * 1024;
/// Hard cap on index entries regardless of budget (DoS / structure).
pub const HARD_MAX_INDEX_ENTRIES: u64 = 10_000_000;
/// Hard cap on total reads in one archive for reorder/original-order planning.
pub const HARD_MAX_TOTAL_READS: u64 = 1_000_000_000;
/// Max zstd expansion factor when no tighter bound is known.
pub const ZSTD_MAX_EXPANSION: u64 = 64;

/// Per-operation decode/verify resource policy (no global mutable state).
#[derive(Debug, Clone)]
pub struct DecodeBudget {
    /// Soft operation limit in bytes (from `--memory-limit` or auto).
    pub limit_bytes: u64,
    /// Max bytes for a single allocation (stream, index, map, zstd output).
    pub max_alloc_bytes: u64,
    /// Max block-index entries.
    pub max_index_entries: u64,
    /// Max total reads (reorder / original-order).
    pub max_total_reads: u64,
    /// True when limit came from automatic selection (`--memory-limit 0`).
    pub automatic: bool,
}

impl DecodeBudget {
    /// Resolve user MB (`0` = automatic = 75% of available, still capped).
    pub fn resolve(user_limit_mb: usize) -> Self {
        let automatic = user_limit_mb == 0;
        let mut limit_mb = if automatic {
            ((get_available_memory_mb() as f64) * 0.75) as usize
        } else {
            user_limit_mb
        };
        limit_mb = limit_mb.clamp(MIN_DECODE_MEMORY_MB, HARD_MAX_DECODE_MEMORY_MB);
        let limit_bytes = (limit_mb as u64).saturating_mul(1024 * 1024);
        Self {
            limit_bytes,
            max_alloc_bytes: (limit_bytes / 2).max(1024 * 1024),
            max_index_entries: HARD_MAX_INDEX_ENTRIES
                .min(limit_bytes / 64) // ~64B accounting per entry
                .max(1),
            max_total_reads: HARD_MAX_TOTAL_READS.min(limit_bytes / 32).max(1),
            automatic,
        }
    }

    pub fn automatic() -> Self {
        Self::resolve(0)
    }

    pub fn checked_usize(&self, value: u64, location: &str) -> Result<usize> {
        usize::try_from(value).map_err(|_| FqcError::ResourceLimit {
            location: location.to_string(),
            declared: value,
            allowed: usize::MAX as u64,
        })
    }

    pub fn check_alloc(&self, declared: u64, location: &str) -> Result<()> {
        if declared > self.max_alloc_bytes {
            return Err(FqcError::ResourceLimit {
                location: location.to_string(),
                declared,
                allowed: self.max_alloc_bytes,
            });
        }
        Ok(())
    }

    pub fn check_index_entries(&self, declared: u64, file_region_bytes: u64, entry_size: u64) -> Result<()> {
        if declared > self.max_index_entries {
            return Err(FqcError::ResourceLimit {
                location: "block_index.num_blocks".to_string(),
                declared,
                allowed: self.max_index_entries,
            });
        }
        if entry_size == 0 {
            return Err(FqcError::Format("block index entry size is zero".to_string()));
        }
        let max_by_file = file_region_bytes / entry_size;
        if declared > max_by_file {
            return Err(FqcError::ResourceLimit {
                location: "block_index.num_blocks vs file region".to_string(),
                declared,
                allowed: max_by_file,
            });
        }
        let index_bytes = declared.saturating_mul(entry_size);
        self.check_alloc(index_bytes, "block_index.entries")?;
        Ok(())
    }

    pub fn check_total_reads(&self, declared: u64, location: &str) -> Result<()> {
        if declared > self.max_total_reads {
            return Err(FqcError::ResourceLimit {
                location: location.to_string(),
                declared,
                allowed: self.max_total_reads,
            });
        }
        Ok(())
    }

    /// Estimate peak for original-order: maps + all ReadRecords skeleton + one block.
    pub fn check_original_order_peak(
        &self,
        total_reads: u64,
        max_block_compressed: u64,
        avg_bases_hint: u64,
    ) -> Result<()> {
        let map_bytes = total_reads.saturating_mul(16); // forward+reverse u64
        let record_bytes = total_reads.saturating_mul(avg_bases_hint.saturating_mul(2).saturating_add(128));
        let peak = map_bytes
            .saturating_add(record_bytes)
            .saturating_add(max_block_compressed.saturating_mul(2));
        if peak > self.limit_bytes {
            return Err(FqcError::ResourceLimit {
                location: "original-order peak estimate".to_string(),
                declared: peak,
                allowed: self.limit_bytes,
            });
        }
        Ok(())
    }

    /// Bound parallel batch size by threads and budget; never returns 0.
    pub fn parallel_batch_size(&self, threads: usize, block_compressed_hint: u64) -> Result<usize> {
        let threads = threads.max(1);
        let per_block = block_compressed_hint.max(64 * 1024).saturating_mul(3);
        if per_block > self.limit_bytes {
            return Err(FqcError::ResourceLimit {
                location: "single block decode peak".to_string(),
                declared: per_block,
                allowed: self.limit_bytes,
            });
        }
        let by_budget = (self.limit_bytes / per_block).max(1) as usize;
        Ok((threads * 2).max(1).min(by_budget).min(64))
    }

    /// Portion of the budget available to codec workers after reserving room
    /// for the reader, writer, block buffers, and allocator/thread overhead.
    pub fn worker_budget_bytes(&self) -> u64 {
        self.limit_bytes / 3 * 2
    }

    /// Upper bound on concurrent codec workers from their resident estimate.
    ///
    /// This is deliberately conservative and never returns 0, so a tiny
    /// explicit budget still produces a valid single-worker run.
    pub fn max_workers_for(&self, worker_bytes: u64) -> usize {
        self.max_workers_for_budget(worker_bytes, self.limit_bytes)
    }

    /// Worker cap for an explicit budget slice.
    pub fn max_workers_for_budget(&self, worker_bytes: u64, budget_bytes: u64) -> usize {
        if worker_bytes == 0 {
            return 64;
        }
        usize::try_from(budget_bytes / worker_bytes).unwrap_or(1).clamp(1, 64)
    }
}

/// Zstd decompress with an explicit output ceiling (never unbounded `decode_all`).
///
/// Uses the streaming decoder instead of `zstd::bulk::decompress` so a forged
/// `max_out` (derived from attacker-controlled read lengths) cannot make zstd
/// preallocate that many bytes before decoding a single byte. The ceiling is
/// enforced while reading; memory grows with actual output, not with `max_out`.
pub fn zstd_decompress_bounded(data: &[u8], max_out: usize, location: &str) -> Result<Vec<u8>> {
    if max_out == 0 {
        return Err(FqcError::ResourceLimit {
            location: location.to_string(),
            declared: 0,
            allowed: 0,
        });
    }
    let mut decoder = zstd::stream::read::Decoder::with_buffer(data)
        .map_err(|e| FqcError::Decompression(format!("{location}: zstd decoder init failed: {e}")))?;
    // Do not reserve max_out: it is untrusted and may be enormous.
    let mut out = Vec::with_capacity(max_out.min(64 * 1024));
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = decoder.read(&mut buf).map_err(|e| {
            FqcError::Decompression(format!("{location}: zstd decompress failed: {e} (max_out={max_out})"))
        })?;
        if n == 0 {
            return Ok(out);
        }
        let new_len = out.len().saturating_add(n);
        if new_len > max_out {
            return Err(FqcError::ResourceLimit {
                location: location.to_string(),
                declared: new_len as u64,
                allowed: max_out as u64,
            });
        }
        out.extend_from_slice(&buf[..n]);
    }
}

/// Conservative max output when only compressed length is known.
pub fn zstd_default_max_out(compressed_len: usize, budget: &DecodeBudget) -> usize {
    let expanded = (compressed_len as u64).saturating_mul(ZSTD_MAX_EXPANSION);
    expanded.min(budget.max_alloc_bytes).max(1) as usize
}
