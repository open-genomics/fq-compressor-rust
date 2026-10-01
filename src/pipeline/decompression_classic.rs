// =============================================================================
// fqc-rust - Sequential Decompression (classic path)
// =============================================================================
// The decompression orchestration that predates the threaded pipeline:
// serial block streaming, batched-parallel decompression, original-order
// restore, and split paired-end output. Since path convergence this is the
// single home of that orchestration; `commands/decompress.rs` only validates
// options and routes here or to `DecompressionPipeline`.
// =============================================================================

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig, DecompressedBlockData};
use crate::archive::format::{flags, get_id_mode, get_pe_layout, get_quality_mode, get_read_length_class};
use crate::archive::reader::FqcReader;
use crate::archive::traits::BlockData;
use crate::error::{FqcError, Result};
use crate::fastq::parser::write_record as write_fastq_record;
use crate::io::{commit_split, OutputTransaction};
use crate::memory_budget::DecodeBudget;
use crate::pipeline::decompression::DecompressionPipelineConfig;
use crate::pipeline::PipelineStats;
use crate::types::*;
use rayon::prelude::*;
use std::io::{BufWriter, Write};
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// Output writers (single file/stdout or split R1/R2)
// =============================================================================

enum OutputWriters {
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
        Ok(read.id.len() as u64 + comment_bytes as u64 + read.sequence.len() as u64 + read.quality.len() as u64 + 4)
    }
}

fn derive_split_output_paths(output_path: &str) -> (String, String) {
    if let Some(dot_pos) = output_path.rfind('.') {
        let (base, ext) = output_path.split_at(dot_pos);
        (format!("{base}_R1{ext}"), format!("{base}_R2{ext}"))
    } else {
        (format!("{output_path}_R1"), format!("{output_path}_R2"))
    }
}

fn placeholder_record(config: &DecompressionPipelineConfig, block_id: u32, read_idx: usize) -> ReadRecord {
    let placeholder_seq = config.corrupted_placeholder.clone().unwrap_or_else(|| "N".to_string());
    let quality = "!".repeat(placeholder_seq.len());
    ReadRecord {
        id: format!("corrupted_block{}_read{}", block_id, read_idx),
        comment: String::new(),
        sequence: placeholder_seq,
        quality,
    }
}

// =============================================================================
// SequentialDecompressor
// =============================================================================

pub struct SequentialDecompressor {
    config: DecompressionPipelineConfig,
}

impl SequentialDecompressor {
    pub fn new(config: DecompressionPipelineConfig) -> Self {
        Self { config }
    }

