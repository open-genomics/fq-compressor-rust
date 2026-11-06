// =============================================================================
// fqc-rust - Decompression writer stage
// =============================================================================
// Owns everything after block decoding: ordered reassembly, range filtering,
// original-order restoration, skip-corrupted placeholders, split R1/R2 routing
// and transactional commit. The scheduler only feeds decoded block results.

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use crossbeam_channel::Receiver;

use crate::archive::format::{flags as format_flags, get_pe_layout};
use crate::block::DecodedBlock;
use crate::error::{FqcError, Result};
use crate::fastq::parser::write_record as write_fastq_record;
use crate::io::async_io::AsyncWriter;
use crate::io::{commit_split, OutputTransaction};
use crate::types::{PeLayout, ReadRecord};

// =============================================================================
// Writer inputs and outputs
// =============================================================================

/// One decoded block or the error that prevented decoding it.
pub(crate) struct BlockResult {
    pub block_id: u32,
    pub result: std::result::Result<DecodedBlock, FqcError>,
    pub expected_read_count: u32,
}

/// Immutable writer-stage plan prepared by the decompression scheduler.
pub(crate) struct WriterPlan {
    pub output_path: PathBuf,
    pub force_overwrite: bool,
    pub split_pe: bool,
    pub archive_flags: u64,
    pub original_order: bool,
    pub header_only: bool,
    pub skip_corrupted: bool,
    pub corrupted_placeholder: String,
    pub range_start: u64,
    pub range_end: u64,
    pub has_range: bool,
    pub start_block: usize,
    pub end_block: usize,
    pub reads_before_start_block: u64,
    pub total_archive_reads: u64,
    pub forward_map: Option<Vec<crate::types::ReadId>>,
}

pub(crate) struct WriterStats {
    pub reads_written: u64,
    pub output_bytes: u64,
    pub total_bases: u64,
    pub corrupted_blocks: u32,
    pub write_ms: u64,
}

pub(crate) fn spawn_writer_thread(
    plan: WriterPlan,
    result_rx: Receiver<BlockResult>,
) -> thread::JoinHandle<Result<WriterStats>> {
    thread::spawn(move || write_blocks(&plan, &result_rx))
}

// =============================================================================
// Ordered output assembly
// =============================================================================

/// 1-based read-range check shared by archive-order and original-order output.
fn in_range(index: u64, has_range: bool, start: u64, end: u64) -> bool {
    if !has_range {
        return true;
    }
    if start > 0 && index < start {
        return false;
    }
    if end > 0 && index > end {
        return false;
    }
    true
}

struct WriteProgress {
    total_output_bytes: u64,
    total_reads_written: u64,
    total_bases: u64,
    corrupted_blocks: u32,
    global_read_idx: u64,
}

impl WriteProgress {
    fn new(plan: &WriterPlan) -> Self {
        Self {
            total_output_bytes: 0,
            total_reads_written: 0,
            total_bases: 0,
            corrupted_blocks: 0,
            global_read_idx: plan.reads_before_start_block,
        }
    }

    fn write_read(
        &mut self,
        output: &mut OutputWriters,
        plan: &WriterPlan,
        read: &ReadRecord,
        archive_read_idx: u64,
        block_local_idx: Option<(u64, u64)>,
    ) -> Result<()> {
        self.total_output_bytes += output.write_record(
            read,
            plan.header_only,
            archive_read_idx,
            plan.total_archive_reads,
            block_local_idx,
        )?;
        self.total_reads_written += 1;
        self.total_bases += read.sequence.len() as u64;
        Ok(())
    }

    fn write_decoded_block(
        &mut self,
        output: &mut OutputWriters,
        plan: &WriterPlan,
        decoded: &DecodedBlock,
    ) -> Result<()> {
        let block_read_count = decoded.reads.len() as u64;
        for (local_idx, read) in decoded.reads.iter().enumerate() {
            self.global_read_idx += 1;
            if !in_range(self.global_read_idx, plan.has_range, plan.range_start, plan.range_end) {
                continue;
            }
            self.write_read(
                output,
                plan,
                read,
                self.global_read_idx - 1,
                Some((local_idx as u64, block_read_count)),
            )?;
        }
        Ok(())
    }

