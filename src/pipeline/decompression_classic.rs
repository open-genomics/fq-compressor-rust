// =============================================================================
// fqc-rust - Sequential Decompression (classic path)
// =============================================================================
// The decompression orchestration that predates the threaded pipeline:
// serial block streaming, batched-parallel decompression, original-order
// restore, and split paired-end output. Since path convergence this is the
// single home of that orchestration; `commands/decompress.rs` only validates
// options and routes here or to `DecompressionPipeline`.
// =============================================================================

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use crate::archive::format::{get_id_mode, get_quality_mode, get_read_length_class};
use crate::archive::reader::FqcReader;
use crate::block::{DecodedBlock, EncodedBlock};
use crate::error::{FqcError, Result};
use crate::memory::DecodeBudget;
use crate::pipeline::decompression::DecompressionConfig;
use crate::pipeline::decompression_writer::{placeholder_record, OutputWriters};
use crate::stats::OperationStats;
use crate::types::*;
use rayon::prelude::*;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// SequentialDecompressor
// =============================================================================

pub struct SequentialDecompressor {
    config: DecompressionConfig,
}

impl SequentialDecompressor {
    pub fn new(config: DecompressionConfig) -> Self {
        Self { config }
    }

    pub fn run(&mut self, input_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> Result<OperationStats> {
        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();
        let start = Instant::now();
        let mut stats = OperationStats::default();

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
            let avg_length = reader.memory_estimate_read_length() as u64;
            budget.check_original_order_peak(
                reader.total_read_count(),
                reader.max_block_compressed_size(),
                avg_length,
            )?;
            reader.load_reorder_map()?;
            log::info!("Reorder map loaded");
        }

        // Build block compressor config from global header
        let archive_flags = reader.global_header.flags;
        let block_config = BlockCompressorConfig {
            read_length_class: get_read_length_class(archive_flags),
            quality_mode: get_quality_mode(archive_flags),
            id_mode: get_id_mode(archive_flags),
            ..Default::default()
        };

        let mut compressor = BlockCompressor::new(block_config.clone());

        let total_archive_reads = reader.total_read_count();
        let mut output = OutputWriters::buffered(
            output_path,
            self.config.force_overwrite,
            self.config.split_pe,
            archive_flags,
        )?;

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
                        stats.blocks += 1;
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
            stats.blocks
        );
        stats.elapsed_seconds = start.elapsed().as_secs_f64();
        Ok(stats)
    }

    /// Parallel decompression: read blocks sequentially, decompress in parallel batches, write sequentially.
    fn run_parallel(
        &mut self,
        reader: &mut FqcReader,
        block_config: &BlockCompressorConfig,
        block_count: usize,
        total_archive_reads: u64,
        stats: &mut OperationStats,
        output: &mut OutputWriters,
    ) -> Result<()> {
        log::info!("Using parallel decompression ({} blocks)", block_count);

        let block_hint = reader.max_block_compressed_size();
        let max_block_reads = reader
            .block_index
            .entries
            .iter()
            .map(|entry| entry.read_count as usize)
            .max()
            .unwrap_or(1_000);
        let worker_bytes =
            block_config.estimated_worker_bytes(max_block_reads, reader.memory_estimate_read_length()) as u64;
        let max_workers = reader
            .budget()
            .max_workers_for_budget(worker_bytes, reader.budget().worker_budget_bytes());
        let batch_size = reader
            .budget()
            .parallel_batch_size(self.config.effective_threads(), block_hint)?
            .min(max_workers)
            .min(self.config.effective_threads())
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
            let mut block_data_vec: Vec<(u32, EncodedBlock)> = Vec::with_capacity(batch_end - block_start);
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
            let results: Vec<std::result::Result<(u32, DecodedBlock), (u32, String, u32)>> = block_data_vec
                .into_par_iter()
                .map(|(bid, bd)| {
                    let mut comp = BlockCompressor::new((*cfg).clone());
                    match comp.decompress_block(&bd) {
                        Ok(dec) => Ok((bid, dec)),
                        Err(e) => Err((bid, format!("{e}"), bd.meta.read_count)),
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
                        stats.blocks += 1;
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
        stats: &mut OperationStats,
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
        stats: &mut OperationStats,
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
                            all_reads.extend((0..entry.read_count as usize).map(|read_idx| {
                                placeholder_record(
                                    self.config.corrupted_placeholder.as_deref().unwrap_or("N"),
                                    block_id as u32,
                                    read_idx,
                                )
                            }));
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
                    stats.blocks += 1;
                }
                Err(e) => {
                    if self.config.skip_corrupted {
                        if let Some(entry) = reader.block_index.entries.get(block_id) {
                            all_reads.extend((0..entry.read_count as usize).map(|read_idx| {
                                placeholder_record(
                                    self.config.corrupted_placeholder.as_deref().unwrap_or("N"),
                                    block_id as u32,
                                    read_idx,
                                )
                            }));
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
        stats: &mut OperationStats,
    ) -> Result<()> {
        for local_idx in 0..read_count as usize {
            let read_idx = *global_read_idx;
            *global_read_idx += 1;
            let placeholder = placeholder_record(
                self.config.corrupted_placeholder.as_deref().unwrap_or("N"),
                block_id,
                local_idx,
            );
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
        stats: &mut OperationStats,
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
