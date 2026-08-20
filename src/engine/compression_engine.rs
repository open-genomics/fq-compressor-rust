// =============================================================================
// fqc-rust - Compression Engine
// =============================================================================
//! Compression execution engine and outcome types.
//!
//! This module provides the core compression engine abstraction and
//! the outcome type that captures compression results.

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig, CompressedBlockData};
use crate::algo::global_analyzer::{GlobalAnalyzer, GlobalAnalyzerConfig};
use crate::archive::format::{build_flags, GlobalHeader};
use crate::engine::compression_request::{CompressionExecutionMode, CompressionRequest};
use crate::error::{FqcError, Result};
use crate::fastq::parser::{open_fastq, open_fastq_interleaved, open_fastq_paired, open_fastq_stdin};
use crate::io::begin_fqc_writer;
use crate::types::*;
use rayon::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_LENGTH_SAMPLE_READS: usize = 4_096;

/// Processing statistics from a compression operation.
///
/// This type captures the core metrics from compression,
/// suitable for handoff to the command layer for reporting.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProcessingStats {
    /// Total reads compressed
    pub total_reads: u64,
    /// Total bases compressed
    pub total_bases: u64,
    /// Input bytes read
    pub input_bytes: u64,
    /// Output bytes written
    pub output_bytes: u64,
    /// Number of blocks written
    pub blocks_written: u64,
    /// Elapsed time in seconds
    pub elapsed_seconds: f64,
    /// Stage timings (ms). Serial stages are wall-clock; process_ms
    /// aggregates parallel worker time across threads.
    pub parse_ms: u64,
    pub reorder_ms: u64,
    pub process_ms: u64,
    pub write_ms: u64,
}

impl ProcessingStats {
    /// Compute compression ratio.
    pub fn compression_ratio(&self) -> f64 {
        if self.output_bytes == 0 {
            return 0.0;
        }
        self.input_bytes as f64 / self.output_bytes as f64
    }

    /// Compute bits per base.
    pub fn bits_per_base(&self) -> f64 {
        if self.total_bases == 0 {
            return 0.0;
        }
        (self.output_bytes as f64 * 8.0) / self.total_bases as f64
    }
}

/// Outcome of a compression operation.
///
/// This type captures the results and metadata from a completed
/// compression operation, including detected parameters and statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionOutcome {
    /// Execution mode used
    pub mode: CompressionExecutionMode,
    /// Detected read length class
    pub detected_read_length_class: ReadLengthClass,
    /// Whether a reorder map was written
    pub reorder_map_written: bool,
    /// Number of blocks written
    pub blocks_written: usize,
    /// Total reads compressed
    pub reads_compressed: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Compression ratio
    pub compression_ratio: f64,
    /// Processing statistics for handoff to command layer
    pub stats: ProcessingStats,
}

impl CompressionOutcome {
    /// Create a minimal outcome for testing purposes.
    ///
    /// This method provides a simple way to construct outcomes in tests
    /// with only the essential fields populated.
    pub fn new_for_tests(
        mode: CompressionExecutionMode,
        detected_read_length_class: ReadLengthClass,
        reorder_map_written: bool,
        blocks_written: usize,
    ) -> Self {
        Self {
            mode,
            detected_read_length_class,
            reorder_map_written,
            blocks_written,
            reads_compressed: 0,
            bytes_read: 0,
            bytes_written: 0,
            compression_ratio: 0.0,
            stats: ProcessingStats::default(),
        }
    }
}

/// Compression execution engine.
///
/// This engine provides the centralized orchestration logic for
/// all compression modes (archive, streaming, pipeline).
#[derive(Default)]
pub struct CompressionEngine;

impl CompressionEngine {
    /// Create a new compression engine.
    pub fn new() -> Self {
        Self
    }

    /// Execute a compression request and return the outcome.
    ///
    /// This is the main entry point for compression operations.
    /// It dispatches to the appropriate mode-specific handler based
    /// on the request's execution mode.
    pub fn run(&self, request: CompressionRequest) -> Result<CompressionOutcome> {
        match request.mode {
            CompressionExecutionMode::Archive => self.run_archive(request),
            CompressionExecutionMode::Streaming => self.run_streaming(request),
            CompressionExecutionMode::Pipeline => self.run_pipeline(request),
        }
    }