    fn write_corrupted_block(
        &mut self,
        output: &mut OutputWriters,
        plan: &WriterPlan,
        block_id: u32,
        expected_read_count: u32,
    ) -> Result<()> {
        for read_idx in 0..expected_read_count as usize {
            self.global_read_idx += 1;
            if !in_range(self.global_read_idx, plan.has_range, plan.range_start, plan.range_end) {
                continue;
            }
            let placeholder = placeholder_record(&plan.corrupted_placeholder, block_id, read_idx);
            self.write_read(
                output,
                plan,
                &placeholder,
                self.global_read_idx - 1,
                Some((read_idx as u64, expected_read_count as u64)),
            )?;
        }
        Ok(())
    }
}

fn write_blocks(plan: &WriterPlan, result_rx: &Receiver<BlockResult>) -> Result<WriterStats> {
    let started = Instant::now();
    let mut output = OutputWriters::open(
        &plan.output_path,
        plan.force_overwrite,
        plan.split_pe,
        plan.archive_flags,
    )?;
    let mut pending: BTreeMap<u32, BlockResult> = BTreeMap::new();
    let mut next_expected = plan.start_block as u32;
    let end_block_id = plan.end_block as u32;
    let mut progress = WriteProgress::new(plan);
    let mut original_reads: Vec<ReadRecord> = if plan.original_order {
        Vec::with_capacity(plan.total_archive_reads as usize)
    } else {
        Vec::new()
    };

    for block in result_rx.iter() {
        pending.insert(block.block_id, block);

        while let Some(block) = pending.remove(&next_expected) {
            let block_id = block.block_id;
            match block.result {
                Ok(decoded) => {
                    if plan.original_order {
                        original_reads.extend(decoded.reads);
                    } else {
                        progress.write_decoded_block(&mut output, plan, &decoded)?;
                    }
                }
                Err(e) => {
                    if !plan.skip_corrupted {
                        return Err(e);
                    }
                    if plan.original_order {
                        for read_idx in 0..block.expected_read_count as usize {
                            original_reads.push(placeholder_record(&plan.corrupted_placeholder, block_id, read_idx));
                        }
                    } else {
                        progress.write_corrupted_block(&mut output, plan, block_id, block.expected_read_count)?;
                    }
                    progress.corrupted_blocks += 1;
                    log::warn!("Block {} corrupted, skipping: {}", block_id, e);
                }
            }
            next_expected += 1;
        }
    }

    // A worker error closes the channel early. Never commit a partial output:
    // dropping the writers aborts their output transactions.
    if !pending.is_empty() || next_expected < end_block_id {
        return Err(FqcError::Decompression(format!(
            "Decompression incomplete: processed {} of blocks up to {}; output aborted",
            next_expected - plan.start_block as u32,
            end_block_id
        )));
    }

    if plan.original_order {
        write_original_order(plan, &mut output, &original_reads, &mut progress)?;
    }

    output.commit()?;
    Ok(WriterStats {
        reads_written: progress.total_reads_written,
        output_bytes: progress.total_output_bytes,
        total_bases: progress.total_bases,
        corrupted_blocks: progress.corrupted_blocks,
        write_ms: started.elapsed().as_millis() as u64,
    })
}

fn write_original_order(
    plan: &WriterPlan,
    output: &mut OutputWriters,
    original_reads: &[ReadRecord],
    progress: &mut WriteProgress,
) -> Result<()> {
    let forward = plan
        .forward_map
        .as_ref()
        .ok_or_else(|| FqcError::Format("Forward reorder map missing".to_string()))?;
    if forward.len() != original_reads.len() || original_reads.len() as u64 != plan.total_archive_reads {
        return Err(FqcError::Format(format!(
            "Original-order buffer holds {} reads, forward map has {}, archive declares {}",
            original_reads.len(),
            forward.len(),
            plan.total_archive_reads
        )));
    }

    for (original_id, &archive_id) in forward.iter().enumerate() {
        let read = original_reads.get(archive_id as usize).ok_or_else(|| {
            FqcError::Format(format!(
                "Forward reorder map references archive id {archive_id}, but only {} reads were decoded",
                original_reads.len()
            ))
        })?;
        let current_id = original_id as u64 + 1;
        if !in_range(current_id, plan.has_range, plan.range_start, plan.range_end) {
            continue;
        }
        progress.write_read(output, plan, read, original_id as u64, None)?;
    }
    Ok(())
}

