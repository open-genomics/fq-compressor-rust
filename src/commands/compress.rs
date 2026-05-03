// =============================================================================
// fqc-rust - Compress Command
// =============================================================================

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig, CompressedBlockData};
use crate::algo::global_analyzer::{GlobalAnalyzer, GlobalAnalyzerConfig};
use crate::common::memory_budget::{auto_memory_budget, ChunkingStrategy, MemoryEstimator};
use crate::error::{FqcError, Result};
use crate::fastq::parser::{open_fastq, open_fastq_paired, open_fastq_stdin};
use crate::format::{build_flags, GlobalHeader};
use crate::fqc_writer::FqcWriter;
use crate::pipeline::compression::{CompressionPipeline, CompressionPipelineConfig};
use crate::pipeline::DEFAULT_MAX_IN_FLIGHT_BLOCKS;
use crate::types::*;
use rayon::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_LENGTH_SAMPLE_READS: usize = 4_096;

// =============================================================================
// CompressOptions
// =============================================================================

#[derive(Debug, Clone)]
pub struct CompressOptions {
    pub input_path: String,
    pub input2_path: Option<String>,
    pub output_path: String,
    pub level: CompressionLevel,
    pub enable_reorder: bool,
    pub streaming_mode: bool,
    pub quality_mode: QualityMode,
    pub id_mode: IdMode,
    pub threads: usize,
    pub memory_limit_mb: usize,
    pub force_overwrite: bool,
    pub show_progress: bool,
    pub read_length_class: Option<ReadLengthClass>,
    pub auto_detect_length: bool,
    pub block_size: usize,
    pub pe_layout: PeLayout,
    pub interleaved: bool,
    pub max_block_bases: usize,
    pub scan_all_lengths: bool,
    pub use_pipeline: bool,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            input2_path: None,
            output_path: String::new(),
            level: DEFAULT_COMPRESSION_LEVEL,
            enable_reorder: true,
            streaming_mode: false,
            quality_mode: QualityMode::Lossless,
            id_mode: IdMode::Exact,
            threads: 0,
            memory_limit_mb: 0,
            force_overwrite: false,
            show_progress: true,
            read_length_class: None,
            auto_detect_length: true,
            block_size: 0,
            pe_layout: PeLayout::Interleaved,
            interleaved: false,
            max_block_bases: 0,
            scan_all_lengths: false,
            use_pipeline: false,
        }
    }
}

impl CompressOptions {
    /// Create a BlockCompressorConfig from these options.
    pub fn to_block_config(&self, read_length_class: ReadLengthClass) -> BlockCompressorConfig {
        BlockCompressorConfig {
            read_length_class,
            compression_level: self.level,
            quality_mode: self.quality_mode,
            id_mode: self.id_mode,
            zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(self.level),
            ..Default::default()
        }
    }

    /// Create a CompressionPipelineConfig from these options.
    pub fn to_pipeline_config(
        &self,
        read_length_class: ReadLengthClass,
        block_size: usize,
        max_in_flight_blocks: usize,
        is_paired: bool,
    ) -> CompressionPipelineConfig {
        CompressionPipelineConfig {
            num_threads: self.threads,
            max_in_flight_blocks,
            block_size,
            read_length_class,
            quality_mode: self.quality_mode,
            id_mode: self.id_mode,
            compression_level: self.level,
            enable_reorder: self.enable_reorder && !is_paired,
            save_reorder_map: self.enable_reorder && !is_paired,
            streaming_mode: false,
            pe_layout: self.pe_layout,
            memory_limit_mb: self.memory_limit_mb,
        }
    }
}

// =============================================================================
// CompressStats
// =============================================================================

#[derive(Debug, Default)]
struct CompressStats {
    total_reads: u64,
    total_bases: u64,
    input_bytes: u64,
    output_bytes: u64,
    blocks_written: u64,
    elapsed_seconds: f64,
}

