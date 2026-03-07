// =============================================================================
// fqc-rust - Compression Pipeline
// =============================================================================
// 3-stage pipeline: Reader (serial) → Compressor (parallel) → Writer (serial)
// Uses bounded channels for backpressure control.
// =============================================================================

use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig, CompressedBlockData};
use crate::algo::global_analyzer::{GlobalAnalyzer, GlobalAnalyzerConfig};
use crate::error::{FqcError, Result};
use crate::fastq::parser::{open_fastq, open_fastq_paired, open_fastq_stdin};
use crate::format::{build_flags, GlobalHeader};
use crate::fqc_writer::FqcWriter;
use crate::types::*;

use super::{PipelineControl, PipelineStats, ProgressCallback, ReadChunk,
            DEFAULT_MAX_IN_FLIGHT_BLOCKS};

// =============================================================================
// CompressionPipelineConfig
// =============================================================================

/// Configuration for compression pipeline
#[derive(Clone)]
pub struct CompressionPipelineConfig {
    pub num_threads: usize,
    pub max_in_flight_blocks: usize,
    pub block_size: usize,
    pub read_length_class: ReadLengthClass,
    pub quality_mode: QualityMode,
    pub id_mode: IdMode,
    pub compression_level: CompressionLevel,
    pub enable_reorder: bool,
    pub save_reorder_map: bool,
    pub streaming_mode: bool,
    pub pe_layout: PeLayout,
    pub memory_limit_mb: usize,
}

impl Default for CompressionPipelineConfig {
    fn default() -> Self {
        Self {
            num_threads: 0,
            max_in_flight_blocks: DEFAULT_MAX_IN_FLIGHT_BLOCKS,
            block_size: DEFAULT_BLOCK_SIZE_SHORT,
            read_length_class: ReadLengthClass::Short,
            quality_mode: QualityMode::Lossless,
            id_mode: IdMode::Exact,
            compression_level: DEFAULT_COMPRESSION_LEVEL,
            enable_reorder: true,
            save_reorder_map: true,
            streaming_mode: false,
            pe_layout: PeLayout::Interleaved,
            memory_limit_mb: 8192,
        }
    }
}

impl CompressionPipelineConfig {
    pub fn effective_threads(&self) -> usize {
        if self.num_threads == 0 {
            num_cpus().max(1)
        } else {
            self.num_threads
        }
    }