// =============================================================================
// Output destinations
// =============================================================================

pub(crate) enum OutputWriters {
    Single {
        writer: Box<dyn Write>,
        tx: Option<OutputTransaction>,
    },
    Split {
        r1: Box<dyn Write>,
        r2: Box<dyn Write>,
        r1_tx: OutputTransaction,
        r2_tx: OutputTransaction,
        pe_layout: PeLayout,
        streaming_block_layout: bool,
    },
}

impl OutputWriters {
    fn open(output_path: &Path, force_overwrite: bool, split_pe: bool, archive_flags: u64) -> Result<Self> {
        const ASYNC_WRITE_BUF: usize = 4 * 1024 * 1024;
        const ASYNC_WRITE_DEPTH: usize = 4;

        if split_pe {
            return Self::split(output_path, force_overwrite, archive_flags);
        }
        if output_path == Path::new("-") {
            return Ok(Self::single_raw(Box::new(BufWriter::new(std::io::stdout())), None));
        }
        let mut tx = OutputTransaction::begin(output_path, force_overwrite)?;
        let file = tx.take_file()?;
        Ok(Self::single_raw(
            Box::new(AsyncWriter::new(file, ASYNC_WRITE_DEPTH, ASYNC_WRITE_BUF)),
            Some(tx),
        ))
    }

    /// Open split R1/R2 destinations derived from `output_path`.
    fn split(output_path: &Path, force_overwrite: bool, archive_flags: u64) -> Result<Self> {
        let is_paired = (archive_flags & format_flags::IS_PAIRED) != 0;
        if !is_paired {
            return Err(FqcError::InvalidArgument(
                "--split-pe requires a paired-end archive".to_string(),
            ));
        }
        if output_path == Path::new("-") {
            return Err(FqcError::InvalidArgument(
                "--split-pe cannot be used with stdout output".to_string(),
            ));
        }

        let (r1_path, r2_path) = derive_split_output_paths(output_path);
        let mut r1_tx = OutputTransaction::begin(r1_path, force_overwrite)?;
        let mut r2_tx = OutputTransaction::begin(r2_path, force_overwrite)?;
        let pe_layout = get_pe_layout(archive_flags);
        Ok(Self::Split {
            r1: Box::new(BufWriter::new(r1_tx.take_file()?)),
            r2: Box::new(BufWriter::new(r2_tx.take_file()?)),
            r1_tx,
            r2_tx,
            pe_layout,
            streaming_block_layout: (archive_flags & format_flags::STREAMING_MODE) != 0
                && pe_layout == PeLayout::Consecutive,
        })
    }

    fn single_raw(writer: Box<dyn Write>, tx: Option<OutputTransaction>) -> Self {
        Self::Single { writer, tx }
    }

    fn write_record(
        &mut self,
        read: &ReadRecord,
        header_only: bool,
        zero_based_read_idx: u64,
        total_archive_reads: u64,
        block_local_idx: Option<(u64, u64)>,
    ) -> Result<u64> {
        match self {
            Self::Single { writer, .. } => write_to_target(writer.as_mut(), read, header_only),
            Self::Split {
                r1,
                r2,
                pe_layout,
                streaming_block_layout,
                ..
            } => {
                let to_r1 = match (pe_layout, *streaming_block_layout, block_local_idx) {
                    (PeLayout::Consecutive, true, Some((local_idx, block_reads))) => local_idx < (block_reads / 2),
                    (PeLayout::Interleaved, _, _) => zero_based_read_idx % 2 == 0,
                    (PeLayout::Consecutive, _, _) => zero_based_read_idx < (total_archive_reads / 2),
                };

                if to_r1 {
                    write_to_target(r1.as_mut(), read, header_only)
                } else {
                    write_to_target(r2.as_mut(), read, header_only)
                }
            }
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            Self::Single { writer, .. } => writer.flush().map_err(FqcError::Io),
            Self::Split { r1, r2, .. } => {
                r1.flush().map_err(FqcError::Io)?;
                r2.flush().map_err(FqcError::Io)
            }
        }
    }