    /// Execute archive mode compression.
    ///
    /// This method implements the full archive compression flow:
    /// 1. Read all input records
    /// 2. Detect read length class
    /// 3. Perform global analysis (reordering)
    /// 4. Compress blocks in parallel
    /// 5. Write archive with optional reorder map
    #[allow(clippy::needless_pass_by_value)]
    fn run_archive(&self, request: CompressionRequest) -> Result<CompressionOutcome> {
        let input = request.input.resolve();

        let t_parse = std::time::Instant::now();
        log::info!("Reading input file: {}", input.primary_path);
        let records = if let Some(path2) = input.secondary_path.as_deref() {
            Self::read_all_paired_records(
                &input.primary_path,
                path2,
                input.archive_layout,
                request.memory_limit_mb,
            )?
        } else if input.is_interleaved {
            Self::read_all_interleaved_records(&input.primary_path, input.archive_layout, request.memory_limit_mb)?
        } else {
            Self::read_all_records(&input.primary_path, request.memory_limit_mb)?
        };

        if records.is_empty() {
            return Err(FqcError::InvalidArgument(
                "Input file contains no FASTQ records".to_string(),
            ));
        }

        let parse_ms = t_parse.elapsed().as_millis() as u64;

        let total_bases: u64 = records.iter().map(|r| r.sequence.len() as u64).sum();
        let total_reads = records.len() as u64;
        log::info!("Loaded {} reads ({} bases)", records.len(), total_bases);

        // Detect read length class
        let length_stats = Self::length_stats_from_records_sampled(&records, request.scan_all_lengths);
        let effective_length_class = Self::effective_length_class(request.requested_read_length_class, &length_stats);

        // Adjust parameters based on length class
        let block_size = Self::effective_block_size(
            request.block_size,
            effective_length_class,
            &length_stats,
            request.max_block_bases,
        );
        let enable_reorder =
            request.enable_reorder && !input.is_paired && effective_length_class == ReadLengthClass::Short;

        log::info!("Read length class: {}", effective_length_class.as_str());
        log::info!(
            "Length detection: sample={} avg={}bp median={}bp max={}bp",
            length_stats.sample_size,
            length_stats.avg_length,
            length_stats.median_length,
            length_stats.max_length
        );
        log::info!("Block size: {}", block_size);
        log::info!("Reordering: {}", enable_reorder);

        // Phase 1: Global analysis (reordering)
        let t_reorder = std::time::Instant::now();
        let sequences: Vec<String> = records.iter().map(|r| r.sequence.clone()).collect();

        let analyzer_config = GlobalAnalyzerConfig {
            reads_per_block: block_size,
            enable_reorder,
            read_length_class: Some(effective_length_class),
            ..Default::default()
        };

        let analyzer = GlobalAnalyzer::new(analyzer_config);
        let analysis = analyzer.analyze(&sequences)?;

        log::info!(
            "Analysis: {} blocks, reordering={}",
            analysis.num_blocks,
            analysis.reordering_performed
        );

        // Phase 2: Write FQC archive (temp file; commit after finalize)
        let (mut writer, output_tx) = begin_fqc_writer(&request.output_path, request.force_overwrite)?;

        // Build flags
        let flags = build_flags(
            input.is_paired,
            !analysis.reordering_performed,
            request.quality_mode,
            request.id_mode,
            analysis.reordering_performed,
            input.archive_layout,
            effective_length_class,
            false, // not streaming
        );

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let input_filename = std::path::Path::new(&input.primary_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let global_header = GlobalHeader::new(flags, records.len() as u64, input_filename, timestamp);
        writer.write_global_header(&global_header)?;

        // Block compressor config
        let block_config = std::sync::Arc::new(BlockCompressorConfig {
            read_length_class: effective_length_class,
            quality_mode: request.quality_mode,
            id_mode: request.id_mode,
            zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(request.level),
            ..Default::default()
        });

        // Extract block read sets
        let block_read_sets: Vec<(u32, Vec<ReadRecord>)> = analysis
            .block_boundaries
            .iter()
            .filter_map(|boundary| {
                let start = boundary.archive_id_start as usize;
                let end = boundary.archive_id_end as usize;

                let block_reads: Vec<ReadRecord> = if analysis.reordering_performed && !analysis.reverse_map.is_empty()
                {
                    (start..end)
                        .filter_map(|archive_id| {
                            analysis
                                .reverse_map
                                .get(archive_id)
                                .and_then(|&orig_id| records.get(orig_id as usize).cloned())
                        })
                        .collect()
                } else {
                    (start..end.min(records.len())).map(|i| records[i].clone()).collect()
                };

                if block_reads.is_empty() {
                    None
                } else {
                    Some((boundary.block_id, block_reads))
                }
            })
            .collect();
        let reorder_ms = t_reorder.elapsed().as_millis() as u64;

        // Parallel block compression
        let num_blocks = block_read_sets.len();
        log::info!(
            "Compressing {} blocks{}...",
            num_blocks,
            if num_blocks > 1 { " in parallel" } else { "" }
        );

        let process_ns = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let process_ns_ref = std::sync::Arc::clone(&process_ns);
        let compressed_blocks: Vec<Result<CompressedBlockData>> = block_read_sets
            .par_iter()
            .map(|(block_id, reads)| {
                let t = std::time::Instant::now();
                let mut compressor = BlockCompressor::new((*block_config).clone());
                let r = compressor.compress(reads, *block_id);
                process_ns_ref.fetch_add(t.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
                r
            })
            .collect();
        let process_ms = process_ns.load(std::sync::atomic::Ordering::Relaxed) / 1_000_000;

        // Sequential write (file I/O must be ordered)
        let t_write = std::time::Instant::now();
        let mut archive_id_start = 0u64;
        let mut output_bytes = 0u64;
        let mut blocks_written = 0;

        for (i, result) in compressed_blocks.into_iter().enumerate() {
            let compressed = result?;
            let num_reads = compressed.read_count as u64;

            writer.write_block_with_id(&compressed, archive_id_start)?;
            archive_id_start += num_reads;

            output_bytes += compressed.total_compressed_size() as u64;
            blocks_written += 1;

            log::debug!(
                "Block {} written: {} reads, {} bytes",
                i,
                num_reads,
                compressed.total_compressed_size()
            );
        }

        // Write reorder map if applicable
        let reorder_map_written = analysis.reordering_performed && !analysis.forward_map.is_empty();
        if reorder_map_written {
            writer.write_reorder_map(&analysis.forward_map, &analysis.reverse_map)?;
            log::info!("Reorder map written: {} reads", analysis.forward_map.len());
        }

        // Finalize then atomically replace the destination
        writer.finalize()?;
        output_tx.commit()?;
        let write_ms = t_write.elapsed().as_millis() as u64;

        log::info!("Compression complete! {} blocks written.", blocks_written);

        let compression_ratio = if output_bytes > 0 {
            total_bases as f64 / output_bytes as f64
        } else {
            0.0
        };

        // Build processing stats for handoff to command layer
        let stats = ProcessingStats {
            total_reads,
            total_bases,
            input_bytes: total_bases,
            output_bytes,
            blocks_written: blocks_written as u64,
            elapsed_seconds: 0.0, // Will be filled by command layer
            parse_ms,
            reorder_ms,
            process_ms,
            write_ms,
        };

        Ok(CompressionOutcome {
            mode: CompressionExecutionMode::Archive,
            detected_read_length_class: effective_length_class,
            reorder_map_written,
            blocks_written,
            reads_compressed: total_reads,
            bytes_read: total_bases,
            bytes_written: output_bytes,
            compression_ratio,
            stats,
        })
    }

    /// Execute streaming mode compression.
    ///
    /// This method implements streaming compression:
    /// 1. Read blocks incrementally
    /// 2. No global analysis/reordering
    /// 3. Lower memory footprint
    #[allow(clippy::needless_pass_by_value)]
    fn run_streaming(&self, request: CompressionRequest) -> Result<CompressionOutcome> {
        log::info!("Streaming compression mode");

        let input = request.input.resolve();

        // Inspect input lengths
        let length_stats = Self::inspect_input_lengths_for_streaming(
            &input.primary_path,
            input.secondary_path.as_deref(),
            input.is_interleaved,
            request.scan_all_lengths,
        )?
        .unwrap_or(LengthStats {
            sample_size: 0,
            avg_length: MEDIUM_READ_THRESHOLD,
            median_length: MEDIUM_READ_THRESHOLD,
            max_length: MEDIUM_READ_THRESHOLD,
        });

        let effective_length_class = Self::effective_length_class(request.requested_read_length_class, &length_stats);
        let block_size = Self::effective_block_size(
            request.block_size,
            effective_length_class,
            &length_stats,
            request.max_block_bases,
        );

        log::info!(
            "Streaming profile: sample={} avg={}bp median={}bp max={}bp, class={}, block_size={}",
            length_stats.sample_size,
            length_stats.avg_length,
            length_stats.median_length,
            length_stats.max_length,
            effective_length_class.as_str(),
            block_size
        );

        // Get archive layout for paired/interleaved
        // Route to appropriate streaming handler
        if let Some(path2) = input.secondary_path {
            Self::run_streaming_paired(
                &input.primary_path,
                &path2,
                &request,
                effective_length_class,
                block_size,
                input.archive_layout,
            )
        } else if input.is_interleaved {
            Self::run_streaming_interleaved(
                &input.primary_path,
                &request,
                effective_length_class,
                block_size,
                input.archive_layout,
            )
        } else {
            Self::run_streaming_single(&input.primary_path, &request, effective_length_class, block_size)
        }
    }

    /// Execute pipeline mode compression.
    ///
    /// This method implements pipeline compression:
    /// 1. 3-stage pipeline: Reader → Compressor → Writer
    /// 2. Parallel compression with bounded channels
    /// 3. Optional reordering for single-end reads
    #[allow(clippy::needless_pass_by_value)]
    fn run_pipeline(&self, request: CompressionRequest) -> Result<CompressionOutcome> {
        use crate::pipeline::compression::{CompressionPipeline, CompressionPipelineConfig};

        log::info!("Pipeline compression mode");

        let input = request.input.resolve();

        // Inspect input lengths
        let length_stats = Self::inspect_input_lengths_for_streaming(
            &input.primary_path,
            input.secondary_path.as_deref(),
            input.is_interleaved,
            request.scan_all_lengths,
        )?
        .unwrap_or(LengthStats {
            sample_size: 0,
            avg_length: 150,
            median_length: 150,
            max_length: 150,
        });

        let effective_length_class = Self::effective_length_class(request.requested_read_length_class, &length_stats);
        let block_size = Self::effective_block_size_for_pipeline(
            request.block_size,
            effective_length_class,
            &length_stats,
            request.max_block_bases,
            request.threads,
            request.memory_limit_mb,
        );

        let max_in_flight_blocks = Self::effective_in_flight_blocks(block_size, &length_stats, request.memory_limit_mb);

        log::info!("pipeline mode applies the archive ingest budget; use --streaming for bounded memory");

        log::info!(
            "Pipeline profile: sample={} avg={}bp median={}bp max={}bp, class={}, block_size={}, in_flight_blocks={}",
            length_stats.sample_size,
            length_stats.avg_length,
            length_stats.median_length,
            length_stats.max_length,
            effective_length_class.as_str(),
            block_size,
            max_in_flight_blocks
        );

        // Get archive layout for paired/interleaved
        // Create pipeline config
        let effective_threads = if request.threads == 0 {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        } else {
            request.threads
        };

        let pipeline_config = CompressionPipelineConfig {
            num_threads: effective_threads,
            max_in_flight_blocks,
            block_size,
            read_length_class: effective_length_class,
            quality_mode: request.quality_mode,
            id_mode: request.id_mode,
            compression_level: request.level,
            enable_reorder: request.enable_reorder && !input.is_paired,
            save_reorder_map: request.enable_reorder && !input.is_paired,
            streaming_mode: false,
            pe_layout: input.archive_layout,
            force_overwrite: request.force_overwrite,
            memory_limit_mb: request.memory_limit_mb,
        };

        let mut pipeline = CompressionPipeline::new(pipeline_config);

        let input_filename = std::path::Path::new(&input.primary_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        if let Some(ref path2) = input.secondary_path {
            pipeline.run_paired(
                &input.primary_path,
                path2,
                &request.output_path,
                input_filename,
                input.archive_layout,
            )?;
        } else if input.is_interleaved {
            pipeline.run_interleaved(
                &input.primary_path,
                &request.output_path,
                input_filename,
                input.archive_layout,
            )?;
        } else {
            pipeline.run(&input.primary_path, &request.output_path, input_filename)?;
        }

        let stats = pipeline.stats();

        log::info!(
            "Pipeline compression complete! {} blocks, {:.2}x ratio",
            stats.total_blocks,
            if stats.compression_ratio() > 0.0 {
                1.0 / stats.compression_ratio()
            } else {
                0.0
            }
        );

        // Build outcome
        let processing_stats = ProcessingStats {
            total_reads: stats.total_reads,
            total_bases: stats.total_bases,
            input_bytes: stats.input_bytes,
            output_bytes: stats.output_bytes,
            blocks_written: stats.total_blocks as u64,
            elapsed_seconds: 0.0, // Will be filled by command layer
            parse_ms: stats.parse_ms,
            reorder_ms: stats.reorder_ms,
            process_ms: stats.process_ms,
            write_ms: stats.write_ms,
        };

        Ok(CompressionOutcome {
            mode: CompressionExecutionMode::Pipeline,
            detected_read_length_class: effective_length_class,
            reorder_map_written: stats.reorder_map_written,
            blocks_written: stats.total_blocks as usize,
            reads_compressed: stats.total_reads,
            bytes_read: stats.input_bytes,
            bytes_written: stats.output_bytes,
            compression_ratio: if stats.output_bytes > 0 {
                stats.input_bytes as f64 / stats.output_bytes as f64
            } else {
                0.0
            },
            stats: processing_stats,
        })
    }

    // =============================================================================
    // Helper methods (extracted from CompressCommand)
    // =============================================================================

    fn read_all_records(input_path: &str, memory_limit_mb: usize) -> Result<Vec<ReadRecord>> {
        if input_path == "-" {
            open_fastq_stdin().collect_all_within_archive_budget(memory_limit_mb)
        } else {
            open_fastq(input_path)?.collect_all_within_archive_budget(memory_limit_mb)
        }
    }

    fn read_all_paired_records(
        input_path: &str,
        input2_path: &str,
        pe_layout: PeLayout,
        memory_limit_mb: usize,
    ) -> Result<Vec<ReadRecord>> {
        open_fastq_paired(input_path, input2_path)?.collect_pairs_within_archive_budget(pe_layout, memory_limit_mb)
    }

    fn read_all_interleaved_records(
        input_path: &str,
        pe_layout: PeLayout,
        memory_limit_mb: usize,
    ) -> Result<Vec<ReadRecord>> {
        let mut parser = if input_path == "-" {
            crate::fastq::parser::InterleavedPeParser::new(open_fastq_stdin())
        } else {
            open_fastq_interleaved(input_path)?
        };
        parser.collect_pairs_within_archive_budget(pe_layout, memory_limit_mb)
    }

    fn length_stats_from_records_sampled(records: &[ReadRecord], scan_all: bool) -> LengthStats {
        if scan_all || records.len() <= DEFAULT_LENGTH_SAMPLE_READS {
            let lengths: Vec<usize> = records.iter().map(|r| r.sequence.len()).collect();
            return LengthStats::from_lengths(&lengths);
        }

        let sample_size = DEFAULT_LENGTH_SAMPLE_READS.min(records.len());
        let lengths: Vec<usize> = (0..sample_size)
            .map(|i| {
                let idx = i * records.len() / sample_size;
                records[idx].sequence.len()
            })
            .collect();
        LengthStats::from_lengths(&lengths)
    }

    fn effective_length_class(requested: Option<ReadLengthClass>, stats: &LengthStats) -> ReadLengthClass {
        requested.unwrap_or_else(|| crate::types::classify_read_length(stats.median_length, stats.max_length))
    }

    fn effective_block_size(
        requested_block_size: usize,
        class: ReadLengthClass,
        stats: &LengthStats,
        max_block_bases: usize,
    ) -> usize {
        if requested_block_size > 0 {
            return requested_block_size;
        }

        let mut block_size = if stats.max_length < SPRING_MAX_READ_LENGTH {
            DEFAULT_BLOCK_SIZE_SHORT
        } else {
            match class {
                ReadLengthClass::Short => DEFAULT_BLOCK_SIZE_SHORT,
                ReadLengthClass::Medium => DEFAULT_BLOCK_SIZE_MEDIUM,
                ReadLengthClass::Long => DEFAULT_BLOCK_SIZE_LONG,
            }
        };

        // Apply max_block_bases limit for non-short reads
        if max_block_bases > 0 && class != ReadLengthClass::Short {
            let per_read_bases = stats.max_length.max(1);
            block_size = block_size.min((max_block_bases / per_read_bases).max(1));
        }

        block_size.max(1)
    }

    // =============================================================================
    // Streaming mode helpers
    // =============================================================================

    fn run_streaming_single(
        input_path: &str,
        request: &CompressionRequest,
        effective_length_class: ReadLengthClass,
        block_size: usize,
    ) -> Result<CompressionOutcome> {
        // Open input
        let mut parser = if input_path == "-" {
            open_fastq_stdin()
        } else {
            open_fastq(input_path)?
        };

        let (mut writer, output_tx) = begin_fqc_writer(&request.output_path, request.force_overwrite)?;

        let flags = build_flags(
            false,
            true,
            request.quality_mode,
            request.id_mode,
            false,
            PeLayout::Interleaved,
            effective_length_class,
            true,
        );
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let input_filename = std::path::Path::new(input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("stdin");
        let global_header = GlobalHeader::new(flags, 0, input_filename, timestamp);
        writer.write_global_header(&global_header)?;

        let block_config = BlockCompressorConfig {
            read_length_class: effective_length_class,
            quality_mode: request.quality_mode,
            id_mode: request.id_mode,
            zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(request.level),
            ..Default::default()
        };
        let mut compressor = BlockCompressor::new(block_config);

        let mut block_id = 0u32;
        let mut archive_id_start = 0u64;
        let mut block_buf: Vec<ReadRecord> = Vec::with_capacity(block_size);
        let mut total_reads = 0u64;
        let mut total_bases = 0u64;
        let mut output_bytes = 0u64;
        let mut blocks_written = 0;

        while let Some(rec) = parser.next_record()? {
            total_reads += 1;
            total_bases += rec.sequence.len() as u64;
            block_buf.push(rec);

            if block_buf.len() >= block_size {
                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                archive_id_start += block_buf.len() as u64;
                output_bytes += compressed.total_compressed_size() as u64;
                blocks_written += 1;
                block_id += 1;
                block_buf.clear();
            }
        }

        // Flush remaining reads
        if !block_buf.is_empty() {
            let compressed = compressor.compress(&block_buf, block_id)?;
            writer.write_block_with_id(&compressed, archive_id_start)?;
            output_bytes += compressed.total_compressed_size() as u64;
            blocks_written += 1;
        }

        writer.patch_total_read_count(total_reads)?;
        writer.finalize()?;
        output_tx.commit()?;
        log::info!("Streaming compression complete! {} blocks written.", blocks_written);

        let stats = ProcessingStats {
            total_reads,
            total_bases,
            input_bytes: total_bases,
            output_bytes,
            blocks_written: blocks_written as u64,
            elapsed_seconds: 0.0,
            parse_ms: 0,
            reorder_ms: 0,
            process_ms: 0,
            write_ms: 0,
        };

        Ok(CompressionOutcome {
            mode: CompressionExecutionMode::Streaming,
            detected_read_length_class: effective_length_class,
            reorder_map_written: false,
            blocks_written,
            reads_compressed: total_reads,
            bytes_read: total_bases,
            bytes_written: output_bytes,
            compression_ratio: if output_bytes > 0 {
                total_bases as f64 / output_bytes as f64
            } else {
                0.0
            },
            stats,
        })
    }

    fn run_streaming_paired(
        input_path: &str,
        input2_path: &str,
        request: &CompressionRequest,
        effective_length_class: ReadLengthClass,
        block_size: usize,
        pe_layout: PeLayout,
    ) -> Result<CompressionOutcome> {
        log::info!("Streaming compression mode (paired-end)");

        let mut pe_reader = open_fastq_paired(input_path, input2_path)?;
        let (mut writer, output_tx) = begin_fqc_writer(&request.output_path, request.force_overwrite)?;

        let flags = build_flags(
            true,
            true,
            request.quality_mode,
            request.id_mode,
            false,
            pe_layout,
            effective_length_class,
            true,
        );
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let input_filename = std::path::Path::new(input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("stdin");
        let global_header = GlobalHeader::new(flags, 0, input_filename, timestamp);
        writer.write_global_header(&global_header)?;

        let block_config = BlockCompressorConfig {
            read_length_class: effective_length_class,
            quality_mode: request.quality_mode,
            id_mode: request.id_mode,
            zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(request.level),
            ..Default::default()
        };
        let mut compressor = BlockCompressor::new(block_config);

        let mut block_id = 0u32;
        let mut archive_id_start = 0u64;
        let target_pairs = (block_size.max(2)) / 2;
        let mut r1_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        let mut r2_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        let mut total_reads = 0u64;
        let mut total_bases = 0u64;
        let mut output_bytes = 0u64;
        let mut blocks_written = 0;

        while let Some((r1, r2)) = pe_reader.next_pair()? {
            total_reads += 2;
            total_bases += (r1.sequence.len() + r2.sequence.len()) as u64;
            r1_buf.push(r1);
            r2_buf.push(r2);

            if r1_buf.len() >= target_pairs {
                let block_buf = pe_layout.arrange(std::mem::take(&mut r1_buf), std::mem::take(&mut r2_buf));

                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                archive_id_start += block_buf.len() as u64;
                output_bytes += compressed.total_compressed_size() as u64;
                blocks_written += 1;
                block_id += 1;
            }
        }

        if !r1_buf.is_empty() || !r2_buf.is_empty() {
            let block_buf = pe_layout.arrange(r1_buf, r2_buf);

            if !block_buf.is_empty() {
                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                output_bytes += compressed.total_compressed_size() as u64;
                blocks_written += 1;
            }
        }

        writer.patch_total_read_count(total_reads)?;
        writer.finalize()?;
        output_tx.commit()?;
        log::info!("Streaming compression complete! {} blocks written.", blocks_written);

        let stats = ProcessingStats {
            total_reads,
            total_bases,
            input_bytes: total_bases,
            output_bytes,
            blocks_written: blocks_written as u64,
            elapsed_seconds: 0.0,
            parse_ms: 0,
            reorder_ms: 0,
            process_ms: 0,
            write_ms: 0,
        };

        Ok(CompressionOutcome {
            mode: CompressionExecutionMode::Streaming,
            detected_read_length_class: effective_length_class,
            reorder_map_written: false,
            blocks_written,
            reads_compressed: total_reads,
            bytes_read: total_bases,
            bytes_written: output_bytes,
            compression_ratio: if output_bytes > 0 {
                total_bases as f64 / output_bytes as f64
            } else {
                0.0
            },
            stats,
        })
    }

    fn run_streaming_interleaved(
        input_path: &str,
        request: &CompressionRequest,
        effective_length_class: ReadLengthClass,
        block_size: usize,
        pe_layout: PeLayout,
    ) -> Result<CompressionOutcome> {
        log::info!("Streaming compression mode (interleaved single-file PE)");

        let mut parser = if input_path == "-" {
            crate::fastq::parser::InterleavedPeParser::new(open_fastq_stdin())
        } else {
            open_fastq_interleaved(input_path)?
        };

        let (mut writer, output_tx) = begin_fqc_writer(&request.output_path, request.force_overwrite)?;

        let flags = build_flags(
            true,
            true,
            request.quality_mode,
            request.id_mode,
            false,
            pe_layout,
            effective_length_class,
            true,
        );
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let input_filename = std::path::Path::new(input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("stdin");
        let global_header = GlobalHeader::new(flags, 0, input_filename, timestamp);
        writer.write_global_header(&global_header)?;

        let block_config = BlockCompressorConfig {
            read_length_class: effective_length_class,
            quality_mode: request.quality_mode,
            id_mode: request.id_mode,
            zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(request.level),
            ..Default::default()
        };
        let mut compressor = BlockCompressor::new(block_config);

        let mut block_id = 0u32;
        let mut archive_id_start = 0u64;
        let target_pairs = (block_size.max(2)) / 2;
        let mut r1_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        let mut r2_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        let mut total_reads = 0u64;
        let mut total_bases = 0u64;
        let mut output_bytes = 0u64;
        let mut blocks_written = 0;

        while let Some((r1, r2)) = parser.next_pair()? {
            total_reads += 2;
            total_bases += (r1.sequence.len() + r2.sequence.len()) as u64;
            r1_buf.push(r1);
            r2_buf.push(r2);

            if r1_buf.len() >= target_pairs {
                let block_buf = pe_layout.arrange(std::mem::take(&mut r1_buf), std::mem::take(&mut r2_buf));

                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                archive_id_start += block_buf.len() as u64;
                output_bytes += compressed.total_compressed_size() as u64;
                blocks_written += 1;
                block_id += 1;
            }
        }

        if !r1_buf.is_empty() || !r2_buf.is_empty() {
            let block_buf = pe_layout.arrange(r1_buf, r2_buf);

            if !block_buf.is_empty() {
                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                output_bytes += compressed.total_compressed_size() as u64;
                blocks_written += 1;
            }
        }

        writer.patch_total_read_count(total_reads)?;
        writer.finalize()?;
        output_tx.commit()?;
        log::info!("Streaming compression complete! {} blocks written.", blocks_written);

        let stats = ProcessingStats {
            total_reads,
            total_bases,
            input_bytes: total_bases,
            output_bytes,
            blocks_written: blocks_written as u64,
            elapsed_seconds: 0.0,
            parse_ms: 0,
            reorder_ms: 0,
            process_ms: 0,
            write_ms: 0,
        };

        Ok(CompressionOutcome {
            mode: CompressionExecutionMode::Streaming,
            detected_read_length_class: effective_length_class,
            reorder_map_written: false,
            blocks_written,
            reads_compressed: total_reads,
            bytes_read: total_bases,
            bytes_written: output_bytes,
            compression_ratio: if output_bytes > 0 {
                total_bases as f64 / output_bytes as f64
            } else {
                0.0
            },
            stats,
        })
    }

    fn inspect_input_lengths_for_streaming(
        input_path: &str,
        input2_path: Option<&str>,
        is_interleaved: bool,
        scan_all: bool,
    ) -> Result<Option<LengthStats>> {
        if input_path == "-" {
            return Ok(None);
        }

        let sample_limit = if scan_all {
            usize::MAX
        } else {
            DEFAULT_LENGTH_SAMPLE_READS
        };
        let mut lengths = Vec::new();

        if let Some(path2) = input2_path {
            let mut reader = open_fastq_paired(input_path, path2)?;
            while let Some((r1, r2)) = reader.next_pair()? {
                lengths.push(r1.sequence.len());
                lengths.push(r2.sequence.len());
                if lengths.len() >= sample_limit {
                    break;
                }
            }
        } else if is_interleaved {
            let mut parser = open_fastq_interleaved(input_path)?;
            while let Some((r1, r2)) = parser.next_pair()? {
                if scan_all || lengths.len() < sample_limit {
                    lengths.push(r1.sequence.len());
                    lengths.push(r2.sequence.len());
                }
            }
        } else {
            let mut parser = open_fastq(input_path)?;
            while let Some(record) = parser.next_record()? {
                lengths.push(record.sequence.len());
                if lengths.len() >= sample_limit {
                    break;
                }
            }
        }

        if lengths.is_empty() {
            Ok(None)
        } else {
            Ok(Some(LengthStats::from_lengths(&lengths)))
        }
    }

    fn effective_block_size_for_pipeline(
        requested_block_size: usize,
        class: ReadLengthClass,
        stats: &LengthStats,
        max_block_bases: usize,
        threads: usize,
        memory_limit_mb: usize,
    ) -> usize {
        use crate::memory_budget::{auto_memory_budget, MemoryEstimator};

        if requested_block_size > 0 {
            return requested_block_size;
        }

        let effective_threads = if threads == 0 {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        } else {
            threads
        };

        let budget = auto_memory_budget(memory_limit_mb);
        let estimator = MemoryEstimator::new(budget);
        let mut block_size =
            crate::types::recommended_block_size(class).min(estimator.optimal_block_size(effective_threads));

        if max_block_bases > 0 && class != ReadLengthClass::Short {
            let per_read_bases = stats.max_length.max(1);
            block_size = block_size.min((max_block_bases / per_read_bases).max(1));
        }

        block_size.max(1)
    }

    fn effective_in_flight_blocks(block_size: usize, stats: &LengthStats, memory_limit_mb: usize) -> usize {
        use crate::memory_budget::auto_memory_budget;
        use crate::pipeline::DEFAULT_MAX_IN_FLIGHT_BLOCKS;

        let budget = auto_memory_budget(memory_limit_mb);
        let bytes_per_read = stats.avg_length.max(1).saturating_mul(3).saturating_add(80);
        let chunk_bytes = block_size.saturating_mul(bytes_per_read);
        if chunk_bytes == 0 {
            return DEFAULT_MAX_IN_FLIGHT_BLOCKS;
        }

        budget
            .block_buffer_bytes()
            .saturating_div(chunk_bytes)
            .clamp(1, DEFAULT_MAX_IN_FLIGHT_BLOCKS)
    }
}
