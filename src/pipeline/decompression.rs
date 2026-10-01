// =============================================================================
// fqc-rust - Decompression Pipeline
// =============================================================================
// 3-stage pipeline: Reader (serial) → Decompressor (parallel) → Writer (serial)
// Supports range extraction and original order output.
// =============================================================================

use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig, DecompressedBlockData};
use crate::archive::format::{get_id_mode, get_quality_mode, get_read_length_class};
use crate::archive::reader::FqcReader;
use crate::archive::traits::BlockData;
use crate::error::{FqcError, Result};
use crate::fastq::parser::write_record;
use crate::io::async_io::AsyncWriter;
use crate::io::OutputTransaction;
use crate::memory_budget::DecodeBudget;

use super::{PipelineControl, PipelineStats, DEFAULT_MAX_IN_FLIGHT_BLOCKS};

// =============================================================================
// DecompressionPipelineConfig
// =============================================================================

#[derive(Clone)]
pub struct DecompressionPipelineConfig {
    pub num_threads: usize,
    pub max_in_flight_blocks: usize,
    pub range_start: u64,
    pub range_end: u64,
    pub original_order: bool,
    pub header_only: bool,
    pub skip_corrupted: bool,
    pub corrupted_placeholder: Option<String>,
    pub force_overwrite: bool,
    /// Split paired-end output into R1/R2 files (sequential path only).
    pub split_pe: bool,
    /// Memory limit in MB (`0` = automatic, still finite).
    pub memory_limit_mb: usize,
}

impl Default for DecompressionPipelineConfig {
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

impl DecompressionPipelineConfig {
    pub fn effective_threads(&self) -> usize {
        if self.num_threads == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
                .max(1)
        } else {
            self.num_threads
        }
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
    block_data: BlockData,
}

struct DecompressedResult {
    block_id: u32,
    result: std::result::Result<DecompressedBlockData, FqcError>,
    expected_read_count: u32,
}

fn placeholder_record(placeholder_seq: &str, block_id: u32, read_idx: usize) -> crate::types::ReadRecord {
    crate::types::ReadRecord {
        id: format!("corrupted_block{}_read{}", block_id, read_idx),
        comment: String::new(),
        sequence: placeholder_seq.to_string(),
        quality: "!".repeat(placeholder_seq.len()),
    }
}

fn write_output_read(
    output: &mut dyn std::io::Write,
    read: &crate::types::ReadRecord,
    header_only: bool,
) -> Result<u64> {
    if header_only {
        let line = if read.comment.is_empty() {
            format!("@{}\n", read.id)
        } else {
            format!("@{} {}\n", read.id, read.comment)
        };
        output.write_all(line.as_bytes()).map_err(FqcError::Io)?;
        Ok(line.len() as u64)
    } else {
        write_record(output, read)?;
        let comment_bytes = if read.comment.is_empty() {
            0_u64
        } else {
            read.comment.len() as u64 + 1
        };
        Ok(read.id.len() as u64 + comment_bytes + read.sequence.len() as u64 + read.quality.len() as u64 + 4)
    }
}

struct PreparedRun {
    reader: FqcReader,
    file_size: u64,
    start_block: usize,
    end_block: usize,
    compressor_config: Arc<BlockCompressorConfig>,
    max_inflight: usize,
    writer_input: WriterThreadInput,
}

struct WriterThreadInput {
    output_path: String,
    force_overwrite: bool,
    header_only: bool,
    skip_corrupted: bool,
    corrupted_placeholder: String,
    range_start: u64,
    range_end: u64,
    has_range: bool,
    start_block: usize,
    end_block: usize,
    reads_before_start_block: u64,
}

// =============================================================================
// DecompressionPipeline
// =============================================================================

pub struct DecompressionPipeline {
    config: DecompressionPipelineConfig,
    control: PipelineControl,
    stats: PipelineStats,
}

impl DecompressionPipeline {
    pub fn new(config: DecompressionPipelineConfig) -> Self {
        Self {
            config,
            control: PipelineControl::new(),
            stats: PipelineStats::default(),
        }
    }

