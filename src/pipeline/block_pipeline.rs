// =============================================================================
// fqc - Shared bounded block pipeline
// =============================================================================
//! Reader -> parallel compressor -> ordered writer.
//!
//! The caller supplies already-chunked reads in archive order. This module
//! owns the bounded channels, worker lifecycle, ordered block writes,
//! transactional output and the error propagation contract shared by archive
//! and pipeline compression.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use crate::archive::writer::{begin_fqc_writer, write_global_header};
use crate::block::EncodedBlock;
use crate::error::{FqcError, Result};
use crate::types::{ReadId, ReadRecord};

/// Everything the shared block pipeline needs after parsing and analysis.
pub(crate) struct BlockPipelineConfig {
    pub num_workers: usize,
    pub max_in_flight_blocks: usize,
    pub force_overwrite: bool,
    pub flags: u64,
    pub total_reads: u64,
    pub original_filename: String,
    pub reorder_map: Option<(Vec<ReadId>, Vec<ReadId>)>,
    pub compressor_config: BlockCompressorConfig,
}

/// Counters produced by the shared block pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlockPipelineReport {
    pub output_bytes: u64,
    pub blocks_written: u64,
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

/// Compress chunks in parallel and write complete blocks in archive order.
///
/// The input chunk order defines block order; block IDs are assigned from the
/// slice position. A worker or writer error aborts the transaction and never
/// commits a partial archive.
pub(crate) fn run(
    chunks: Vec<Vec<ReadRecord>>,
    output_path: impl AsRef<Path>,
    config: BlockPipelineConfig,
) -> Result<BlockPipelineReport> {
    let num_chunks = chunks.len();
    if num_chunks == 0 {
        return Err(FqcError::InvalidArgument("no blocks to write".to_string()));
    }

    let num_chunks_u32 =
        u32::try_from(num_chunks).map_err(|_| FqcError::Compression("block count exceeds u32".to_string()))?;
    let BlockPipelineConfig {
        num_workers,
        max_in_flight_blocks,
        force_overwrite,
        flags,
        total_reads,
        original_filename,
        reorder_map,
        compressor_config,
    } = config;
    let max_inflight = max_in_flight_blocks.max(1);
    let num_workers = num_workers.min(num_chunks).max(1);
    let output_path_owned: PathBuf = output_path.as_ref().to_path_buf();
    let compressor_config = Arc::new(compressor_config);
    let (chunk_tx, chunk_rx): (Sender<ReadChunk>, Receiver<ReadChunk>) = bounded(max_inflight);
    let (block_tx, block_rx): (Sender<OrderedBlock>, Receiver<OrderedBlock>) = bounded(max_inflight);

    // ---- Reader thread: feed pre-chunked reads in order ----
    let reader_handle = thread::spawn(move || -> Result<()> {
        for (chunk_id, reads) in chunks.into_iter().enumerate() {
            chunk_tx
                .send(ReadChunk {
                    reads,
                    chunk_id: chunk_id as u32,
                })
                .map_err(|_| FqcError::Compression("Reader: channel closed".to_string()))?;
        }
        Ok(())
    });

    // ---- Compressor threads ----
    let process_ns = Arc::new(AtomicU64::new(0));
    let mut compressor_handles = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        let rx = chunk_rx.clone();
        let tx = block_tx.clone();
        let cfg = Arc::clone(&compressor_config);
        let process_ns = Arc::clone(&process_ns);

        let handle = thread::spawn(move || -> Result<()> {
            let mut compressor = BlockCompressor::new((*cfg).clone());
            for chunk in rx.iter() {
                let t = Instant::now();
                let compressed = compressor.compress(&chunk.reads, chunk.chunk_id)?;
                process_ns.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);

                tx.send(OrderedBlock {
                    chunk_id: chunk.chunk_id,
                    compressed,
                })
                .map_err(|_| FqcError::Compression("Compressor: channel closed".to_string()))?;
            }
            Ok(())
        });
        compressor_handles.push(handle);
    }

    drop(chunk_rx);
    drop(block_tx);

    // ---- Writer thread: reorder completed blocks and commit atomically ----
    let writer_handle = thread::spawn(move || -> Result<(u64, u64)> {
        let t_setup = Instant::now();
        let (mut writer, output_tx) = begin_fqc_writer(&output_path_owned, force_overwrite)?;
        write_global_header(&mut writer, flags, total_reads, &original_filename)?;
        let mut write_ns = t_setup.elapsed().as_nanos() as u64;

        let mut pending: BTreeMap<u32, OrderedBlock> = BTreeMap::new();
        let mut next_expected: u32 = 0;

        for ordered_block in block_rx.iter() {
            pending.insert(ordered_block.chunk_id, ordered_block);
            while let Some(block) = pending.remove(&next_expected) {
                let t = Instant::now();
                writer.write_block(&block.compressed)?;
                write_ns += t.elapsed().as_nanos() as u64;
                next_expected += 1;
            }
        }

        // A worker error or cancellation closes the channel early. Never
        // commit a partial archive: the transaction drops the temp file.
        if next_expected < num_chunks_u32 || !pending.is_empty() {
            return Err(FqcError::Compression(format!(
                "Compression incomplete: wrote {}/{} blocks; output aborted",
                next_expected + pending.len() as u32,
                num_chunks
            )));
        }

        if let Some((forward, reverse)) = reorder_map.as_ref() {
            let t = Instant::now();
            writer.write_reorder_map(forward, reverse)?;
            write_ns += t.elapsed().as_nanos() as u64;
        }

        let t = Instant::now();
        let output_bytes = writer.finalize()?;
        output_tx.commit()?;
        write_ns += t.elapsed().as_nanos() as u64;
        Ok((output_bytes, write_ns / 1_000_000))
    });

    // Join every stage before returning so no detached worker can keep writing
    // after the caller observes an error.
    let reader_result = reader_handle
        .join()
        .map_err(|_| FqcError::Compression("Reader thread panicked".to_string()))
        .and_then(|result| result);
    let mut compressor_result = Ok(());
    for handle in compressor_handles {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                if compressor_result.is_ok() {
                    compressor_result = Err(error);
                }
            }
            Err(_) => {
                if compressor_result.is_ok() {
                    compressor_result = Err(FqcError::Compression("Compressor thread panicked".to_string()));
                }
            }
        }
    }
    let process_ms = process_ns.load(Ordering::Relaxed) / 1_000_000;
    let writer_result = writer_handle
        .join()
        .map_err(|_| FqcError::Compression("Writer thread panicked".to_string()))
        .and_then(|result| result);

    reader_result?;
    compressor_result?;
    let (output_bytes, write_ms) = writer_result?;

    Ok(BlockPipelineReport {
        output_bytes,
        blocks_written: num_chunks as u64,
        process_ms,
        write_ms,
    })
}