impl CompressStats {
    fn compression_ratio(&self) -> f64 {
        if self.output_bytes == 0 {
            return 0.0;
        }
        self.input_bytes as f64 / self.output_bytes as f64
    }

    fn bits_per_base(&self) -> f64 {
        if self.total_bases == 0 {
            return 0.0;
        }
        (self.output_bytes as f64 * 8.0) / self.total_bases as f64
    }

    fn throughput_mbps(&self) -> f64 {
        if self.elapsed_seconds == 0.0 {
            return 0.0;
        }
        (self.input_bytes as f64 / 1_048_576.0) / self.elapsed_seconds
    }
}

// =============================================================================
// CompressCommand
// =============================================================================

pub struct CompressCommand {
    opts: CompressOptions,
    stats: CompressStats,
}

impl CompressCommand {
    pub fn new(opts: CompressOptions) -> Self {
        Self {
            opts,
            stats: CompressStats::default(),
        }
    }

    pub fn execute(mut self) -> i32 {
        let start = std::time::Instant::now();

        match self.run() {
            Ok(()) => {
                self.stats.elapsed_seconds = start.elapsed().as_secs_f64();
                if self.opts.show_progress {
                    self.print_summary();
                }
                0
            }
            Err(e) => {
                eprintln!("Compression failed: {e}");
                e.exit_code_num()
            }
        }
    }

