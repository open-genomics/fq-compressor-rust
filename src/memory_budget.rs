// =============================================================================
// fqc-rust - Memory Budget Management
// =============================================================================
// Memory budget calculation, monitoring, and chunking strategy for large files.
// =============================================================================

// =============================================================================
// Constants
// =============================================================================

const MIN_MEMORY_LIMIT_MB: usize = 256;
const DEFAULT_BLOCK_BUFFER_MB: usize = 512;
const DEFAULT_WORKER_STACK_MB: usize = 64;
const MEMORY_PER_READ_PHASE1: usize = 24;
const MEMORY_PER_READ_PHASE2: usize = 50;
const MEMORY_SAFETY_MARGIN: f64 = 1.2;

// =============================================================================
// MemoryBudget
// =============================================================================

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct MemoryBudget {
    pub max_total_mb: usize,
    pub block_buffer_mb: usize,
    pub worker_stack_mb: usize,
}

impl MemoryBudget {
    pub fn from_memory_limit(memory_limit_mb: usize) -> Self {
        let total_mb = memory_limit_mb.max(MIN_MEMORY_LIMIT_MB);
        let block_mb = (total_mb / 16).min(DEFAULT_BLOCK_BUFFER_MB);

        Self {
            max_total_mb: total_mb,
            block_buffer_mb: block_mb,
            worker_stack_mb: DEFAULT_WORKER_STACK_MB,
        }
    }

    pub fn block_buffer_bytes(&self) -> usize {
        self.block_buffer_mb * 1024 * 1024
    }

    pub fn phase2_available_bytes(&self) -> usize {
        let used = self.block_buffer_mb + self.worker_stack_mb;
        if self.max_total_mb > used {
            (self.max_total_mb - used) * 1024 * 1024
        } else {
            MIN_MEMORY_LIMIT_MB * 1024 * 1024
        }
    }
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self::from_memory_limit(8192)
    }
}

// =============================================================================
// MemoryEstimator
// =============================================================================

pub struct MemoryEstimator {
    budget: MemoryBudget,
}

impl MemoryEstimator {
    pub fn new(budget: MemoryBudget) -> Self {
        Self { budget }
    }

    pub fn optimal_block_size(&self, num_threads: usize) -> usize {
        let available = self.budget.phase2_available_bytes();
        let per_thread = (available as f64 / (num_threads as f64 * MEMORY_SAFETY_MARGIN)) as usize;
        let block_size = per_thread / MEMORY_PER_READ_PHASE2;
        block_size.clamp(1000, 500_000)
    }
}

// =============================================================================
// DecodeBudget — operation-scoped limits for decompress / verify
// =============================================================================

use crate::error::{FqcError, Result};
use crate::types::ReadRecord;

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
}

/// Zstd decompress with an explicit output ceiling (never unbounded `decode_all`).
pub fn zstd_decompress_bounded(data: &[u8], max_out: usize, location: &str) -> Result<Vec<u8>> {
    if max_out == 0 {
        return Err(FqcError::ResourceLimit {
            location: location.to_string(),
            declared: 0,
            allowed: 0,
        });
    }
    zstd::bulk::decompress(data, max_out).map_err(|e| {
        // Distinguish capacity hits from corrupt frames when possible.
        let msg = e.to_string();
        if msg.contains("Destination buffer is too small") || msg.contains("capacity") {
            FqcError::ResourceLimit {
                location: location.to_string(),
                declared: max_out as u64 + 1,
                allowed: max_out as u64,
            }
        } else {
            FqcError::Decompression(format!("{location}: zstd decompress failed: {e} (max_out={max_out})"))
        }
    })
}

/// Conservative max output when only compressed length is known.
pub fn zstd_default_max_out(compressed_len: usize, budget: &DecodeBudget) -> usize {
    let expanded = (compressed_len as u64).saturating_mul(ZSTD_MAX_EXPANSION);
    expanded.min(budget.max_alloc_bytes).max(1) as usize
}

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

// =============================================================================
// System Memory Detection
// =============================================================================

/// Get available system memory in MB.
pub fn get_available_memory_mb() -> usize {
    #[cfg(target_os = "windows")]
    {
        get_available_memory_windows()
    }
    #[cfg(target_os = "linux")]
    {
        get_available_memory_linux()
    }
    #[cfg(target_os = "macos")]
    {
        get_available_memory_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        8192 // Default fallback
    }
}