    pub fn stats(&self) -> &PipelineStats {
        &self.stats
    }

    /// Run decompression pipeline
    pub fn run(&mut self, input_path: &str, output_path: &str) -> Result<()> {
        if self.config.split_pe {
            return Err(FqcError::InvalidArgument(
                "--split-pe requires the sequential decompression path".to_string(),
            ));
        }
        if self.config.original_order {
            return Err(FqcError::InvalidArgument(
                "--original-order requires the sequential decompression path".to_string(),
            ));
        }
        self.config.validate_range()?;
        let start = Instant::now();
        let threads = self.config.effective_threads();

        let PreparedRun {
            reader,
            file_size,
            start_block,
            end_block,
            compressor_config,
            max_inflight,
            writer_input,
            ..
        } = self.prepare(input_path, output_path)?;

        let (task_tx, task_rx): (Sender<BlockTask>, Receiver<BlockTask>) = bounded(max_inflight);
        let (result_tx, result_rx): (Sender<DecompressedResult>, Receiver<DecompressedResult>) = bounded(max_inflight);

        let control = self.control.clone();
        let skip_corrupted = self.config.skip_corrupted;
        let reader_result_tx = result_tx.clone();

        // ---- Reader thread ----
        let reader_control = control.clone();
        let reader_handle = thread::spawn(move || -> Result<u64> {
            let mut reader = reader;
            let t = Instant::now();
            for block_id in start_block..end_block {
                if reader_control.is_cancelled() {
                    break;
                }

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
        let num_decomp_threads = threads.max(1);
        let mut decomp_handles = Vec::new();
        let decode_ns = Arc::new(std::sync::atomic::AtomicU64::new(0));

        for _ in 0..num_decomp_threads {
            let rx = task_rx.clone();
            let tx = result_tx.clone();
            let cfg = compressor_config.clone();
            let ctrl = control.clone();
            let decode_ns = Arc::clone(&decode_ns);

            let handle = thread::spawn(move || -> Result<()> {
                let mut compressor = BlockCompressor::new((*cfg).clone());

                for task in rx.iter() {
                    if ctrl.is_cancelled() {
                        break;
                    }

                    let bh = &task.block_data.header;
                    let t = Instant::now();
                    let decomp_result = compressor.decompress_block(&task.block_data);
                    decode_ns.fetch_add(t.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);

                    ctrl.add_reads(bh.uncompressed_count as u64);

                    tx.send(DecompressedResult {
                        block_id: task.block_id,
                        result: decomp_result,
                        expected_read_count: bh.uncompressed_count,
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
        let writer_handle = self.spawn_writer_thread(writer_input, result_rx, control.clone());

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
        self.stats = PipelineStats {
            total_reads: reads_written,
            total_bases,
            total_blocks: (end_block - start_block) as u32,
            input_bytes: file_size,
            output_bytes,
            processing_time_ms: elapsed.as_millis() as u64,
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
            self.stats.total_blocks,
            self.stats.throughput_mbps(),
        );

        Ok(())
    }

    /// Prepare the reader, block range, and per-writer inputs ahead of
    /// spawning the threaded pipeline. Kept separate so `run` stays under the
    /// line budget and the reorder/range semantics live in one place.
    fn prepare(&self, input_path: &str, output_path: &str) -> Result<PreparedRun> {
        let budget = DecodeBudget::resolve(self.config.memory_limit_mb);
        let reader = FqcReader::open_with_budget(input_path, budget)?;
        let block_count = reader.block_count();
        let file_size = reader.file_size;

        let f = reader.global_header.flags;
        let quality_mode = get_quality_mode(f);
        let id_mode = get_id_mode(f);
        let read_length_class = get_read_length_class(f);

        let (start_block, end_block) = if self.config.has_range() {
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
        let budget_batch = reader
            .budget()
            .parallel_batch_size(self.config.effective_threads(), block_hint)?
            .max(1);
        let max_inflight = self.config.max_in_flight_blocks.min(budget_batch).max(1);

        Ok(PreparedRun {
            reader,
            file_size,
            start_block,
            end_block,
            compressor_config,
            max_inflight,
            writer_input: WriterThreadInput {
                output_path: output_path.to_string(),
                force_overwrite: self.config.force_overwrite,
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
            },
        })
    }

    /// Writer stage: receives decompressed blocks in archive order, applies
    /// range filtering, and writes transactionally (or to stdout).
    #[allow(clippy::too_many_arguments)]
    fn spawn_writer_thread(
        &self,
        input: WriterThreadInput,
        result_rx: Receiver<DecompressedResult>,
        control: PipelineControl,
    ) -> thread::JoinHandle<Result<(u64, u64, u64, u32, u64)>> {
        let output_path_owned = input.output_path;
        let force_overwrite = input.force_overwrite;
        let header_only = input.header_only;
        let skip_corrupted = input.skip_corrupted;
        let corrupted_placeholder = input.corrupted_placeholder;
        let range_start = input.range_start;
        let range_end = input.range_end;
        let has_range = input.has_range;
        let start_block = input.start_block;
        let end_block = input.end_block;
        let reads_before_start_block = input.reads_before_start_block;

        thread::spawn(move || -> Result<(u64, u64, u64, u32, u64)> {
            let t = Instant::now();
            const ASYNC_WRITE_BUF: usize = 4 * 1024 * 1024; // 4 MB write-behind buffer
            const ASYNC_WRITE_DEPTH: usize = 4;

            let (mut output, output_tx): (Box<dyn std::io::Write>, Option<OutputTransaction>) =
                if output_path_owned == "-" {
                    (Box::new(std::io::BufWriter::new(std::io::stdout())), None)
                } else {
                    let mut tx = OutputTransaction::begin(&output_path_owned, force_overwrite)?;
                    let file = tx.take_file()?;
                    (
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

            for dr in result_rx.iter() {
                if control.is_cancelled() {
                    break;
                }
                pending.insert(dr.block_id, dr);

                while let Some(dr) = pending.remove(&next_expected) {
                    match dr.result {
                        Ok(decompressed) => {
                            for read in &decompressed.reads {
                                global_read_idx += 1;
                                // Per-read range filtering (1-based)
                                if has_range {
                                    if range_start > 0 && global_read_idx < range_start {
                                        continue;
                                    }
                                    if range_end > 0 && global_read_idx > range_end {
                                        continue;
                                    }
                                }
                                total_output_bytes += write_output_read(output.as_mut(), read, header_only)?;
                                total_reads_written += 1;
                                total_bases += read.sequence.len() as u64;
                            }
                        }
                        Err(e) => {
                            if skip_corrupted {
                                for read_idx in 0..dr.expected_read_count as usize {
                                    global_read_idx += 1;
                                    if has_range {
                                        if range_start > 0 && global_read_idx < range_start {
                                            continue;
                                        }
                                        if range_end > 0 && global_read_idx > range_end {
                                            continue;
                                        }
                                    }
                                    let placeholder = placeholder_record(&corrupted_placeholder, dr.block_id, read_idx);
                                    total_output_bytes +=
                                        write_output_read(output.as_mut(), &placeholder, header_only)?;
                                    total_reads_written += 1;
                                    total_bases += placeholder.sequence.len() as u64;
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

            // A worker error or cancellation closes the channel early. Never
            // commit a partial output: the transaction drops the temp file.
            if !pending.is_empty() || (next_expected < end_block_id && !control.is_cancelled()) {
                return Err(FqcError::Decompression(format!(
                    "Decompression incomplete: processed {} of blocks up to {}; output aborted",
                    next_expected - start_block as u32,
                    end_block_id
                )));
            }

            output.flush().map_err(FqcError::Io)?;
            drop(output);
            if let Some(tx) = output_tx {
                tx.commit()?;
            }
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
