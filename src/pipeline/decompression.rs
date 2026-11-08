// =============================================================================
// fqc-rust - Decompression pipeline
// =============================================================================
// Single decompression orchestration: Reader (serial) → Decompressor
// (parallel) → Writer (serial). Supports range extraction, header-only output,
// original-order restore, skip-corrupted placeholders and split R1/R2 output.
// =============================================================================

use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use crate::archive::format::{get_id_mode, get_quality_mode, get_read_length_class};
use crate::archive::reader::FqcReader;
use crate::block::EncodedBlock;
use crate::error::{FqcError, Result};
use crate::memory::DecodeBudget;
use crate::plan::decompress::DecompressSettings;

use super::decompression_writer::{spawn_writer_thread, BlockResult, WriterPlan};
use super::DEFAULT_MAX_IN_FLIGHT_BLOCKS;

/// 多条线程同时失败时优先返回真实根因，而不是 channel 关闭的次生错误。
fn prefer_thread_error(errors: Vec<FqcError>) -> FqcError {
    let mut fallback = None;
    for error in errors {
        if !format!("{error}").contains("channel closed") {
            return error;
        }
        fallback.get_or_insert(error);
    }
    fallback.unwrap_or_else(|| FqcError::Decompression("Decompression pipeline failed without an error".to_string()))
}
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

impl From<&DecompressSettings> for DecompressionConfig {
    fn from(settings: &DecompressSettings) -> Self {
        Self {
            num_threads: settings.threads,
            max_in_flight_blocks: DEFAULT_MAX_IN_FLIGHT_BLOCKS,
            range_start: settings.range_start,
            range_end: settings.range_end,
            original_order: settings.original_order,
            header_only: settings.header_only,
            skip_corrupted: settings.skip_corrupted,
            corrupted_placeholder: settings.corrupted_placeholder.clone(),
            force_overwrite: settings.force_overwrite,
            split_pe: settings.split_pe,
            memory_limit_mb: settings.memory_limit_mb,
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

struct PreparedRun {
    reader: FqcReader,
    file_size: u64,
    start_block: usize,
    end_block: usize,
    compressor_config: Arc<BlockCompressorConfig>,
    max_inflight: usize,
    num_workers: usize,
    writer_input: WriterPlan,
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
        let (result_tx, result_rx): (Sender<BlockResult>, Receiver<BlockResult>) = bounded(max_inflight);

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
                                .send(BlockResult {
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

                    tx.send(BlockResult {
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
        let writer_handle = spawn_writer_thread(writer_input, result_rx);

        // ---- Wait ----
        // 先 join 全部线程再决定返回哪个错误：解码/写入失败会让别处的 channel
        // 关闭并产生 "channel closed" 次生错误，提前返回会把它当成根因。
        let mut thread_errors: Vec<FqcError> = Vec::new();
        let reader_read_ms = match reader_handle.join() {
            Ok(Ok(ms)) => Some(ms),
            Ok(Err(err)) => {
                thread_errors.push(err);
                None
            }
            Err(_) => {
                thread_errors.push(FqcError::Decompression("Reader thread panicked".to_string()));
                None
            }
        };
        for h in decomp_handles {
            match h.join() {
                Ok(Ok(())) => {}
                Ok(Err(err)) => thread_errors.push(err),
                Err(_) => thread_errors.push(FqcError::Decompression("Decompressor thread panicked".to_string())),
            }
        }
        let decode_ms = decode_ns.load(std::sync::atomic::Ordering::Relaxed) / 1_000_000;
        let writer = match writer_handle.join() {
            Ok(Ok(writer)) => Some(writer),
            Ok(Err(err)) => {
                thread_errors.push(err);
                None
            }
            Err(_) => {
                thread_errors.push(FqcError::Decompression("Writer thread panicked".to_string()));
                None
            }
        };
        if !thread_errors.is_empty() || reader_read_ms.is_none() || writer.is_none() {
            return Err(prefer_thread_error(thread_errors));
        }
        let reader_read_ms = reader_read_ms.expect("checked above");
        let writer = writer.expect("checked above");

        let elapsed = start.elapsed();
        self.stats = OperationStats {
            total_reads: writer.reads_written,
            total_bases: writer.total_bases,
            blocks: (end_block - start_block) as u64,
            input_bytes: file_size,
            output_bytes: writer.output_bytes,
            elapsed_seconds: elapsed.as_secs_f64(),
            reorder_map_written: false,
            corrupted_blocks: writer.corrupted_blocks,
            parse_ms: reader_read_ms,
            reorder_ms: 0,
            process_ms: decode_ms,
            write_ms: writer.write_ms,
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
            // 单块 ReadRecord 骨架不得超过单次分配预算：read_count 来自
            // 文件，旧路径在预算检查前直接按它分配（--memory-limit 16 下
            // 500k reads 的构造归档峰值超出约 35 MB）。
            max_block_decoded_bytes: reader.budget().max_alloc_bytes,
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
            writer_input: WriterPlan {
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