#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn get_available_memory_windows() -> usize {
    use std::mem;

    #[repr(C)]
    struct MemoryStatusEx {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
        ull_total_page_file: u64,
        ull_avail_page_file: u64,
        ull_total_virtual: u64,
        ull_avail_virtual: u64,
        ull_avail_extended_virtual: u64,
    }

    extern "system" {
        fn GlobalMemoryStatusEx(lpBuffer: *mut MemoryStatusEx) -> i32;
    }

    unsafe {
        let mut status: MemoryStatusEx = mem::zeroed();
        status.dw_length = mem::size_of::<MemoryStatusEx>() as u32;
        if GlobalMemoryStatusEx(&mut status) != 0 {
            (status.ull_avail_phys / (1024 * 1024)) as usize
        } else {
            8192
        }
    }
}

#[cfg(target_os = "linux")]
fn get_available_memory_linux() -> usize {
    if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
        for line in contents.lines() {
            if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<usize>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    8192
}

#[cfg(target_os = "macos")]
fn get_available_memory_macos() -> usize {
    // Use sysctl to get physical memory
    use std::process::Command;
    if let Ok(output) = Command::new("sysctl").arg("-n").arg("hw.memsize").output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            if let Ok(bytes) = s.trim().parse::<u64>() {
                return (bytes / (1024 * 1024)) as usize;
            }
        }
    }
    8192
}

// =============================================================================
// Dynamic Chunking Strategy
// =============================================================================

/// Strategy for divide-and-conquer chunking when data exceeds memory limits
#[derive(Debug, Clone)]
pub struct ChunkingStrategy {
    pub num_chunks: usize,
    pub reads_per_chunk: usize,
    pub block_size: usize,
    pub blocks_per_chunk: usize,
    pub estimated_peak_mb: usize,
}

impl ChunkingStrategy {
    /// Compute an optimal chunking strategy given dataset and system constraints
    pub fn compute(
        total_reads: usize,
        avg_read_length: usize,
        block_size: usize,
        num_threads: usize,
        memory_limit_mb: usize,
    ) -> Self {
        let effective_limit = if memory_limit_mb == 0 {
            let system_mb = get_available_memory_mb();
            // Use 75% of available memory
            (system_mb as f64 * 0.75) as usize
        } else {
            memory_limit_mb
        };

        let bytes_per_read = avg_read_length * 3 + 80; // seq + qual + id + overhead
        let phase1_per_read = MEMORY_PER_READ_PHASE1 + bytes_per_read;
        let phase1_total_mb = (total_reads * phase1_per_read) / (1024 * 1024);

        let available_mb = effective_limit.max(MIN_MEMORY_LIMIT_MB);

        let num_chunks = if phase1_total_mb <= available_mb {
            1
        } else {
            phase1_total_mb.div_ceil(available_mb).max(2)
        };

        let reads_per_chunk = total_reads.div_ceil(num_chunks);
        let blocks_per_chunk = reads_per_chunk.div_ceil(block_size);

        let chunk_phase1_mb = (reads_per_chunk * phase1_per_read) / (1024 * 1024);
        let phase2_per_block_mb = (block_size * MEMORY_PER_READ_PHASE2) / (1024 * 1024);
        let phase2_mb = phase2_per_block_mb * num_threads.min(blocks_per_chunk);
        let estimated_peak_mb =
            chunk_phase1_mb.max(phase2_mb) + DEFAULT_BLOCK_BUFFER_MB + DEFAULT_WORKER_STACK_MB * num_threads;

        Self {
            num_chunks,
            reads_per_chunk,
            block_size,
            blocks_per_chunk,
            estimated_peak_mb,
        }
    }

    pub fn requires_chunking(&self) -> bool {
        self.num_chunks > 1
    }

    /// Summary string for logging
    pub fn summary(&self) -> String {
        if self.requires_chunking() {
            format!(
                "{} chunks × {} reads/chunk ({} blocks/chunk), est. peak {} MB",
                self.num_chunks, self.reads_per_chunk, self.blocks_per_chunk, self.estimated_peak_mb
            )
        } else {
            format!(
                "single pass, {} reads, est. peak {} MB",
                self.reads_per_chunk, self.estimated_peak_mb
            )
        }
    }
}

/// Auto-configure memory budget from system state
pub fn auto_memory_budget(user_limit_mb: usize) -> MemoryBudget {
    let limit = if user_limit_mb == 0 {
        let avail = get_available_memory_mb();
        (avail as f64 * 0.75) as usize
    } else {
        user_limit_mb
    };
    MemoryBudget::from_memory_limit(limit)
}