    pub fn run(&mut self, input_path: &str, output_path: &str) -> Result<PipelineStats> {
        let start = Instant::now();
        let mut stats = PipelineStats::default();

        let budget = DecodeBudget::resolve(self.config.memory_limit_mb);
        if budget.automatic {
            log::info!(
                "Decode memory budget: automatic {} MB (not unlimited)",
                budget.limit_bytes / (1024 * 1024)
            );
        } else {
            log::info!("Decode memory budget: {} MB", budget.limit_bytes / (1024 * 1024));
        }

        // Open archive
        let mut reader = FqcReader::open_with_budget(input_path, budget.clone())?;

        log::info!(
            "Archive: {} reads, {} blocks",
            reader.total_read_count(),
            reader.block_count()
        );

        // Load reorder map if needed
        if self.config.original_order {
            if !reader.has_reorder_map() {
                return Err(FqcError::Format(
                    "Original order requested but no reorder map present".to_string(),
                ));
            }
            // Fail before creating outputs if peak estimate exceeds budget.
            budget.check_original_order_peak(reader.total_read_count(), reader.max_block_compressed_size(), 256)?;
            reader.load_reorder_map()?;
            log::info!("Reorder map loaded");
        }

        // Build block compressor config from global header
        let flags = reader.global_header.flags;
        let block_config = BlockCompressorConfig {
            read_length_class: get_read_length_class(flags),
            quality_mode: get_quality_mode(flags),
            id_mode: get_id_mode(flags),
            ..Default::default()
        };

        let mut compressor = BlockCompressor::new(block_config.clone());

        let is_paired = (flags & flags::IS_PAIRED) != 0;
        let total_archive_reads = reader.total_read_count();

        // Open output
        let mut output = if self.config.split_pe {
            if !is_paired {
                return Err(FqcError::InvalidArgument(
                    "--split-pe requires a paired-end archive".to_string(),
                ));
            }
            if output_path == "-" {
                return Err(FqcError::InvalidArgument(
                    "--split-pe cannot be used with stdout output".to_string(),
                ));
            }

            let (r1_path, r2_path) = derive_split_output_paths(output_path);
            let mut r1_tx = OutputTransaction::begin(&r1_path, self.config.force_overwrite)?;
            let mut r2_tx = OutputTransaction::begin(&r2_path, self.config.force_overwrite)?;
            let pe_layout = get_pe_layout(flags);
            OutputWriters::Split {
                r1: Box::new(BufWriter::new(r1_tx.take_file()?)),
                r2: Box::new(BufWriter::new(r2_tx.take_file()?)),
                r1_tx,
                r2_tx,
                pe_layout,
                streaming_block_layout: (flags & flags::STREAMING_MODE) != 0 && pe_layout == PeLayout::Consecutive,
            }
        } else if output_path == "-" {
            OutputWriters::Single {
                writer: Box::new(std::io::stdout()),
                tx: None,
            }
        } else {
            let mut tx = OutputTransaction::begin(output_path, self.config.force_overwrite)?;
            OutputWriters::Single {
                writer: Box::new(BufWriter::new(tx.take_file()?)),
                tx: Some(tx),
            }
        };

        // Process blocks
        let block_count = reader.block_count();

        if self.config.original_order && reader.reorder_forward.is_some() {
            // Original order mode: buffer all reads, then output in original order
            self.run_original_order(
                &mut reader,
                &mut compressor,
                block_count,
                total_archive_reads,
                &mut stats,
                &mut output,
            )?;
        } else if block_count > 1 && self.config.num_threads != 1 {
            // Parallel decompression: read blocks sequentially, decompress in parallel, write sequentially
            self.run_parallel(
                &mut reader,
                &block_config,
                block_count,
                total_archive_reads,
                &mut stats,
                &mut output,
            )?;
        } else {
            // Normal mode: stream blocks directly to output
            let mut global_read_idx = 0u64;
            for block_id in 0..block_count {
                log::debug!("Processing block {}/{}", block_id + 1, block_count);

                match self.process_block(
                    &mut reader,
                    &mut compressor,
                    block_id as u32,
                    total_archive_reads,
                    &mut global_read_idx,
                    &mut stats,
                    &mut output,
                ) {
                    Ok(()) => {
                        stats.total_blocks += 1;
                    }
                    Err(e) => {
                        if self.config.skip_corrupted {
                            if let Some(entry) = reader.block_index.entries.get(block_id) {
                                self.emit_corrupted_placeholders(
                                    &mut output,
                                    block_id as u32,
                                    entry.read_count,
                                    total_archive_reads,
                                    &mut global_read_idx,
                                    &mut stats,
                                )?;
                            }
                            log::warn!("Block {} failed, skipping: {}", block_id, e);
                            stats.corrupted_blocks += 1;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }

        // Get input file size
        if let Ok(meta) = std::fs::metadata(input_path) {
            stats.input_bytes = meta.len();
        }

        output.commit()?;

        log::info!(
            "Decompression complete: {} reads, {} blocks",
            stats.total_reads,
            stats.total_blocks
        );
        stats.processing_time_ms = start.elapsed().as_millis() as u64;
        Ok(stats)
    }

    /// Parallel decompression: read blocks sequentially, decompress in parallel batches, write sequentially.
    fn run_parallel(
        &mut self,
        reader: &mut FqcReader,
        block_config: &BlockCompressorConfig,
        block_count: usize,
        total_archive_reads: u64,
        stats: &mut PipelineStats,
        output: &mut OutputWriters,
    ) -> Result<()> {
        log::info!("Using parallel decompression ({} blocks)", block_count);

        let block_hint = reader.max_block_compressed_size();
        let batch_size = reader
            .budget()
            .parallel_batch_size(self.config.num_threads, block_hint)?
            .min(block_count)
            .max(1);
        let config = Arc::new(block_config.clone());
        let skip_corrupted = self.config.skip_corrupted;

        let mut global_read_idx = 0u64;
        let mut block_start = 0usize;

        while block_start < block_count {
            let batch_end = (block_start + batch_size).min(block_count);

            // Phase 1: Read block data sequentially
            let t_phase = Instant::now();
            let mut block_data_vec: Vec<(u32, BlockData)> = Vec::with_capacity(batch_end - block_start);
            let mut phase1_failures: Vec<(u32, String, u32)> = Vec::new();
            for block_id in block_start..batch_end {
                match reader.read_block(block_id as u32) {
                    Ok(bd) => block_data_vec.push((block_id as u32, bd)),
                    Err(e) => {
                        if skip_corrupted {
                            let read_count = reader
                                .block_index
                                .entries
                                .get(block_id)
                                .map(|entry| entry.read_count)
                                .unwrap_or(0);
                            phase1_failures.push((block_id as u32, e.to_string(), read_count));
                            log::warn!("Block {} read failed, skipping: {}", block_id, e);
                            stats.corrupted_blocks += 1;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            stats.parse_ms += t_phase.elapsed().as_millis() as u64;

            // Phase 2: Decompress in parallel
            let t_phase = Instant::now();
            let cfg = Arc::clone(&config);
            let results: Vec<std::result::Result<(u32, DecompressedBlockData), (u32, String, u32)>> = block_data_vec
                .into_par_iter()
                .map(|(bid, bd)| {
                    let mut comp = BlockCompressor::new((*cfg).clone());
                    match comp.decompress_block(&bd) {
                        Ok(dec) => Ok((bid, dec)),
                        Err(e) => Err((bid, format!("{e}"), bd.header.uncompressed_count)),
                    }
                })
                .collect();
            stats.process_ms += t_phase.elapsed().as_millis() as u64;

            // Phase 3: Write results sequentially (sorted by block_id)
            let t_phase = Instant::now();
            let mut sorted: Vec<_> = results;
            sorted.extend(phase1_failures.into_iter().map(Err));
            sorted.sort_by_key(|r| match r {
                Ok((bid, _)) => *bid,
                Err((bid, _, _)) => *bid,
            });

            for result in sorted {
                match result {
                    Ok((_bid, decompressed)) => {
                        let block_read_count = decompressed.reads.len() as u64;
                        for (local_idx, read) in decompressed.reads.iter().enumerate() {
                            self.emit_read(
                                stats,
                                output,
                                read,
                                global_read_idx,
                                total_archive_reads,
                                Some((local_idx as u64, block_read_count)),
                            )?;
                            global_read_idx += 1;
                        }
                        stats.total_blocks += 1;
                    }
                    Err((bid, msg, read_count)) => {
                        if skip_corrupted {
                            self.emit_corrupted_placeholders(
                                output,
                                bid,
                                read_count,
                                total_archive_reads,
                                &mut global_read_idx,
                                stats,
                            )?;
                            log::warn!("Block {} decompress failed, skipping: {}", bid, msg);
                            stats.corrupted_blocks += 1;
                        } else {
                            return Err(FqcError::Decompression(format!("Block {} failed: {}", bid, msg)));
                        }
                    }
                }
            }
            stats.write_ms += t_phase.elapsed().as_millis() as u64;

            block_start = batch_end;
        }

        Ok(())
    }

    fn process_block(
        &mut self,
        reader: &mut FqcReader,
        compressor: &mut BlockCompressor,
        block_id: u32,
        total_archive_reads: u64,
        global_read_idx: &mut u64,
        stats: &mut PipelineStats,
        output: &mut OutputWriters,
    ) -> Result<()> {
        // Serial path: time each phase so single-block files still report
        // stage timings (run_parallel only covers the multi-block path).
        let t_read = Instant::now();
        let block_data = reader.read_block(block_id)?;
        let parse_ms = t_read.elapsed().as_millis() as u64;

        let t_proc = Instant::now();
        let decompressed = compressor.decompress_block(&block_data)?;
        let process_ms = t_proc.elapsed().as_millis() as u64;

        let t_write = Instant::now();
        let block_read_count = decompressed.reads.len() as u64;
        for (local_idx, read) in decompressed.reads.iter().enumerate() {
            let read_idx = *global_read_idx;
            *global_read_idx += 1;
            self.emit_read(
                stats,
                output,
                read,
                read_idx,
                total_archive_reads,
                Some((local_idx as u64, block_read_count)),
            )?;
        }
        let write_ms = t_write.elapsed().as_millis() as u64;

        stats.parse_ms += parse_ms;
        stats.process_ms += process_ms;
        stats.write_ms += write_ms;

        Ok(())
    }

    /// Decompress all blocks, then output reads in original order using the reorder map.
    fn run_original_order(
        &mut self,
        reader: &mut FqcReader,
        compressor: &mut BlockCompressor,
        block_count: usize,
        total_archive_reads: u64,
        stats: &mut PipelineStats,
        output: &mut OutputWriters,
    ) -> Result<()> {
        log::info!("Restoring original read order...");

        let forward_map = reader
            .reorder_forward
            .clone()
            .ok_or_else(|| FqcError::Format("Forward reorder map missing".to_string()))?;
        let total_reads = forward_map.len();

        // Buffer all reads in archive order
        let mut all_reads: Vec<ReadRecord> = Vec::with_capacity(total_reads);

        for block_id in 0..block_count {
            let t_read = Instant::now();
            let block_data = match reader.read_block(block_id as u32) {
                Ok(block_data) => block_data,
                Err(e) => {
                    if self.config.skip_corrupted {
                        if let Some(entry) = reader.block_index.entries.get(block_id) {
                            all_reads.extend(
                                (0..entry.read_count as usize)
                                    .map(|read_idx| placeholder_record(&self.config, block_id as u32, read_idx)),
                            );
                        }
                        log::warn!("Block {} failed, skipping: {}", block_id, e);
                        stats.corrupted_blocks += 1;
                        continue;
                    }
                    return Err(e);
                }
            };
            stats.parse_ms += t_read.elapsed().as_millis() as u64;

            let t_proc = Instant::now();
            match compressor.decompress_block(&block_data) {
                Ok(decompressed) => {
                    all_reads.extend(decompressed.reads);
                    stats.total_blocks += 1;
                }
                Err(e) => {
                    if self.config.skip_corrupted {
                        if let Some(entry) = reader.block_index.entries.get(block_id) {
                            all_reads.extend(
                                (0..entry.read_count as usize)
                                    .map(|read_idx| placeholder_record(&self.config, block_id as u32, read_idx)),
                            );
                        }
                        log::warn!("Block {} failed, skipping: {}", block_id, e);
                        stats.corrupted_blocks += 1;
                    } else {
                        return Err(e);
                    }
                }
            }
            stats.process_ms += t_proc.elapsed().as_millis() as u64;
        }

        // Reorder: forward_map[original_id] = archive_id
        // So to output in original order, iterate original_id 0..N
        // and output all_reads[forward_map[original_id]]
        let t_write = Instant::now();
        for (original_id, &fwd) in forward_map.iter().enumerate().take(total_reads) {
            let archive_id = fwd as usize;
            if archive_id >= all_reads.len() {
                log::warn!(
                    "Invalid reorder map reference: original_id {} -> archive_id {} (max {})",
                    original_id,
                    archive_id,
                    all_reads.len() - 1
                );
                continue;
            }

            let read = &all_reads[archive_id];
            self.emit_read(stats, output, read, original_id as u64, total_archive_reads, None)?;
        }
        stats.write_ms += t_write.elapsed().as_millis() as u64;

        Ok(())
    }

    fn emit_corrupted_placeholders(
        &mut self,
        output: &mut OutputWriters,
        block_id: u32,
        read_count: u32,
        total_archive_reads: u64,
        global_read_idx: &mut u64,
        stats: &mut PipelineStats,
    ) -> Result<()> {
        for local_idx in 0..read_count as usize {
            let read_idx = *global_read_idx;
            *global_read_idx += 1;
            let placeholder = placeholder_record(&self.config, block_id, local_idx);
            self.emit_read(
                stats,
                output,
                &placeholder,
                read_idx,
                total_archive_reads,
                Some((local_idx as u64, read_count as u64)),
            )?;
        }
        Ok(())
    }

    fn emit_read(
        &mut self,
        stats: &mut PipelineStats,
        output: &mut OutputWriters,
        read: &ReadRecord,
        zero_based_read_idx: u64,
        total_archive_reads: u64,
        block_local_idx: Option<(u64, u64)>,
    ) -> Result<()> {
        let current_id = zero_based_read_idx + 1;

        if self.config.range_end > 0 {
            if current_id < self.config.range_start || current_id > self.config.range_end {
                return Ok(());
            }
        } else if self.config.range_start > 0 && current_id < self.config.range_start {
            return Ok(());
        }

        let bytes_written = output.write_record(
            read,
            self.config.header_only,
            zero_based_read_idx,
            total_archive_reads,
            block_local_idx,
        )?;
        stats.total_reads += 1;
        stats.total_bases += read.sequence.len() as u64;
        stats.output_bytes += bytes_written;
        Ok(())
    }
}