    fn run(&mut self) -> Result<()> {
        self.validate_options()?;

        // Streaming mode: read and compress blocks incrementally
        if self.opts.streaming_mode {
            return self.run_streaming();
        }

        // Pipeline mode: use 3-stage Reader→Compressor→Writer pipeline
        if self.opts.use_pipeline {
            return self.run_pipeline();
        }

        // Phase 0: Read all records
        let is_paired = self.opts.input2_path.is_some() || self.opts.interleaved;
        log::info!("Reading input file: {}", self.opts.input_path);
        let records = self.read_all_records()?;

        if records.is_empty() {
            return Err(FqcError::InvalidArgument(
                "Input file contains no FASTQ records".to_string(),
            ));
        }

        let total_bases: u64 = records.iter().map(|r| r.sequence.len() as u64).sum();
        self.stats.total_reads = records.len() as u64;
        self.stats.total_bases = total_bases;
        self.stats.input_bytes = total_bases; // approximate

        log::info!("Loaded {} reads ({} bases)", records.len(), total_bases);

        // Detect read length class if auto
        let length_stats = self.length_stats_from_records(&records);
        let effective_length_class = self.effective_length_class(&length_stats);

        // Adjust parameters based on length class
        let block_size = self.effective_block_size(effective_length_class, &length_stats);
        let enable_reorder = self.opts.enable_reorder
            && !self.opts.streaming_mode
            && !is_paired
            && effective_length_class == ReadLengthClass::Short;

        self.enforce_archive_mode_memory_limit(records.len(), block_size, &length_stats)?;

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

        // Phase 2: Write FQC archive
        if !self.opts.force_overwrite && std::path::Path::new(&self.opts.output_path).exists() {
            return Err(FqcError::InvalidArgument(format!(
                "Output file already exists: {} (use -f to overwrite)",
                self.opts.output_path
            )));
        }

        let mut writer = FqcWriter::create(&self.opts.output_path)?;

        // Build flags
        let flags = build_flags(
            is_paired,
            !analysis.reordering_performed,
            self.opts.quality_mode,
            self.opts.id_mode,
            analysis.reordering_performed,
            self.opts.pe_layout,
            effective_length_class,
            self.opts.streaming_mode,
        );

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let input_filename = std::path::Path::new(&self.opts.input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let global_header = GlobalHeader::new(flags, records.len() as u64, input_filename, timestamp);
        writer.write_global_header(&global_header)?;

        // Block compressor config
        let block_config = std::sync::Arc::new(self.opts.to_block_config(effective_length_class));

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

        // Parallel block compression
        let num_blocks = block_read_sets.len();
        log::info!(
            "Compressing {} blocks{}...",
            num_blocks,
            if num_blocks > 1 { " in parallel" } else { "" }
        );

        let compressed_blocks: Vec<Result<CompressedBlockData>> = block_read_sets
            .par_iter()
            .map(|(block_id, reads)| {
                let compressor = BlockCompressor::new((*block_config).clone());
                compressor.compress(reads, *block_id)
            })
            .collect();

        // Sequential write (file I/O must be ordered)
        let mut archive_id_start = 0u64;
        for (i, result) in compressed_blocks.into_iter().enumerate() {
            let compressed = result?;
            let num_reads = compressed.read_count as u64;

            writer.write_block_with_id(&compressed, archive_id_start)?;
            archive_id_start += num_reads;

            self.stats.output_bytes += compressed.total_compressed_size() as u64;
            self.stats.blocks_written += 1;

            log::debug!(
                "Block {} written: {} reads, {} bytes",
                i,
                num_reads,
                compressed.total_compressed_size()
            );
        }

        // Write reorder map if applicable
        if analysis.reordering_performed && !analysis.forward_map.is_empty() {
            writer.write_reorder_map(&analysis.forward_map, &analysis.reverse_map)?;
            log::info!("Reorder map written: {} reads", analysis.forward_map.len());
        }

        // Finalize
        writer.finalize()?;

        log::info!("Compression complete! {} blocks written.", self.stats.blocks_written);
        Ok(())
    }

    /// Streaming compression: read blocks incrementally, no global reordering.
    fn run_streaming(&mut self) -> Result<()> {
        log::info!("Streaming compression mode");

        // Force overwrite check
        if !self.opts.force_overwrite && std::path::Path::new(&self.opts.output_path).exists() {
            return Err(FqcError::InvalidArgument(format!(
                "Output file already exists: {} (use -f to overwrite)",
                self.opts.output_path
            )));
        }

        let length_stats = self.inspect_input_lengths()?.unwrap_or(LengthStats {
            sample_size: 0,
            avg_length: MEDIUM_READ_THRESHOLD,
            median_length: MEDIUM_READ_THRESHOLD,
            max_length: MEDIUM_READ_THRESHOLD,
        });
        let effective_length_class = self.effective_length_class(&length_stats);
        let block_size = self.effective_block_size(effective_length_class, &length_stats);

        log::info!(
            "Streaming profile: sample={} avg={}bp median={}bp max={}bp, class={}, block_size={}",
            length_stats.sample_size,
            length_stats.avg_length,
            length_stats.median_length,
            length_stats.max_length,
            effective_length_class.as_str(),
            block_size
        );

        if let Some(path2) = self.opts.input2_path.clone() {
            return self.run_streaming_paired(&path2, effective_length_class, block_size);
        }

        // Interleaved single-file PE streaming
        if self.opts.interleaved {
            return self.run_streaming_interleaved(effective_length_class, block_size);
        }

        // Open input
        let mut parser = if self.opts.input_path == "-" {
            open_fastq_stdin()
        } else {
            open_fastq(&self.opts.input_path)?
        };

        // Open writer
        let mut writer = FqcWriter::create(&self.opts.output_path)?;

        let flags = build_flags(
            false,
            true,
            self.opts.quality_mode,
            self.opts.id_mode,
            false,
            self.opts.pe_layout,
            effective_length_class,
            true,
        );
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let input_filename = std::path::Path::new(&self.opts.input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("stdin");
        let global_header = GlobalHeader::new(flags, 0, input_filename, timestamp);
        writer.write_global_header(&global_header)?;

        let compressor = BlockCompressor::new(self.opts.to_block_config(effective_length_class));

        let mut block_id = 0u32;
        let mut archive_id_start = 0u64;
        let mut block_buf: Vec<ReadRecord> = Vec::with_capacity(block_size);

        while let Some(rec) = parser.next_record()? {
            self.stats.total_reads += 1;
            self.stats.total_bases += rec.sequence.len() as u64;
            block_buf.push(rec);

            if block_buf.len() >= block_size {
                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                archive_id_start += block_buf.len() as u64;
                self.stats.output_bytes += compressed.total_compressed_size() as u64;
                self.stats.blocks_written += 1;
                block_id += 1;
                block_buf.clear();
            }
        }

        // Flush remaining reads
        if !block_buf.is_empty() {
            let compressed = compressor.compress(&block_buf, block_id)?;
            writer.write_block_with_id(&compressed, archive_id_start)?;
            self.stats.output_bytes += compressed.total_compressed_size() as u64;
            self.stats.blocks_written += 1;
        }

        self.stats.input_bytes = self.stats.total_bases;
        writer.patch_total_read_count(self.stats.total_reads)?;
        writer.finalize()?;
        log::info!(
            "Streaming compression complete! {} blocks written.",
            self.stats.blocks_written
        );
        Ok(())
    }

    /// Streaming compression for interleaved single-file paired-end input.
    fn run_streaming_interleaved(&mut self, effective_length_class: ReadLengthClass, block_size: usize) -> Result<()> {
        log::info!("Streaming compression mode (interleaved single-file PE)");

        let mut parser = if self.opts.input_path == "-" {
            open_fastq_stdin()
        } else {
            open_fastq(&self.opts.input_path)?
        };

        let mut writer = FqcWriter::create(&self.opts.output_path)?;

        let flags = build_flags(
            true,
            true,
            self.opts.quality_mode,
            self.opts.id_mode,
            false,
            self.opts.pe_layout,
            effective_length_class,
            true,
        );
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let input_filename = std::path::Path::new(&self.opts.input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("stdin");
        let global_header = GlobalHeader::new(flags, 0, input_filename, timestamp);
        writer.write_global_header(&global_header)?;

        let compressor = BlockCompressor::new(self.opts.to_block_config(effective_length_class));

        let mut block_id = 0u32;
        let mut archive_id_start = 0u64;
        let target_pairs = (block_size.max(2)) / 2;
        let mut r1_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        let mut r2_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);

        // Read pairs from single interleaved file
        loop {
            let r1 = match parser.next_record()? {
                Some(rec) => rec,
                None => break,
            };
            let r2 = match parser.next_record()? {
                Some(rec) => rec,
                None => {
                    log::warn!("Odd number of reads in interleaved file, last read treated as unpaired");
                    self.stats.total_reads += 1;
                    self.stats.total_bases += r1.sequence.len() as u64;
                    r1_buf.push(r1);
                    break;
                }
            };

            self.stats.total_reads += 2;
            self.stats.total_bases += (r1.sequence.len() + r2.sequence.len()) as u64;
            r1_buf.push(r1);
            r2_buf.push(r2);

            if r1_buf.len() >= target_pairs {
                let block_buf = self
                    .opts
                    .pe_layout
                    .arrange(std::mem::take(&mut r1_buf), std::mem::take(&mut r2_buf));

                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                archive_id_start += block_buf.len() as u64;
                self.stats.output_bytes += compressed.total_compressed_size() as u64;
                self.stats.blocks_written += 1;
                block_id += 1;
            }
        }

        // Flush remaining
        if !r1_buf.is_empty() || !r2_buf.is_empty() {
            let block_buf = self.opts.pe_layout.arrange(r1_buf, r2_buf);

            if !block_buf.is_empty() {
                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                self.stats.output_bytes += compressed.total_compressed_size() as u64;
                self.stats.blocks_written += 1;
            }
        }

        self.stats.input_bytes = self.stats.total_bases;
        writer.patch_total_read_count(self.stats.total_reads)?;
        writer.finalize()?;
        log::info!(
            "Streaming compression complete! {} blocks written.",
            self.stats.blocks_written
        );
        Ok(())
    }

    fn run_streaming_paired(
        &mut self,
        path2: &str,
        effective_length_class: ReadLengthClass,
        block_size: usize,
    ) -> Result<()> {
        log::info!("Streaming compression mode (paired-end)");

        let mut pe_reader = open_fastq_paired(&self.opts.input_path, path2)?;
        let mut writer = FqcWriter::create(&self.opts.output_path)?;

        let flags = build_flags(
            true,
            true,
            self.opts.quality_mode,
            self.opts.id_mode,
            false,
            self.opts.pe_layout,
            effective_length_class,
            true,
        );
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let input_filename = std::path::Path::new(&self.opts.input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("stdin");
        let global_header = GlobalHeader::new(flags, 0, input_filename, timestamp);
        writer.write_global_header(&global_header)?;

        let compressor = BlockCompressor::new(self.opts.to_block_config(effective_length_class));

        let mut block_id = 0u32;
        let mut archive_id_start = 0u64;
        let target_pairs = (block_size.max(2)) / 2;
        let mut r1_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);
        let mut r2_buf: Vec<ReadRecord> = Vec::with_capacity(target_pairs);

        while let Some((r1, r2)) = pe_reader.next_pair()? {
            self.stats.total_reads += 2;
            self.stats.total_bases += (r1.sequence.len() + r2.sequence.len()) as u64;
            r1_buf.push(r1);
            r2_buf.push(r2);

            if r1_buf.len() >= target_pairs {
                let block_buf = self
                    .opts
                    .pe_layout
                    .arrange(std::mem::take(&mut r1_buf), std::mem::take(&mut r2_buf));

                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                archive_id_start += block_buf.len() as u64;
                self.stats.output_bytes += compressed.total_compressed_size() as u64;
                self.stats.blocks_written += 1;
                block_id += 1;
            }
        }

        if !r1_buf.is_empty() || !r2_buf.is_empty() {
            let block_buf = self.opts.pe_layout.arrange(r1_buf, r2_buf);

            if !block_buf.is_empty() {
                let compressed = compressor.compress(&block_buf, block_id)?;
                writer.write_block_with_id(&compressed, archive_id_start)?;
                self.stats.output_bytes += compressed.total_compressed_size() as u64;
                self.stats.blocks_written += 1;
            }
        }

        self.stats.input_bytes = self.stats.total_bases;
        writer.patch_total_read_count(self.stats.total_reads)?;
        writer.finalize()?;
        log::info!(
            "Streaming compression complete! {} blocks written.",
            self.stats.blocks_written
        );
        Ok(())
    }

    fn validate_options(&self) -> Result<()> {
        if self.opts.input_path != "-" && !std::path::Path::new(&self.opts.input_path).exists() {
            return Err(FqcError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", self.opts.input_path),
            )));
        }
        if self.opts.level < MIN_COMPRESSION_LEVEL || self.opts.level > MAX_COMPRESSION_LEVEL {
            return Err(FqcError::InvalidArgument(format!(
                "Compression level must be {}-{}",
                MIN_COMPRESSION_LEVEL, MAX_COMPRESSION_LEVEL
            )));
        }
        Ok(())
    }