    pub fn effective_block_size(&self) -> usize {
        if self.block_size > 0 {
            self.block_size
        } else {
            recommended_block_size(self.read_length_class)
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.block_size > 0 && self.block_size < super::MIN_BLOCK_SIZE {
            return Err(FqcError::InvalidArgument(
                format!("Block size {} is too small (min {})", self.block_size, super::MIN_BLOCK_SIZE)));
        }
        if self.block_size > super::MAX_BLOCK_SIZE {
            return Err(FqcError::InvalidArgument(
                format!("Block size {} is too large (max {})", self.block_size, super::MAX_BLOCK_SIZE)));
        }
        Ok(())
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

// =============================================================================
// Ordered compressed block for writer stage
// =============================================================================

struct OrderedBlock {
    chunk_id: u32,
    compressed: CompressedBlockData,
    is_last: bool,
}

// =============================================================================
// CompressionPipeline
// =============================================================================

pub struct CompressionPipeline {
    config: CompressionPipelineConfig,
    control: PipelineControl,
    stats: PipelineStats,
}

impl CompressionPipeline {
    pub fn new(config: CompressionPipelineConfig) -> Self {
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

    /// Run compression on a single-end input file
    pub fn run(
        &mut self,
        input_path: &str,
        output_path: &str,
        original_filename: &str,
        _progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        self.config.validate()?;
        let start = Instant::now();
        let threads = self.config.effective_threads();
        let block_size = self.config.effective_block_size();
        let output_path_owned = output_path.to_string();
        let original_filename_owned = original_filename.to_string();

        log::info!("Compression pipeline: {} threads, block_size={}", threads, block_size);

        // ---- Phase 1: Read all records (needed for global analysis) ----
        let all_reads = if input_path == "-" {
            let mut parser = open_fastq_stdin();
            parser.collect_all()?
        } else {
            let mut parser = open_fastq(input_path)?;
            parser.collect_all()?
        };

        if all_reads.is_empty() {
            return Err(FqcError::InvalidArgument("Input file is empty".to_string()));
        }

        let total_reads = all_reads.len();
        let input_bytes: usize = all_reads.iter()
            .map(|r| r.id.len() + r.sequence.len() + r.quality.len() + 4)
            .sum();

        log::info!("Read {} records ({} bytes)", total_reads, input_bytes);

        // ---- Phase 1b: Global analysis (reordering) ----
        let (ordered_reads, forward_map, reverse_map) = if self.config.enable_reorder && !self.config.streaming_mode {
            log::info!("Running global analysis...");
            let ga_config = GlobalAnalyzerConfig {
                reads_per_block: block_size,
                ..Default::default()
            };
            let sequences: Vec<String> = all_reads.iter()
                .map(|r| r.sequence.clone())
                .collect();
            let analyzer = GlobalAnalyzer::new(ga_config);
            let result = analyzer.analyze(&sequences)?;
            // reverse_map[archive_id] = original_id
            let ordered: Vec<ReadRecord> = result.reverse_map.iter()
                .map(|&orig_idx| all_reads[orig_idx as usize].clone())
                .collect();
            (ordered, Some(result.forward_map), Some(result.reverse_map))
        } else {
            (all_reads, None, None)
        };

        // ---- Phase 2: Pipeline compression ----
        let is_paired = false;
        let flags = build_flags(
            is_paired,
            !self.config.enable_reorder,
            self.config.quality_mode,
            self.config.id_mode,
            forward_map.is_some(),
            self.config.pe_layout,
            self.config.read_length_class,
            self.config.streaming_mode,
        );

        let compressor_config = BlockCompressorConfig {
            read_length_class: self.config.read_length_class,
            compression_level: self.config.compression_level,
            quality_mode: self.config.quality_mode,
            id_mode: self.config.id_mode,
            zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(
                self.config.compression_level),
            ..Default::default()
        };

        // Split reads into chunks
        let chunks: Vec<Vec<ReadRecord>> = ordered_reads
            .chunks(block_size)
            .map(|c| c.to_vec())
            .collect();
        let num_chunks = chunks.len();

        // Setup channels with bounded capacity for backpressure
        let max_inflight = self.config.max_in_flight_blocks;
        let (chunk_tx, chunk_rx): (Sender<ReadChunk>, Receiver<ReadChunk>) =
            bounded(max_inflight);
        let (block_tx, block_rx): (Sender<OrderedBlock>, Receiver<OrderedBlock>) =
            bounded(max_inflight);

        let control = self.control.clone();
        let compressor_config_arc = Arc::new(compressor_config.clone());

        // ---- Reader thread: send chunks ----
        let reader_control = control.clone();
        let reader_handle = thread::spawn(move || -> Result<()> {
            let mut start_read_id: u64 = 0;
            for (i, chunk_reads) in chunks.into_iter().enumerate() {
                if reader_control.is_cancelled() {
                    break;
                }
                let chunk = ReadChunk {
                    reads: chunk_reads,
                    chunk_id: i as u32,
                    start_read_id,
                    is_last: i + 1 == num_chunks,
                };
                start_read_id += chunk.size() as u64;
                chunk_tx.send(chunk).map_err(|_| {
                    FqcError::Compression("Reader: channel closed".to_string())
                })?;
            }
            Ok(())
        });

        // ---- Compressor threads (parallel) ----
        let num_compressor_threads = threads.max(1);
        let mut compressor_handles = Vec::new();

        for _ in 0..num_compressor_threads {
            let rx = chunk_rx.clone();
            let tx = block_tx.clone();
            let cfg = compressor_config_arc.clone();
            let ctrl = control.clone();

            let handle = thread::spawn(move || -> Result<()> {
                let compressor = BlockCompressor::new((*cfg).clone());
                for chunk in rx.iter() {
                    if ctrl.is_cancelled() { break; }

                    let compressed = compressor.compress(&chunk.reads, chunk.chunk_id)?;
                    ctrl.add_reads(chunk.reads.len() as u64);

                    tx.send(OrderedBlock {
                        chunk_id: chunk.chunk_id,
                        compressed,
                        is_last: chunk.is_last,
                    }).map_err(|_| {
                        FqcError::Compression("Compressor: channel closed".to_string())
                    })?;
                }
                Ok(())
            });
            compressor_handles.push(handle);
        }
        // Drop extra sender so writer knows when all compressors are done
        drop(chunk_rx);
        drop(block_tx);

        // ---- Writer thread: receive and write in order ----
        let writer_control = control.clone();
        let writer_handle = thread::spawn(move || -> Result<u64> {
            let mut writer = FqcWriter::create(&output_path_owned)?;

            let gh = GlobalHeader::new(flags, total_reads as u64, &original_filename_owned, 
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs()).unwrap_or(0));
            writer.write_global_header(&gh)?;

            // Collect blocks and write in order
            let mut pending: std::collections::BTreeMap<u32, OrderedBlock> = std::collections::BTreeMap::new();
            let mut next_expected: u32 = 0;
            let mut total_output_bytes: u64 = 0;

            for ordered_block in block_rx.iter() {
                if writer_control.is_cancelled() { break; }
                pending.insert(ordered_block.chunk_id, ordered_block);

                // Write all consecutive blocks starting from next_expected
                while let Some(block) = pending.remove(&next_expected) {
                    writer.write_block(&block.compressed)?;
                    total_output_bytes += block.compressed.total_compressed_size() as u64;
                    next_expected += 1;
                }
            }

            // Write remaining pending blocks
            while let Some((&id, _)) = pending.iter().next() {
                if let Some(block) = pending.remove(&id) {
                    writer.write_block(&block.compressed)?;
                    total_output_bytes += block.compressed.total_compressed_size() as u64;
                }
            }

            // Write reorder map if present
            if let (Some(fwd), Some(rev)) = (&forward_map, &reverse_map) {
                writer.write_reorder_map(fwd, rev)?;
            }

            writer.finalize()?;
            Ok(total_output_bytes)
        });

        // ---- Wait for all stages ----
        reader_handle.join().map_err(|_| FqcError::Compression("Reader thread panicked".to_string()))??;
        for h in compressor_handles {
            h.join().map_err(|_| FqcError::Compression("Compressor thread panicked".to_string()))??;
        }
        let output_bytes = writer_handle.join()
            .map_err(|_| FqcError::Compression("Writer thread panicked".to_string()))??;

        // ---- Collect stats ----
        let elapsed = start.elapsed();
        self.stats = PipelineStats {
            total_reads: total_reads as u64,
            total_blocks: num_chunks as u32,
            input_bytes: input_bytes as u64,
            output_bytes,
            processing_time_ms: elapsed.as_millis() as u64,
            peak_memory_bytes: 0,
            threads_used: threads,
        };

        log::info!(
            "Compression complete: {} reads, {} blocks, {:.2}x ratio, {:.1} MB/s",
            self.stats.total_reads,
            self.stats.total_blocks,
            if self.stats.compression_ratio() > 0.0 { 1.0 / self.stats.compression_ratio() } else { 0.0 },
            self.stats.throughput_mbps(),
        );

        Ok(())
    }

    /// Run compression on paired-end input files
    pub fn run_paired(
        &mut self,
        input1_path: &str,
        input2_path: &str,
        output_path: &str,
        original_filename: &str,
        pe_layout: PeLayout,
        _progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        self.config.validate()?;
        let start = Instant::now();

        log::info!("Reading paired-end files: {} + {}", input1_path, input2_path);

        let mut pe_reader = open_fastq_paired(input1_path, input2_path)?;
        let all_reads = match pe_layout {
            PeLayout::Interleaved => pe_reader.collect_all_interleaved()?,
            PeLayout::Consecutive => pe_reader.collect_all_consecutive()?,
        };

        if all_reads.is_empty() {
            return Err(FqcError::InvalidArgument("Input files are empty".to_string()));
        }

        // Store reads, then run pipeline (reuse single-end logic for Phase 2)
        let total_reads = all_reads.len();
        let input_bytes: usize = all_reads.iter()
            .map(|r| r.id.len() + r.sequence.len() + r.quality.len() + 4)
            .sum();
        let block_size = self.config.effective_block_size();
        let threads = self.config.effective_threads();

        let (ordered_reads, forward_map, reverse_map) = if self.config.enable_reorder && !self.config.streaming_mode {
            let ga_config = GlobalAnalyzerConfig {
                reads_per_block: block_size,
                ..Default::default()
            };
            let sequences: Vec<String> = all_reads.iter()
                .map(|r| r.sequence.clone())
                .collect();
            let analyzer = GlobalAnalyzer::new(ga_config);
            let result = analyzer.analyze(&sequences)?;
            let ordered: Vec<ReadRecord> = result.reverse_map.iter()
                .map(|&orig_idx| all_reads[orig_idx as usize].clone())
                .collect();
            (ordered, Some(result.forward_map), Some(result.reverse_map))
        } else {
            (all_reads, None, None)
        };

        let flags = build_flags(
            true,
            !self.config.enable_reorder,
            self.config.quality_mode,
            self.config.id_mode,
            forward_map.is_some(),
            pe_layout,
            self.config.read_length_class,
            self.config.streaming_mode,
        );

        let compressor_config = BlockCompressorConfig {
            read_length_class: self.config.read_length_class,
            compression_level: self.config.compression_level,
            quality_mode: self.config.quality_mode,
            id_mode: self.config.id_mode,
            zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(
                self.config.compression_level),
            ..Default::default()
        };

        let compressor = BlockCompressor::new(compressor_config);
        let mut writer = FqcWriter::create(output_path)?;

        let gh = GlobalHeader::new(flags, total_reads as u64, original_filename,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or(0));
        writer.write_global_header(&gh)?;

        let mut output_bytes: u64 = 0;
        for (i, chunk) in ordered_reads.chunks(block_size).enumerate() {
            let compressed = compressor.compress(chunk, i as u32)?;
            output_bytes += compressed.total_compressed_size() as u64;
            writer.write_block(&compressed)?;
        }

        if let (Some(fwd), Some(rev)) = (&forward_map, &reverse_map) {
            writer.write_reorder_map(fwd, rev)?;
        }

        writer.finalize()?;

        let elapsed = start.elapsed();
        self.stats = PipelineStats {
            total_reads: total_reads as u64,
            total_blocks: total_reads.div_ceil(block_size) as u32,
            input_bytes: input_bytes as u64,
            output_bytes,
            processing_time_ms: elapsed.as_millis() as u64,
            peak_memory_bytes: 0,
            threads_used: threads,
        };

        Ok(())
    }

    pub fn cancel(&self) {
        self.control.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.control.is_cancelled()
    }
}