    /// Flush, drop writers, then rename temps onto final paths.
    fn commit(mut self) -> Result<()> {
        self.flush()?;
        match self {
            Self::Single { writer, tx } => {
                drop(writer);
                if let Some(tx) = tx {
                    tx.commit()?;
                }
                Ok(())
            }
            Self::Split {
                r1, r2, r1_tx, r2_tx, ..
            } => {
                drop(r1);
                drop(r2);
                // POSIX cannot atomically rename two paths; R1 then R2.
                commit_split(r1_tx, r2_tx)
            }
        }
    }
}

pub(crate) fn placeholder_record(placeholder_seq: &str, block_id: u32, read_idx: usize) -> ReadRecord {
    ReadRecord {
        id: format!("corrupted_block{block_id}_read{read_idx}"),
        comment: String::new(),
        sequence: placeholder_seq.to_string(),
        quality: "!".repeat(placeholder_seq.len()),
    }
}

fn write_to_target(output: &mut dyn Write, read: &ReadRecord, header_only: bool) -> Result<u64> {
    if header_only {
        if read.comment.is_empty() {
            writeln!(output, "@{}", read.id)?;
            Ok((read.id.len() + 2) as u64)
        } else {
            writeln!(output, "@{} {}", read.id, read.comment)?;
            Ok((read.id.len() + 1 + read.comment.len() + 2) as u64)
        }
    } else {
        write_fastq_record(output, read)?;
        let comment_bytes = if read.comment.is_empty() {
            0
        } else {
            read.comment.len() + 1
        };
        // FASTQ layout: '@' + id + (space + comment) + '\n' + seq + '\n+\n' + qual + '\n'
        Ok(read.id.len() as u64 + comment_bytes as u64 + read.sequence.len() as u64 + read.quality.len() as u64 + 6)
    }
}

fn derive_split_output_paths(output_path: &Path) -> (PathBuf, PathBuf) {
    let stem = output_path
        .file_stem()
        .map(|stem| stem.to_os_string())
        .unwrap_or_default();
    let extension = output_path.extension().map(|ext| ext.to_os_string());
    let split_name = |suffix: &str| {
        let mut name = stem.clone();
        name.push(suffix);
        if let Some(extension) = &extension {
            name.push(".");
            name.push(extension);
        }
        output_path.with_file_name(name)
    };
    (split_name("_R1"), split_name("_R2"))
}

#[cfg(test)]
mod tests {
    use super::derive_split_output_paths;
    use std::path::Path;

    #[cfg(unix)]
    #[test]
    fn split_paths_preserve_non_utf8_components() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let output = Path::new("/tmp/sample").join(OsString::from_vec(b"out\xff.fastq".to_vec()));
        let (r1, r2) = derive_split_output_paths(&output);
        assert_eq!(r1.parent(), output.parent());
        assert_eq!(r2.parent(), output.parent());
        assert_eq!(r1.extension().unwrap(), "fastq");
        assert_eq!(r2.extension().unwrap(), "fastq");
        assert!(r1.to_string_lossy().contains("out�_R1.fastq"));
        assert!(r2.to_string_lossy().contains("out�_R2.fastq"));
    }

    #[test]
    fn split_paths_stay_relative_and_keep_extension() {
        let (r1, r2) = derive_split_output_paths(Path::new("out.fastq"));
        assert_eq!(r1, Path::new("out_R1.fastq"));
        assert_eq!(r2, Path::new("out_R2.fastq"));

        let (r1, r2) = derive_split_output_paths(Path::new("out"));
        assert_eq!(r1, Path::new("out_R1"));
        assert_eq!(r2, Path::new("out_R2"));
    }
}
