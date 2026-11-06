// =============================================================================
// fqc-rust - Shared decompression output writers
// =============================================================================
// Single file/stdout and split R1/R2 destinations, transactional commit and
// FASTQ/header-only record formatting used by the decompression pipeline.

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::archive::format::{flags as format_flags, get_pe_layout};
use crate::error::{FqcError, Result};
use crate::fastq::parser::write_record as write_fastq_record;
use crate::io::{commit_split, OutputTransaction};
use crate::types::{PeLayout, ReadRecord};

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
    /// Open one file/stdout or split R1/R2 files derived from `output_path`.
    pub(crate) fn buffered(
        output_path: &Path,
        force_overwrite: bool,
        split_pe: bool,
        archive_flags: u64,
    ) -> Result<Self> {
        let is_paired = (archive_flags & format_flags::IS_PAIRED) != 0;
        if split_pe {
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
        } else if output_path == Path::new("-") {
            Ok(Self::Single {
                writer: Box::new(std::io::stdout()),
                tx: None,
            })
        } else {
            let mut tx = OutputTransaction::begin(output_path, force_overwrite)?;
            Ok(Self::Single {
                writer: Box::new(BufWriter::new(tx.take_file()?)),
                tx: Some(tx),
            })
        }
    }

    /// Wrap an already-opened single destination (the pipeline uses an
    /// `AsyncWriter` here; split outputs use buffered writers).
    pub(crate) fn single_raw(writer: Box<dyn Write>, tx: Option<OutputTransaction>) -> Self {
        Self::Single { writer, tx }
    }

    pub(crate) fn write_record(
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

    pub(crate) fn flush(&mut self) -> Result<()> {
        match self {
            Self::Single { writer, .. } => writer.flush().map_err(FqcError::Io),
            Self::Split { r1, r2, .. } => {
                r1.flush().map_err(FqcError::Io)?;
                r2.flush().map_err(FqcError::Io)
            }
        }
    }

    /// Flush, drop writers, then rename temps onto final paths.
    pub(crate) fn commit(mut self) -> Result<()> {
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
