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
use crate::error::{FqcError, Result};
use crate::fastq::parser::write_record;
use crate::format::{flags, get_id_mode, get_pe_layout, get_quality_mode, get_read_length_class};
use crate::fqc_reader::{BlockData, FqcReader};
use crate::io::async_io::AsyncWriter;

use super::{PipelineControl, PipelineStats, ProgressCallback, DEFAULT_MAX_IN_FLIGHT_BLOCKS};

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
    pub verify_checksums: bool,
    pub skip_corrupted: bool,
    pub split_pe: bool,
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
            verify_checksums: true,
            skip_corrupted: false,
            split_pe: false,
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
}

// =============================================================================
// Internal types
// =============================================================================

struct BlockTask {
    block_id: u32,
    block_data: BlockData,
    is_last: bool,
}

struct DecompressedResult {
    block_id: u32,
    result: std::result::Result<DecompressedBlockData, FqcError>,
    is_last: bool,
    expected_read_count: u32,
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

    pub fn control(&self) -> &PipelineControl {
        &self.control
    }

    pub fn stats(&self) -> &PipelineStats {
        &self.stats
    }

    /// Run decompression pipeline
    #[allow(clippy::too_many_lines)]
    pub fn run(
        &mut self,
        input_path: &str,
        output_path: &str,
        _progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        let start = Instant::now();
        let threads = self.config.effective_threads();

        let mut reader = FqcReader::open(input_path)?;
        let block_count = reader.block_count();
        let _total_reads = reader.total_read_count();
        let file_size = reader.file_size;

        let f = reader.global_header.flags;
        let quality_mode = get_quality_mode(f);
        let id_mode = get_id_mode(f);
        let read_length_class = get_read_length_class(f);
        let _is_paired = (f & flags::IS_PAIRED) != 0;
        let _pe_layout = get_pe_layout(f);

        let output_path_owned = output_path.to_string();

        // Load reorder map if needed
        if self.config.original_order && reader.has_reorder_map() {
            reader.load_reorder_map()?;
        }

        // Determine block range
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

        let max_inflight = self.config.max_in_flight_blocks;
        let (task_tx, task_rx): (Sender<BlockTask>, Receiver<BlockTask>) = bounded(max_inflight);
        let (result_tx, result_rx): (Sender<DecompressedResult>, Receiver<DecompressedResult>) = bounded(max_inflight);

        let control = self.control.clone();

        // ---- Reader thread ----
        let reader_control = control.clone();
        let reader_handle = thread::spawn(move || -> Result<()> {
            for block_id in start_block..end_block {
                if reader_control.is_cancelled() {
                    break;
                }

                let block_data = reader.read_block(block_id as u32)?;
                let is_last = block_id + 1 == end_block;

                task_tx
                    .send(BlockTask {
                        block_id: block_id as u32,
                        block_data,
                        is_last,
                    })
                    .map_err(|_| FqcError::Decompression("Reader: channel closed".to_string()))?;
            }
            Ok(())
        });

        // ---- Decompressor threads ----
        let num_decomp_threads = threads.max(1);
        let mut decomp_handles = Vec::new();

        for _ in 0..num_decomp_threads {
            let rx = task_rx.clone();
            let tx = result_tx.clone();
            let cfg = compressor_config.clone();
            let ctrl = control.clone();

            let handle = thread::spawn(move || -> Result<()> {
                let compressor = BlockCompressor::new((*cfg).clone());

                for task in rx.iter() {
                    if ctrl.is_cancelled() {
                        break;
                    }

                    let bh = &task.block_data.header;
                    let decomp_result = compressor.decompress_raw(
                        bh.block_id,
                        bh.uncompressed_count,
                        bh.uniform_read_length,
                        bh.codec_seq,
                        bh.codec_qual,
                        &task.block_data.ids_data,
                        &task.block_data.seq_data,
                        &task.block_data.qual_data,
                        &task.block_data.aux_data,
                    );

                    ctrl.add_reads(bh.uncompressed_count as u64);

                    tx.send(DecompressedResult {
                        block_id: task.block_id,
                        result: decomp_result,
                        is_last: task.is_last,
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
        let writer_control = control.clone();
        let header_only = self.config.header_only;
        let skip_corrupted = self.config.skip_corrupted;
        let range_start = self.config.range_start;
        let range_end = self.config.range_end;
        let has_range = self.config.has_range();
        let writer_handle = thread::spawn(move || -> Result<(u64, u64)> {
            const ASYNC_WRITE_BUF: usize = 4 * 1024 * 1024; // 4 MB write-behind buffer
            const ASYNC_WRITE_DEPTH: usize = 4;

            let mut output: Box<dyn std::io::Write> = if output_path_owned == "-" {
                Box::new(std::io::BufWriter::new(std::io::stdout()))
            } else {
                let file = std::fs::File::create(&output_path_owned).map_err(FqcError::Io)?;
                Box::new(AsyncWriter::new(file, ASYNC_WRITE_DEPTH, ASYNC_WRITE_BUF))
            };

            let mut pending: std::collections::BTreeMap<u32, DecompressedResult> = std::collections::BTreeMap::new();
            let mut next_expected: u32 = start_block as u32;
            let mut total_output_bytes: u64 = 0;
            let mut total_reads_written: u64 = 0;
            let mut global_read_idx: u64 = reads_before_start_block;

            for dr in result_rx.iter() {
                if writer_control.is_cancelled() {
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
                                if header_only {
                                    let line = if read.comment.is_empty() {
                                        format!("@{}\n", read.id)
                                    } else {
                                        format!("@{} {}\n", read.id, read.comment)
                                    };
                                    output.write_all(line.as_bytes()).map_err(FqcError::Io)?;
                                    total_output_bytes += line.len() as u64;
                                } else {
                                    write_record(output.as_mut(), read)?;
                                    let comment_bytes = if read.comment.is_empty() {
                                        0
                                    } else {
                                        read.comment.len() + 1
                                    };
                                    total_output_bytes +=
                                        (read.id.len() + comment_bytes + read.sequence.len() + read.quality.len() + 5)
                                            as u64;
                                }
                                total_reads_written += 1;
                            }
                        }
                        Err(e) => {
                            if skip_corrupted {
                                // Account for skipped reads so global_read_idx stays correct
                                global_read_idx += dr.expected_read_count as u64;
                                log::warn!("Block {} corrupted, skipping: {}", dr.block_id, e);
                            } else {
                                return Err(e);
                            }
                        }
                    }
                    next_expected += 1;
                }
            }

            output.flush().map_err(FqcError::Io)?;
            Ok((total_reads_written, total_output_bytes))
        });

        // ---- Wait ----
        reader_handle
            .join()
            .map_err(|_| FqcError::Decompression("Reader thread panicked".to_string()))??;
        for h in decomp_handles {
            h.join()
                .map_err(|_| FqcError::Decompression("Decompressor thread panicked".to_string()))??;
        }
        let (reads_written, output_bytes) = writer_handle
            .join()
            .map_err(|_| FqcError::Decompression("Writer thread panicked".to_string()))??;

        let elapsed = start.elapsed();
        self.stats = PipelineStats {
            total_reads: reads_written,
            total_blocks: (end_block - start_block) as u32,
            input_bytes: file_size,
            output_bytes,
            processing_time_ms: elapsed.as_millis() as u64,
            peak_memory_bytes: 0,
            threads_used: threads,
        };

        log::info!(
            "Decompression complete: {} reads, {} blocks, {:.1} MB/s",
            self.stats.total_reads,
            self.stats.total_blocks,
            self.stats.throughput_mbps(),
        );

        Ok(())
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

    pub fn cancel(&self) {
        self.control.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.control.is_cancelled()
    }
}
