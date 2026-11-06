// =============================================================================
// fqc - Read length sampling and classification
// =============================================================================
//! Shared length-detection policy for archive and pipeline compression.
//!
//! Both modes classify the same way: either scan every ingested read or take
//! evenly spaced samples across the whole input. Sampling a fixed prefix (as
//! the pipeline used to do before ingest) misclassifies inputs whose long
//! reads appear only in the tail.

use crate::types::{classify_read_length, LengthStats, ReadLengthClass, ReadRecord};

pub const DEFAULT_LENGTH_SAMPLE_READS: usize = 4_096;

/// Compute length statistics from ingested records, evenly sampled by default.
pub fn length_stats_from_records_sampled(records: &[ReadRecord], scan_all: bool) -> LengthStats {
    if scan_all || records.len() <= DEFAULT_LENGTH_SAMPLE_READS {
        let lengths: Vec<usize> = records.iter().map(|read| read.sequence.len()).collect();
        return LengthStats::from_lengths(&lengths);
    }

    let sample_size = DEFAULT_LENGTH_SAMPLE_READS.min(records.len());
    let lengths: Vec<usize> = (0..sample_size)
        .map(|i| {
            let idx = i * records.len() / sample_size;
            records[idx].sequence.len()
        })
        .collect();
    LengthStats::from_lengths(&lengths)
}

/// Honor an explicit class; otherwise classify from the sampled statistics.
pub fn effective_read_length_class(requested: Option<ReadLengthClass>, stats: &LengthStats) -> ReadLengthClass {
    requested.unwrap_or_else(|| classify_read_length(stats.median_length, stats.max_length))
}
