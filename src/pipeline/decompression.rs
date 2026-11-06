// =============================================================================
// fqc-rust - Decompression pipeline
// =============================================================================
// Single decompression orchestration: Reader (serial) → Decompressor
// (parallel) → Writer (serial). Supports range extraction, header-only output,
// original-order restore, skip-corrupted placeholders and split R1/R2 output.
// =============================================================================

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use crate::archive::format::{get_id_mode, get_quality_mode, get_read_length_class};
use crate::archive::reader::FqcReader;
use crate::block::{DecodedBlock, EncodedBlock};
use crate::error::{FqcError, Result};
use crate::io::async_io::AsyncWriter;
use crate::io::OutputTransaction;
use crate::memory::DecodeBudget;

use super::decompression_writer::{placeholder_record, OutputWriters};
use super::DEFAULT_MAX_IN_FLIGHT_BLOCKS;
use crate::stats::OperationStats;

// =============================================================================
// DecompressionConfig
// =============================================================================

#[derive(Clone)]
pub struct DecompressionConfig {
    pub num_threads: usize,
    pub max_in_flight_blocks: usize,
    pub range_start: u64,
    pub range_end: u64,
    pub original_order: bool,
    pub header_only: bool,
    pub skip_corrupted: bool,
    pub corrupted_placeholder: Option<String>,
    pub force_overwrite: bool,
    /// Split paired-end output into R1/R2 files.
    pub split_pe: bool,
    /// Memory limit in MB (`0` = automatic, still finite).
    pub memory_limit_mb: usize,
}

impl Default for DecompressionConfig {
    fn default() -> Self {
        Self {
            num_threads: 0,
            max_in_flight_blocks: DEFAULT_MAX_IN_FLIGHT_BLOCKS,
            range_start: 0,
            range_end: 0,
            original_order: false,
            header_only: false,
            skip_corrupted: false,
            corrupted_placeholder: None,
            force_overwrite: false,
            split_pe: false,
            memory_limit_mb: 0,
        }
    }
}

impl DecompressionConfig {
    pub fn effective_threads(&self) -> usize {
        crate::plan::codec_thread_limit(self.num_threads, self.memory_limit_mb)
    }

    pub fn has_range(&self) -> bool {
        self.range_start > 0 || self.range_end > 0
    }

    pub(crate) fn validate_range(&self) -> Result<()> {
        if self.range_start > 0 && self.range_end > 0 && self.range_start > self.range_end {
            return Err(FqcError::InvalidArgument(format!(
                "Invalid read range: start {} is greater than end {}",
                self.range_start, self.range_end
            )));
        }
        Ok(())
    }
}

// =============================================================================
// Internal types
// =============================================================================

struct BlockTask {
    block_id: u32,
    block_data: EncodedBlock,
}

struct DecompressedResult {
    block_id: u32,
    result: std::result::Result<DecodedBlock, FqcError>,
    expected_read_count: u32,
}

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

struct PreparedRun {
    reader: FqcReader,
    file_size: u64,
    start_block: usize,
    end_block: usize,
    compressor_config: Arc<BlockCompressorConfig>,
    max_inflight: usize,
    num_workers: usize,
    writer_input: WriterThreadInput,
}

struct WriterThreadInput {
    output_path: PathBuf,
    force_overwrite: bool,
    split_pe: bool,
    archive_flags: u64,
    original_order: bool,
    header_only: bool,
    skip_corrupted: bool,
    corrupted_placeholder: String,
    range_start: u64,
    range_end: u64,
    has_range: bool,
    start_block: usize,
    end_block: usize,
    reads_before_start_block: u64,
    total_archive_reads: u64,
    forward_map: Option<Vec<crate::types::ReadId>>,
}

// =============================================================================
// DecompressionPipeline
// =============================================================================

pub struct DecompressionPipeline {
    config: DecompressionConfig,
    stats: OperationStats,
}

impl DecompressionPipeline {
    pub fn new(config: DecompressionConfig) -> Self {
        Self {
            config,
            stats: OperationStats::default(),
        }
    }

    pub fn stats(&self) -> &OperationStats {
        &self.stats
    }

