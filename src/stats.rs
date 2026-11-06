// =============================================================================
// fqc - Operation statistics
// =============================================================================
//! Shared counters and timing for compression, decompression and verification.
//!
//! `input_bytes` / `output_bytes` describe the actual byte streams entering and
//! leaving the operation. `total_bases` is tracked separately; it must never be
//! used as a substitute for `input_bytes`.

use std::path::Path;

/// Statistics collected by one compression/decompression/verify operation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OperationStats {
    /// Reads processed.
    pub total_reads: u64,
    /// Bases processed (sequence length, not input file bytes).
    pub total_bases: u64,
    /// Actual input bytes consumed (compressed bytes for compressed inputs).
    pub input_bytes: u64,
    /// Actual output bytes written (archive or FASTQ file size).
    pub output_bytes: u64,
    /// Blocks written or processed.
    pub blocks: u64,
    /// Wall-clock elapsed time.
    pub elapsed_seconds: f64,
    /// Serial parse stage time.
    pub parse_ms: u64,
    /// Reorder/global-analysis stage time.
    pub reorder_ms: u64,
    /// Aggregate worker compression/decompression time.
    pub process_ms: u64,
    /// Serial write stage time.
    pub write_ms: u64,
    /// Blocks replaced by placeholders under `--skip-corrupted`.
    pub corrupted_blocks: u32,
    /// Whether a reorder map was written.
    pub reorder_map_written: bool,
}

impl OperationStats {
    /// Input bytes divided by output bytes. Values above 1 mean compression.
    pub fn compression_ratio(&self) -> f64 {
        if self.output_bytes == 0 {
            return 0.0;
        }
        self.input_bytes as f64 / self.output_bytes as f64
    }

    /// Output bytes divided by input bytes. Useful for reporting a fraction.
    pub fn compressed_fraction(&self) -> f64 {
        if self.input_bytes == 0 {
            return 1.0;
        }
        self.output_bytes as f64 / self.input_bytes as f64
    }

    /// Output bits per input base.
    pub fn bits_per_base(&self) -> f64 {
        if self.total_bases == 0 {
            return 0.0;
        }
        (self.output_bytes as f64 * 8.0) / self.total_bases as f64
    }

    /// Input MiB per wall-clock second.
    pub fn throughput_mbps(&self) -> f64 {
        if self.elapsed_seconds == 0.0 {
            return 0.0;
        }
        (self.input_bytes as f64 / (1024.0 * 1024.0)) / self.elapsed_seconds
    }
}

/// Actual size of a regular input path.
///
/// Stdin (`-`) and other non-regular paths return `0`; callers may use parser
/// byte counters as a fallback for those cases.
pub fn path_bytes(path: &Path) -> u64 {
    if path.as_os_str() == "-" {
        return 0;
    }
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

/// Sum actual file sizes for `paths`, falling back to a parser-provided byte
/// estimate for stdin or other non-regular inputs.
pub fn input_bytes_for_paths(paths: &[&Path], fallback: u64) -> u64 {
    let total: u64 = paths.iter().map(|path| path_bytes(path)).sum();
    if total == 0 {
        fallback
    } else {
        total
    }
}