    fn read_all_records(&self) -> Result<Vec<ReadRecord>> {
        if self.opts.input_path == "-" {
            let mut parser = open_fastq_stdin();
            parser.collect_all()
        } else if let Some(ref path2) = self.opts.input2_path {
            log::info!("Reading paired-end input: {} + {}", self.opts.input_path, path2);
            let mut pe_reader = open_fastq_paired(&self.opts.input_path, path2)?;
            match self.opts.pe_layout {
                PeLayout::Interleaved => pe_reader.collect_all_interleaved(),
                PeLayout::Consecutive => pe_reader.collect_all_consecutive(),
            }
        } else if self.opts.interleaved {
            // Single file with interleaved PE reads: already in R1,R2,R1,R2 order
            log::info!("Reading interleaved paired-end input: {}", self.opts.input_path);
            let mut parser = open_fastq(&self.opts.input_path)?;
            let records = parser.collect_all()?;
            // For consecutive PE layout, need to rearrange from interleaved to consecutive
            if self.opts.pe_layout == PeLayout::Consecutive && records.len() >= 2 {
                let mut r1 = Vec::with_capacity(records.len() / 2);
                let mut r2 = Vec::with_capacity(records.len() / 2);
                for (i, rec) in records.into_iter().enumerate() {
                    if i % 2 == 0 {
                        r1.push(rec);
                    } else {
                        r2.push(rec);
                    }
                }
                r1.extend(r2);
                Ok(r1)
            } else {
                Ok(records)
            }
        } else {
            let mut parser = open_fastq(&self.opts.input_path)?;
            parser.collect_all()
        }
    }