    /// Run decompression pipeline
    pub fn run(&mut self, input_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> Result<()> {
        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();
        self.config.validate_range()?;
        let start = Instant::now();

        let PreparedRun {
            reader,
            file_size,
            start_block,
            end_block,
            compressor_config,
            max_inflight,
            num_workers,
            writer_input,
            ..
        } = self.prepare(input_path, output_path)?;

        let (task_tx, task_rx): (Sender<BlockTask>, Receiver<BlockTask>) = bounded(max_inflight);
        let (result_tx, result_rx): (Sender<DecompressedResult>, Receiver<DecompressedResult>) = bounded(max_inflight);

        let skip_corrupted = self.config.skip_corrupted;
        let reader_result_tx = result_tx.clone();

        // ---- Reader thread ----
        let reader_handle = thread::spawn(move || -> Result<u64> {
            let mut reader = reader;
            let t = Instant::now();
            for block_id in start_block..end_block {
                let expected_read_count = reader
                    .block_index
                    .entries
                    .get(block_id)
                    .map(|entry| entry.read_count)
                    .unwrap_or(0);
                match reader.read_block(block_id as u32) {
                    Ok(block_data) => task_tx
                        .send(BlockTask {
                            block_id: block_id as u32,
                            block_data,
                        })
                        .map_err(|_| FqcError::Decompression("Reader: channel closed".to_string()))?,
                    Err(e) => {
                        if skip_corrupted {
                            reader_result_tx
                                .send(DecompressedResult {
                                    block_id: block_id as u32,
                                    result: Err(e),
                                    expected_read_count,
                                })
                                .map_err(|_| FqcError::Decompression("Reader: result channel closed".to_string()))?;
                            continue;
                        }
                        return Err(e);
                    }
                }
            }
            Ok(t.elapsed().as_millis() as u64)
        });

        // ---- Decompressor threads ----
        let num_decomp_threads = num_workers.max(1);
        let mut decomp_handles = Vec::new();
        let decode_ns = Arc::new(std::sync::atomic::AtomicU64::new(0));

        for _ in 0..num_decomp_threads {
            let rx = task_rx.clone();
            let tx = result_tx.clone();
            let cfg = compressor_config.clone();
            let decode_ns = Arc::clone(&decode_ns);

            let handle = thread::spawn(move || -> Result<()> {
                let mut compressor = BlockCompressor::new((*cfg).clone());

                for task in rx.iter() {
                    let bh = &task.block_data.meta;
                    let t = Instant::now();
                    let decomp_result = compressor.decompress_block(&task.block_data);
                    decode_ns.fetch_add(t.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);

                    tx.send(DecompressedResult {
                        block_id: task.block_id,
                        result: decomp_result,
                        expected_read_count: bh.read_count,
                    })
                    .map_err(|_| FqcError::Decompression("Decompressor: channel closed".to_string()))?;
                }
                Ok(())
            });
            decomp_handles.push(handle);
        }
        drop(task_rx);
        drop(result_tx);

        // ---- Writer thread ----
        let writer_handle = self.spawn_writer_thread(writer_input, result_rx);

        // ---- Wait ----
        let reader_read_ms = reader_handle
            .join()
            .map_err(|_| FqcError::Decompression("Reader thread panicked".to_string()))??;
        for h in decomp_handles {
            h.join()
                .map_err(|_| FqcError::Decompression("Decompressor thread panicked".to_string()))??;
        }
        let decode_ms = decode_ns.load(std::sync::atomic::Ordering::Relaxed) / 1_000_000;
        let (reads_written, output_bytes, total_bases, corrupted_blocks, writer_write_ms) = writer_handle
            .join()
            .map_err(|_| FqcError::Decompression("Writer thread panicked".to_string()))??;

        let elapsed = start.elapsed();
        self.stats = OperationStats {
            total_reads: reads_written,
            total_bases,
            blocks: (end_block - start_block) as u64,
            input_bytes: file_size,
            output_bytes,
            elapsed_seconds: elapsed.as_secs_f64(),
            reorder_map_written: false,
            corrupted_blocks,
            parse_ms: reader_read_ms,
            reorder_ms: 0,
            process_ms: decode_ms,
            write_ms: writer_write_ms,
        };

        log::info!(
            "Decompression complete: {} reads, {} blocks, {:.1} MB/s",
            self.stats.total_reads,
            self.stats.blocks,
            self.stats.throughput_mbps(),
        );

        Ok(())
    }

    /// Prepare the reader, block range, and per-writer inputs ahead of
    /// spawning the threaded pipeline. Kept separate so `run` stays under the
    /// line budget and the reorder/range semantics live in one place.
    fn prepare(&self, input_path: &Path, output_path: &Path) -> Result<PreparedRun> {
        let budget = DecodeBudget::resolve(self.config.memory_limit_mb);
        let mut reader = FqcReader::open_with_budget(input_path, budget)?;
        let block_count = reader.block_count();
        let file_size = reader.file_size;

        if self.config.original_order {
            if !reader.has_reorder_map() {
                return Err(FqcError::Format(
                    "Original order requested but no reorder map present".to_string(),
                ));
            }
            let avg_length = reader.memory_estimate_read_length() as u64;
            reader.budget().check_original_order_peak(
                reader.total_read_count(),
                reader.max_block_compressed_size(),
                avg_length,
            )?;
            reader.load_reorder_map()?;
            log::info!("Reorder map loaded");
        }

        let f = reader.global_header.flags;
        let quality_mode = get_quality_mode(f);
        let id_mode = get_id_mode(f);
        let read_length_class = get_read_length_class(f);

        let (start_block, end_block) = if self.config.original_order {
            (0, block_count)
        } else if self.config.has_range() {
            self.find_block_range(&reader)
        } else {
            (0, block_count)
        };

        // Compute how many reads exist before start_block (for correct range filtering)
        let reads_before_start_block: u64 = if start_block > 0 {
            reader
                .block_index
                .entries
                .get(start_block)
                .map(|e| e.archive_id_start)
                .unwrap_or(0)
        } else {
            0
        };

        let compressor_config = Arc::new(BlockCompressorConfig {
            read_length_class,
            quality_mode,
            id_mode,
            ..Default::default()
        });

        let block_hint = reader.max_block_compressed_size();
        let max_block_reads = reader
            .block_index
            .entries
            .iter()
            .map(|entry| entry.read_count as usize)
            .max()
            .unwrap_or(1_000);
        let worker_bytes =
            compressor_config.estimated_worker_bytes(max_block_reads, reader.memory_estimate_read_length()) as u64;
        let num_workers = reader
            .budget()
            .max_workers_for_budget(worker_bytes, reader.budget().worker_budget_bytes())
            .min(self.config.effective_threads())
            .min(block_count.max(1))
            .max(1);
        let budget_batch = reader
            .budget()
            .parallel_batch_size(self.config.effective_threads(), block_hint)?
            .max(1);
        let max_inflight = self.config.max_in_flight_blocks.min(budget_batch).max(1);
        let total_archive_reads = reader.total_read_count();
        let forward_map = if self.config.original_order {
            reader.reorder_forward.clone()
        } else {
            None
        };

        Ok(PreparedRun {
            reader,
            file_size,
            start_block,
            end_block,
            compressor_config,
            max_inflight,
            num_workers,
            writer_input: WriterThreadInput {
                output_path: output_path.to_path_buf(),
                force_overwrite: self.config.force_overwrite,
                split_pe: self.config.split_pe,
                archive_flags: f,
                original_order: self.config.original_order,
                header_only: self.config.header_only,
                skip_corrupted: self.config.skip_corrupted,
                corrupted_placeholder: self
                    .config
                    .corrupted_placeholder
                    .clone()
                    .unwrap_or_else(|| "N".to_string()),
                range_start: self.config.range_start,
                range_end: self.config.range_end,
                has_range: self.config.has_range(),
                start_block,
                end_block,
                reads_before_start_block,
                total_archive_reads,
                forward_map,
            },
        })
    }

    /// Writer stage: receives decompressed blocks in archive order, applies
    /// range filtering, optional original-order restore and split-PE routing,
    /// then commits the output transaction.
    fn spawn_writer_thread(
        &self,
        input: WriterThreadInput,
        result_rx: Receiver<DecompressedResult>,
    ) -> thread::JoinHandle<Result<(u64, u64, u64, u32, u64)>> {
        let output_path_owned = input.output_path;
        let force_overwrite = input.force_overwrite;
        let split_pe = input.split_pe;
        let archive_flags = input.archive_flags;
        let original_order = input.original_order;
        let header_only = input.header_only;
        let skip_corrupted = input.skip_corrupted;
        let corrupted_placeholder = input.corrupted_placeholder;
        let range_start = input.range_start;
        let range_end = input.range_end;
        let has_range = input.has_range;
        let start_block = input.start_block;
        let end_block = input.end_block;
        let reads_before_start_block = input.reads_before_start_block;
        let total_archive_reads = input.total_archive_reads;
        let forward_map = input.forward_map;

        thread::spawn(move || -> Result<(u64, u64, u64, u32, u64)> {
            let t = Instant::now();
            const ASYNC_WRITE_BUF: usize = 4 * 1024 * 1024; // 4 MB write-behind buffer
            const ASYNC_WRITE_DEPTH: usize = 4;

            let mut output = if split_pe {
                OutputWriters::split(&output_path_owned, force_overwrite, archive_flags)?
            } else if output_path_owned.as_path() == Path::new("-") {
                OutputWriters::single_raw(Box::new(std::io::BufWriter::new(std::io::stdout())), None)
            } else {
                let mut tx = OutputTransaction::begin(&output_path_owned, force_overwrite)?;
                let file = tx.take_file()?;
                OutputWriters::single_raw(
                    Box::new(AsyncWriter::new(file, ASYNC_WRITE_DEPTH, ASYNC_WRITE_BUF)),
                    Some(tx),
                )
            };

            let mut pending: std::collections::BTreeMap<u32, DecompressedResult> = std::collections::BTreeMap::new();
            let mut next_expected: u32 = start_block as u32;
            let mut total_output_bytes: u64 = 0;
            let mut total_reads_written: u64 = 0;
            let mut total_bases: u64 = 0;
            let mut corrupted_blocks: u32 = 0;
            let mut global_read_idx: u64 = reads_before_start_block;
            let end_block_id: u32 = end_block as u32;
            let mut original_reads: Vec<crate::types::ReadRecord> = if original_order {
                Vec::with_capacity(total_archive_reads as usize)
            } else {
                Vec::new()
            };

            for dr in result_rx.iter() {
                pending.insert(dr.block_id, dr);

                while let Some(dr) = pending.remove(&next_expected) {
                    match dr.result {
                        Ok(decompressed) => {
                            if original_order {
                                original_reads.extend(decompressed.reads);
                            } else {
                                let block_read_count = decompressed.reads.len() as u64;
                                for (local_idx, read) in decompressed.reads.iter().enumerate() {
                                    global_read_idx += 1;
                                    if !in_range(global_read_idx, has_range, range_start, range_end) {
                                        continue;
                                    }
                                    total_output_bytes += output.write_record(
                                        read,
                                        header_only,
                                        global_read_idx - 1,
                                        total_archive_reads,
                                        Some((local_idx as u64, block_read_count)),
                                    )?;
                                    total_reads_written += 1;
                                    total_bases += read.sequence.len() as u64;
                                }
                            }
                        }
                        Err(e) => {
                            if skip_corrupted {
                                if original_order {
                                    for read_idx in 0..dr.expected_read_count as usize {
                                        original_reads.push(placeholder_record(
                                            &corrupted_placeholder,
                                            dr.block_id,
                                            read_idx,
                                        ));
                                    }
                                } else {
                                    for read_idx in 0..dr.expected_read_count as usize {
                                        global_read_idx += 1;
                                        if !in_range(global_read_idx, has_range, range_start, range_end) {
                                            continue;
                                        }
                                        let placeholder =
                                            placeholder_record(&corrupted_placeholder, dr.block_id, read_idx);
                                        total_output_bytes += output.write_record(
                                            &placeholder,
                                            header_only,
                                            global_read_idx - 1,
                                            total_archive_reads,
                                            Some((read_idx as u64, dr.expected_read_count as u64)),
                                        )?;
                                        total_reads_written += 1;
                                        total_bases += placeholder.sequence.len() as u64;
                                    }
                                }
                                corrupted_blocks += 1;
                                log::warn!("Block {} corrupted, skipping: {}", dr.block_id, e);
                            } else {
                                return Err(e);
                            }
                        }
                    }
                    next_expected += 1;
                }
            }

            // A worker error closes the channel early. Never commit a partial
            // output: the transaction drops the temp file.
            if !pending.is_empty() || next_expected < end_block_id {
                return Err(FqcError::Decompression(format!(
                    "Decompression incomplete: processed {} of blocks up to {}; output aborted",
                    next_expected - start_block as u32,
                    end_block_id
                )));
            }

            if original_order {
                let forward = forward_map.ok_or_else(|| FqcError::Format("Forward reorder map missing".to_string()))?;
                for (original_id, &archive_id) in forward.iter().enumerate() {
                    let Some(read) = original_reads.get(archive_id as usize) else {
                        log::warn!(
                            "Invalid reorder map reference: original_id {} -> archive_id {} (max {})",
                            original_id,
                            archive_id,
                            original_reads.len().saturating_sub(1)
                        );
                        continue;
                    };
                    let current_id = original_id as u64 + 1;
                    if !in_range(current_id, has_range, range_start, range_end) {
                        continue;
                    }
                    total_output_bytes +=
                        output.write_record(read, header_only, original_id as u64, total_archive_reads, None)?;
                    total_reads_written += 1;
                    total_bases += read.sequence.len() as u64;
                }
            }

            output.commit()?;
            Ok((
                total_reads_written,
                total_output_bytes,
                total_bases,
                corrupted_blocks,
                t.elapsed().as_millis() as u64,
            ))
        })
    }

    /// Find the block range that covers the requested read range
    fn find_block_range(&self, reader: &FqcReader) -> (usize, usize) {
        let entries = &reader.block_index.entries;
        let range_start = if self.config.range_start > 0 {
            self.config.range_start - 1
        } else {
            0
        };
        let range_end = if self.config.range_end > 0 {
            self.config.range_end
        } else {
            reader.total_read_count()
        };

        let mut start_block = 0;
        let mut end_block = entries.len();

        for (i, entry) in entries.iter().enumerate() {
            if entry.archive_id_end() <= range_start {
                start_block = i + 1;
            }
            if entry.archive_id_start >= range_end {
                end_block = i;
                break;
            }
        }

        (start_block, end_block)
    }
}
