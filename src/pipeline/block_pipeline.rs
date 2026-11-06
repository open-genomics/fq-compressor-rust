// =============================================================================
// fqc - Shared bounded block pipeline
// =============================================================================
//! Reader -> parallel compressor -> ordered writer.
//!
//! The caller supplies a block source: archive/pipeline feed pre-chunked reads,
//! while streaming pulls blocks from a parser on demand. This module owns the
//! bounded channels, worker lifecycle, ordered block writes, transactional
//! output and the error propagation contract shared by all compression modes.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use crate::archive::writer::{begin_fqc_writer, write_global_header};
use crate::block::EncodedBlock;
use crate::error::{FqcError, Result};
use crate::io::OutputTransaction;
use crate::types::{ReadId, ReadRecord};

/// Everything the shared block pipeline needs after parsing and analysis.
pub(crate) struct BlockPipelineConfig {
    pub num_workers: usize,
    pub max_in_flight_blocks: usize,
    pub force_overwrite: bool,
    pub flags: u64,
    /// Known total for archive/pipeline; `None` patches it after streaming.
    pub total_reads: Option<u64>,
    /// Known block count for archive/pipeline; `None` accepts source length.
    pub expected_blocks: Option<u32>,
    pub original_filename: String,
    pub reorder_map: Option<(Vec<ReadId>, Vec<ReadId>)>,
    pub compressor_config: BlockCompressorConfig,
}

/// One source block plus the bytes the source consumed to produce it.
///
/// Archive/pipeline sources pass `0` because their caller accounts for file
/// bytes from paths. Streaming sources report parser bytes so stdin stats have
/// a real byte count instead of falling back to sequence bases.
pub(crate) struct ReadBlock {
    pub reads: Vec<ReadRecord>,
    pub input_bytes: u64,
}

/// Counters produced by the shared block pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlockPipelineReport {
    pub output_bytes: u64,
    pub blocks_written: u64,
    pub total_reads: u64,
    pub total_bases: u64,
    /// Bytes reported by the source; `0` for pre-chunked archive/pipeline input.
    pub input_bytes: u64,
    pub parse_ms: u64,
    pub process_ms: u64,
    pub write_ms: u64,
}

struct ReadChunk {
    reads: Vec<ReadRecord>,
    chunk_id: u32,
}

struct OrderedBlock {
    chunk_id: u32,
    compressed: EncodedBlock,
}

struct ReaderOutcome {
    total_reads: u64,
    total_bases: u64,
    input_bytes: u64,
    parse_ns: u64,
}

struct WriterOutcome {
    output_bytes: u64,
    blocks_written: u32,
    total_reads: u64,
    write_ns: u64,
    /// Kept uncommitted until every stage has reported success.
    output_tx: OutputTransaction,
}

/// Result of one pipeline stage.
///
/// `Root` is a failure of the stage's own work. `Derived` only reports that a
/// peer stopped; without this distinction the first thread to observe a closed
/// channel can hide the writer or compressor error that caused it.
enum StageFailure {
    Root(FqcError),
    Derived(String),
    Panicked(&'static str),
}

impl StageFailure {
    /// Lower rank wins. Stable sorting keeps writer failures ahead of equally
    /// ranked worker failures so the reported cause is deterministic.
    fn rank(&self) -> u8 {
        match self {
            Self::Root(_) => 0,
            Self::Panicked(_) => 1,
            Self::Derived(_) => 2,
        }
    }

