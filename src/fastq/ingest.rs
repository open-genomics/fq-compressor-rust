// =============================================================================
// fqc - Full FASTQ ingestion
// =============================================================================
//! Read every record under the archive memory budget.
//!
//! Archive and pipeline both need the complete read set for global analysis;
//! keeping the three topology branches here stops the two runners from
//! maintaining separate stdin/paired/interleaved ingestion paths.

use std::path::Path;

use crate::error::Result;
use crate::types::{PeLayout, ReadRecord};

use super::parser::{open_fastq, open_fastq_interleaved, open_fastq_paired, open_fastq_stdin};

pub(crate) fn read_all_records(input_path: &Path, memory_limit_mb: usize) -> Result<Vec<ReadRecord>> {
    if input_path == Path::new("-") {
        open_fastq_stdin().collect_all_within_archive_budget(memory_limit_mb)
    } else {
        open_fastq(input_path)?.collect_all_within_archive_budget(memory_limit_mb)
    }
}

pub(crate) fn read_all_paired_records(
    input_path: &Path,
    input2_path: &Path,
    pe_layout: PeLayout,
    memory_limit_mb: usize,
) -> Result<Vec<ReadRecord>> {
    open_fastq_paired(input_path, input2_path)?.collect_pairs_within_archive_budget(pe_layout, memory_limit_mb)
}

pub(crate) fn read_all_interleaved_records(
    input_path: &Path,
    pe_layout: PeLayout,
    memory_limit_mb: usize,
) -> Result<Vec<ReadRecord>> {
    let mut parser = if input_path == Path::new("-") {
        crate::fastq::parser::InterleavedPeParser::new(open_fastq_stdin())
    } else {
        open_fastq_interleaved(input_path)?
    };
    parser.collect_pairs_within_archive_budget(pe_layout, memory_limit_mb)
}