    /// Pipeline mode: 3-stage Reader→Compressor→Writer with backpressure
    fn run_pipeline(&mut self) -> Result<()> {
        log::info!("Using pipeline compression mode");

        if !self.opts.force_overwrite && std::path::Path::new(&self.opts.output_path).exists() {
            return Err(FqcError::InvalidArgument(format!(
                "Output file already exists: {} (use -f to overwrite)",
                self.opts.output_path
            )));
        }

        let length_stats = self.inspect_input_lengths()?.unwrap_or(LengthStats {
            sample_size: 0,
            avg_length: 150,
            median_length: 150,
            max_length: 150,
        });
        let effective_length_class = self.effective_length_class(&length_stats);
        let block_size = self.effective_block_size(effective_length_class, &length_stats);
        let is_paired = self.opts.input2_path.is_some() || self.opts.interleaved;
        let max_in_flight_blocks = self.effective_in_flight_blocks(block_size, &length_stats);

        if self.opts.memory_limit_mb > 0 {
            log::warn!(
                "pipeline mode still performs a full ingest before compression; use --streaming for strict memory caps"
            );
        }

        let pipeline_config =
            self.opts
                .to_pipeline_config(effective_length_class, block_size, max_in_flight_blocks, is_paired);

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

        let mut pipeline = CompressionPipeline::new(pipeline_config);

        let input_filename = std::path::Path::new(&self.opts.input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        if let Some(ref path2) = self.opts.input2_path {
            pipeline.run_paired(
                &self.opts.input_path,
                path2,
                &self.opts.output_path,
                input_filename,
                self.opts.pe_layout,
            )?;
        } else {
            pipeline.run(&self.opts.input_path, &self.opts.output_path, input_filename)?;
        }

        let stats = pipeline.stats();
        self.stats.total_reads = stats.total_reads;
        self.stats.total_bases = 0; // pipeline doesn't track bases separately
        self.stats.input_bytes = stats.input_bytes;
        self.stats.output_bytes = stats.output_bytes;
        self.stats.blocks_written = stats.total_blocks as u64;

        log::info!(
            "Pipeline compression complete! {} blocks, {:.2}x ratio",
            stats.total_blocks,
            if stats.compression_ratio() > 0.0 {
                1.0 / stats.compression_ratio()
            } else {
                0.0
            }
        );
        Ok(())
    }

    fn effective_length_class(&self, stats: &LengthStats) -> ReadLengthClass {
        self.opts
            .read_length_class
            .unwrap_or_else(|| classify_read_length(stats.median_length, stats.max_length))
    }

    fn effective_block_size(&self, class: ReadLengthClass, stats: &LengthStats) -> usize {
        if self.opts.block_size > 0 {
            return self.opts.block_size;
        }

        let budget = auto_memory_budget(self.opts.memory_limit_mb);
        let estimator = MemoryEstimator::new(budget);
        let mut block_size = recommended_block_size(class).min(estimator.optimal_block_size(self.effective_threads()));

        if self.opts.max_block_bases > 0 && class != ReadLengthClass::Short {
            let per_read_bases = stats.max_length.max(1);
            block_size = block_size.min((self.opts.max_block_bases / per_read_bases).max(1));
        }

        block_size.max(1)
    }

    fn effective_in_flight_blocks(&self, block_size: usize, stats: &LengthStats) -> usize {
        let budget = auto_memory_budget(self.opts.memory_limit_mb);
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

    fn enforce_archive_mode_memory_limit(
        &self,
        total_reads: usize,
        block_size: usize,
        stats: &LengthStats,
    ) -> Result<()> {
        let strategy = ChunkingStrategy::compute(
            total_reads,
            stats.avg_length.max(1),
            block_size,
            self.effective_threads(),
            self.opts.memory_limit_mb,
        );

        if self.opts.memory_limit_mb > 0 && strategy.requires_chunking() {
            return Err(FqcError::InvalidArgument(format!(
                "--memory-limit {} MB is too small for archive mode with global analysis ({}) ; use --streaming or increase the limit",
                self.opts.memory_limit_mb,
                strategy.summary()
            )));
        }

        if strategy.requires_chunking() {
            log::warn!(
                "Archive mode may exceed the estimated automatic memory budget: {}. Use --streaming for strict memory bounds.",
                strategy.summary()
            );
        }

        Ok(())
    }

    fn length_stats_from_records(&self, records: &[ReadRecord]) -> LengthStats {
        let lengths = self.sample_lengths(records.iter().map(|r| r.sequence.len()));
        Self::build_length_stats(lengths)
    }

    fn inspect_input_lengths(&self) -> Result<Option<LengthStats>> {
        if self.opts.input_path == "-" {
            return Ok(None);
        }

        let sample_limit = self.length_sample_limit();
        let mut lengths = Vec::new();

        if let Some(ref path2) = self.opts.input2_path {
            let mut reader = open_fastq_paired(&self.opts.input_path, path2)?;
            while let Some((r1, r2)) = reader.next_pair()? {
                lengths.push(r1.sequence.len());
                lengths.push(r2.sequence.len());
                if lengths.len() >= sample_limit {
                    break;
                }
            }
        } else {
            let mut parser = open_fastq(&self.opts.input_path)?;
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
            Ok(Some(Self::build_length_stats(lengths)))
        }
    }

    fn sample_lengths<I>(&self, lengths: I) -> Vec<usize>
    where
        I: IntoIterator<Item = usize>,
    {
        let all_lengths: Vec<usize> = lengths.into_iter().collect();
        if self.opts.scan_all_lengths || all_lengths.len() <= DEFAULT_LENGTH_SAMPLE_READS {
            return all_lengths;
        }

        let sample_size = DEFAULT_LENGTH_SAMPLE_READS.min(all_lengths.len());
        (0..sample_size)
            .map(|i| {
                let idx = i * all_lengths.len() / sample_size;
                all_lengths[idx]
            })
            .collect()
    }

    fn build_length_stats(lengths: Vec<usize>) -> LengthStats {
        LengthStats::from_sorted_lengths(lengths)
    }

    fn length_sample_limit(&self) -> usize {
        if self.opts.scan_all_lengths {
            usize::MAX
        } else {
            DEFAULT_LENGTH_SAMPLE_READS
        }
    }

    fn effective_threads(&self) -> usize {
        if self.opts.threads == 0 {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        } else {
            self.opts.threads
        }
    }

    fn print_summary(&self) {
        println!("\n=== Compression Summary ===");
        println!("  Total reads:       {}", self.stats.total_reads);
        println!("  Total bases:       {}", self.stats.total_bases);
        println!("  Blocks written:    {}", self.stats.blocks_written);
        println!("  Output size:       {} bytes", self.stats.output_bytes);
        println!("  Compression ratio: {:.2}x", self.stats.compression_ratio());
        println!("  Bits per base:     {:.3}", self.stats.bits_per_base());
        println!("  Elapsed time:      {:.2} s", self.stats.elapsed_seconds);
        println!("  Throughput:        {:.2} MB/s", self.stats.throughput_mbps());
        println!("===========================");
    }
}