    fn into_error(self) -> FqcError {
        match self {
            Self::Root(error) => error,
            Self::Derived(message) => FqcError::Compression(message),
            Self::Panicked(stage) => FqcError::Compression(format!("{stage} thread panicked")),
        }
    }
}

impl From<FqcError> for StageFailure {
    fn from(error: FqcError) -> Self {
        Self::Root(error)
    }
}

type StageResult<T> = std::result::Result<T, StageFailure>;

fn join_stage<T>(handle: thread::JoinHandle<StageResult<T>>, stage: &'static str) -> StageResult<T> {
    handle.join().unwrap_or_else(|_| Err(StageFailure::Panicked(stage)))
}

/// Pull source blocks and forward them to the compressor channel.
///
/// The source owns parsing, so this stage only accounts for reads, bases and
/// source-reported input bytes before handing each block over.
fn spawn_reader(
    mut next_chunk: impl FnMut() -> Result<Option<ReadBlock>> + Send + 'static,
    chunk_tx: Sender<ReadChunk>,
) -> thread::JoinHandle<StageResult<ReaderOutcome>> {
    thread::spawn(move || -> StageResult<ReaderOutcome> {
        let mut chunk_id: u32 = 0;
        let mut total_reads = 0u64;
        let mut total_bases = 0u64;
        let mut input_bytes = 0u64;
        let mut parse_ns = 0u64;
        loop {
            let t_parse = Instant::now();
            let next = next_chunk()?;
            parse_ns += t_parse.elapsed().as_nanos() as u64;
            let Some(block) = next else {
                break;
            };
            if block.reads.is_empty() {
                continue;
            }
            total_reads = total_reads.saturating_add(block.reads.len() as u64);
            total_bases = total_bases.saturating_add(block.reads.iter().map(|r| r.sequence.len() as u64).sum::<u64>());
            input_bytes = input_bytes.saturating_add(block.input_bytes);
            chunk_tx
                .send(ReadChunk {
                    reads: block.reads,
                    chunk_id,
                })
                .map_err(|_| {
                    StageFailure::Derived(
                        "Compression aborted: compressor stage stopped before reading all blocks".to_string(),
                    )
                })?;
            chunk_id = chunk_id
                .checked_add(1)
                .ok_or_else(|| StageFailure::Root(FqcError::Compression("block count exceeds u32".to_string())))?;
        }
        Ok(ReaderOutcome {
            total_reads,
            total_bases,
            input_bytes,
            parse_ns,
        })
    })
}

/// Compress blocks from `next_chunk` in parallel and write them in block order.
///
/// The source is pulled on the reader thread; `None` ends the stream. A worker
/// or writer error aborts the transaction and never commits a partial archive.
pub(crate) fn run(
    next_chunk: impl FnMut() -> Result<Option<ReadBlock>> + Send + 'static,
    output_path: impl AsRef<Path>,
    config: BlockPipelineConfig,
) -> Result<BlockPipelineReport> {
    let BlockPipelineConfig {
        num_workers,
        max_in_flight_blocks,
        force_overwrite,
        flags,
        total_reads,
        expected_blocks,
        original_filename,
        reorder_map,
        compressor_config,
    } = config;

    // Archive and pipeline know their block count up front; an empty input is a
    // caller error there. Streaming passes `None` and may legitimately write an
    // empty archive.
    if expected_blocks == Some(0) {
        return Err(FqcError::InvalidArgument("no blocks to write".to_string()));
    }

    let max_inflight = max_in_flight_blocks.max(1);
    let num_workers = expected_blocks
        .map(|expected| num_workers.min(expected as usize))
        .unwrap_or(num_workers)
        .max(1);

    // Open the output before pulling any input so an unwritable path fails
    // fast, even when the source is an interactive stdin that may never reach
    // EOF on its own.
    let t_setup = Instant::now();
    let (writer, output_tx) = begin_fqc_writer(output_path.as_ref(), force_overwrite)?;
    let setup_ns = t_setup.elapsed().as_nanos() as u64;

    let compressor_config = Arc::new(compressor_config);
    let (chunk_tx, chunk_rx): (Sender<ReadChunk>, Receiver<ReadChunk>) = bounded(max_inflight);
    let (block_tx, block_rx): (Sender<OrderedBlock>, Receiver<OrderedBlock>) = bounded(max_inflight);

    // ---- Reader thread: pull blocks from the caller's bounded source ----
    let reader_handle = spawn_reader(next_chunk, chunk_tx);

    // ---- Compressor threads ----
    let process_ns = Arc::new(AtomicU64::new(0));
    let mut compressor_handles = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        let rx = chunk_rx.clone();
        let tx = block_tx.clone();
        let cfg = Arc::clone(&compressor_config);
        let process_ns = Arc::clone(&process_ns);

        let handle = thread::spawn(move || -> StageResult<()> {
            let mut compressor = BlockCompressor::new((*cfg).clone());
            for chunk in rx.iter() {
                let t = Instant::now();
                let compressed = compressor.compress(&chunk.reads, chunk.chunk_id)?;
                process_ns.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);

                tx.send(OrderedBlock {
                    chunk_id: chunk.chunk_id,
                    compressed,
                })
                .map_err(|_| {
                    StageFailure::Derived(
                        "Compression aborted: writer stage stopped before receiving all blocks".to_string(),
                    )
                })?;
            }
            Ok(())
        });
        compressor_handles.push(handle);
    }

    drop(chunk_rx);
    drop(block_tx);

    // ---- Writer thread: reorder completed blocks and commit atomically ----
    let writer_handle = thread::spawn(move || -> StageResult<WriterOutcome> {
        let mut writer = writer;
        let t_header = Instant::now();
        write_global_header(&mut writer, flags, total_reads.unwrap_or(0), &original_filename)?;
        let mut write_ns = t_header.elapsed().as_nanos() as u64;

        let mut pending: BTreeMap<u32, OrderedBlock> = BTreeMap::new();
        let mut next_expected: u32 = 0;
        let mut blocks_received: u32 = 0;
        let mut reads_written = 0u64;

        for ordered_block in block_rx.iter() {
            blocks_received = blocks_received
                .checked_add(1)
                .ok_or_else(|| StageFailure::Root(FqcError::Compression("block count exceeds u32".to_string())))?;
            pending.insert(ordered_block.chunk_id, ordered_block);
            while let Some(block) = pending.remove(&next_expected) {
                let t = Instant::now();
                writer.write_block(&block.compressed)?;
                write_ns += t.elapsed().as_nanos() as u64;
                reads_written = reads_written.saturating_add(block.compressed.meta.read_count as u64);
                next_expected = next_expected
                    .checked_add(1)
                    .ok_or_else(|| StageFailure::Root(FqcError::Compression("block count exceeds u32".to_string())))?;
            }
        }

        let (complete, expected) = match expected_blocks {
            Some(expected) => (
                next_expected == expected && blocks_received == expected && pending.is_empty(),
                expected,
            ),
            None => (next_expected == blocks_received && pending.is_empty(), blocks_received),
        };
        if !complete {
            return Err(StageFailure::Derived(format!(
                "Compression incomplete: wrote {}/{} blocks; output aborted",
                next_expected, expected
            )));
        }

        if total_reads.is_none() {
            let t = Instant::now();
            writer.patch_total_read_count(reads_written)?;
            write_ns += t.elapsed().as_nanos() as u64;
        }

        if let Some((forward, reverse)) = reorder_map.as_ref() {
            let t = Instant::now();
            writer.write_reorder_map(forward, reverse)?;
            write_ns += t.elapsed().as_nanos() as u64;
        }

        let t = Instant::now();
        let output_bytes = writer.finalize()?;
        write_ns += t.elapsed().as_nanos() as u64;
        Ok(WriterOutcome {
            output_bytes,
            blocks_written: next_expected,
            total_reads: reads_written,
            write_ns,
            output_tx,
        })
    });

    // Join every stage before returning so no detached worker can keep writing
    // after the caller observes an error. Root failures outrank panics and
    // channel-closed symptoms; see `StageFailure::rank`.
    let mut failures = Vec::new();
    let writer_outcome = match join_stage(writer_handle, "writer") {
        Ok(outcome) => Some(outcome),
        Err(failure) => {
            failures.push(failure);
            None
        }
    };
    for handle in compressor_handles {
        if let Err(failure) = join_stage(handle, "compressor") {
            failures.push(failure);
        }
    }
    let reader_outcome = match join_stage(reader_handle, "reader") {
        Ok(outcome) => Some(outcome),
        Err(failure) => {
            failures.push(failure);
            None
        }
    };

    let process_ms = process_ns.load(Ordering::Relaxed) / 1_000_000;

    failures.sort_by_key(StageFailure::rank);
    if let Some(failure) = failures.into_iter().next() {
        return Err(failure.into_error());
    }

    let writer = writer_outcome
        .ok_or_else(|| FqcError::Compression("Compression aborted without a writer result".to_string()))?;
    let reader = reader_outcome
        .ok_or_else(|| FqcError::Compression("Compression aborted without a reader result".to_string()))?;

    // The reader counted what the source handed over; the writer counted what
    // each compressed block declares. They must agree, otherwise a block lost
    // reads between parsing and archiving.
    if reader.total_reads != writer.total_reads {
        return Err(FqcError::Compression(format!(
            "Compression integrity check failed: reader produced {} reads, writer stored {}",
            reader.total_reads, writer.total_reads
        )));
    }

    // Only publish after every stage is known good. Keeping the commit off the
    // writer thread also means a late reader/compressor failure cannot leave a
    // committed partial archive behind.
    let WriterOutcome {
        output_bytes,
        blocks_written,
        total_reads: stored_reads,
        write_ns,
        output_tx,
    } = writer;
    let t_commit = Instant::now();
    output_tx.commit()?;
    let commit_ns = t_commit.elapsed().as_nanos() as u64;

    Ok(BlockPipelineReport {
        output_bytes,
        blocks_written: blocks_written as u64,
        total_reads: stored_reads,
        total_bases: reader.total_bases,
        input_bytes: reader.input_bytes,
        parse_ms: reader.parse_ns / 1_000_000,
        process_ms,
        write_ms: (setup_ns + write_ns + commit_ns) / 1_000_000,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algo::block_compressor::BlockCompressorConfig;
    use crate::archive::format::build_flags;
    use crate::types::{IdMode, PeLayout, QualityMode, ReadLengthClass};

    #[test]
    fn report_aggregates_source_input_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("out.fqc");
        let mut blocks = vec![
            Some(ReadBlock {
                reads: vec![ReadRecord::new("a".into(), "ACGT".into(), "IIII".into())],
                input_bytes: 11,
            }),
            Some(ReadBlock {
                reads: vec![ReadRecord::new("b".into(), "ACGT".into(), "IIII".into())],
                input_bytes: 22,
            }),
            None,
        ]
        .into_iter();

        let report = run(
            move || Ok(blocks.next().unwrap()),
            &output,
            BlockPipelineConfig {
                num_workers: 1,
                max_in_flight_blocks: 1,
                force_overwrite: true,
                flags: build_flags(
                    false,
                    true,
                    QualityMode::Lossless,
                    IdMode::Tokenize,
                    false,
                    PeLayout::Interleaved,
                    ReadLengthClass::Short,
                    true,
                ),
                total_reads: None,
                expected_blocks: None,
                original_filename: "stdin".to_string(),
                reorder_map: None,
                compressor_config: BlockCompressorConfig::default(),
            },
        )
        .unwrap();

        assert_eq!(report.total_reads, 2);
        assert_eq!(report.total_bases, 8);
        assert_eq!(report.input_bytes, 33);
    }
}
