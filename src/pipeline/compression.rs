// =============================================================================
// fqc-rust - Compression Pipeline
// =============================================================================
// All topologies (single / paired / interleaved) share one convergence core:
// `analyze_and_reorder` owns global analysis + reorder-map construction, and
// `write_blocks_parallel` delegates the bounded compress/write stage to the
// shared `block_pipeline` module.
// =============================================================================

use std::path::Path;
use std::time::Instant;

use crate::algo::block_compressor::BlockCompressorConfig;
use crate::algo::global_analyzer::{GlobalAnalyzer, GlobalAnalyzerConfig};
use crate::archive::format::build_flags;
use crate::error::{FqcError, Result};
use crate::fastq::parser::{open_fastq, open_fastq_interleaved, open_fastq_paired, open_fastq_stdin};
use crate::plan::{plan_compression_layout, CompressionPlan, CompressionPlanInput, ReorderMode};
use crate::types::*;

use super::block_pipeline::{self, BlockPipelineConfig};
use super::DEFAULT_MAX_IN_FLIGHT_BLOCKS;
use crate::stats::OperationStats;

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
    pub reorder: ReorderMode,
    pub save_reorder_map: bool,
    pub pe_layout: PeLayout,
    pub max_block_bases: usize,
    pub force_overwrite: bool,
    pub memory_limit_mb: usize,
}

impl Default for CompressionPipelineConfig {
    fn default() -> Self {
        Self {
            num_threads: 0,
            max_in_flight_blocks: DEFAULT_MAX_IN_FLIGHT_BLOCKS,
            block_size: 0,
            read_length_class: ReadLengthClass::Short,
            quality_mode: QualityMode::Lossless,
            id_mode: IdMode::Exact,
            compression_level: DEFAULT_COMPRESSION_LEVEL,
            reorder: ReorderMode::Auto,
            save_reorder_map: true,
            pe_layout: PeLayout::Interleaved,
            max_block_bases: 0,
            force_overwrite: false,
            memory_limit_mb: 0,
        }
    }
}

impl CompressionPipelineConfig {
    fn block_plan(&self, total_reads: usize, total_bases: u64) -> CompressionPlan {
        let avg_length = crate::plan::avg_read_length(total_bases, total_reads as u64);
        let worker_budget = crate::plan::worker_budget_bytes(self.memory_limit_mb);
        let config = self.block_compressor_config();
        plan_compression_layout(CompressionPlanInput {
            requested_block_size: self.block_size,
            avg_length,
            max_block_bases: self.max_block_bases,
            thread_limit: crate::plan::codec_thread_limit(self.num_threads, self.memory_limit_mb),
            worker_budget_bytes: worker_budget,
            total_reads: Some(total_reads as u64),
            total_bases: Some(total_bases),
            read_length_class: self.read_length_class,
            codec_model_bytes: config.to_quality_config().estimated_model_bytes(),
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.block_size > 0 && self.block_size < crate::plan::MIN_BLOCK_SIZE {
            return Err(FqcError::InvalidArgument(format!(
                "Block size {} is too small (min {})",
                self.block_size,
                crate::plan::MIN_BLOCK_SIZE
            )));
        }
        if self.block_size > crate::plan::MAX_BLOCK_SIZE {
            return Err(FqcError::InvalidArgument(format!(
                "Block size {} is too large (max {})",
                self.block_size,
                crate::plan::MAX_BLOCK_SIZE
            )));
        }
        Ok(())
    }

    fn block_compressor_config(&self) -> BlockCompressorConfig {
        BlockCompressorConfig {
            read_length_class: self.read_length_class,
            quality_mode: self.quality_mode,
            id_mode: self.id_mode,
            zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(self.compression_level),
            ..Default::default()
        }
    }
}

// =============================================================================
// Shared convergence helpers
// =============================================================================

fn input_bytes_for_paths(paths: &[&Path], fallback: usize) -> u64 {
    crate::stats::input_bytes_for_paths(paths, fallback as u64)
}

fn fold_read_bytes(reads: &[ReadRecord]) -> (usize, u64) {
    reads.iter().fold((0usize, 0u64), |(bytes, bases), record| {
        (
            bytes + record.id.len() + record.sequence.len() + record.quality.len() + 4,
            bases + record.sequence.len() as u64,
        )
    })
}

fn split_into_chunks(reads: Vec<ReadRecord>, block_size: usize) -> Vec<Vec<ReadRecord>> {
    let block_size = block_size.max(1);
    let mut chunks = Vec::with_capacity(reads.len().div_ceil(block_size));
    let mut reads = reads.into_iter();
    loop {
        let chunk: Vec<ReadRecord> = reads.by_ref().take(block_size).collect();
        if chunk.is_empty() {
            break;
        }
        chunks.push(chunk);
    }
    chunks
}

struct ReorderOutput {
    ordered_reads: Vec<ReadRecord>,
    forward_map: Option<Vec<ReadId>>,
    reverse_map: Option<Vec<ReadId>>,
    reordering_performed: bool,
}

/// Run the global analyzer and build the reordered read vector by move.
/// Single home of the reorder-map construction previously duplicated across
/// run/run_paired/run_interleaved. When `enable` is false (or the analyzer
/// skips reordering, e.g. long reads) reads pass through in input order.
fn analyze_and_reorder(
    all_reads: Vec<ReadRecord>,
    reads_per_block: usize,
    reorder: ReorderMode,
) -> Result<ReorderOutput> {
    if !reorder.enabled() {
        return Ok(ReorderOutput {
            ordered_reads: all_reads,
            forward_map: None,
            reverse_map: None,
            reordering_performed: false,
        });
    }
    let ga_config = GlobalAnalyzerConfig {
        reads_per_block,
        reorder,
        ..Default::default()
    };
    let sequences: Vec<String> = all_reads.iter().map(|r| r.sequence.clone()).collect();
    let analyzer = GlobalAnalyzer::new(ga_config);
    let result = analyzer.analyze(&sequences)?;
    let reordering_performed = result.reordering_performed;
    // reverse_map[archive_id] = original_id. When the analyzer skips
    // reordering (e.g. long reads) the maps are empty and all reads must
    // pass through untouched — otherwise the archive silently drops every
    // read (0 blocks, exit 0). Build by move (mem::take) rather than
    // cloning each record.
    let mut all_reads = all_reads;
    let ordered_reads = if reordering_performed {
        result
            .reverse_map
            .iter()
            .filter_map(|&orig_idx| all_reads.get_mut(orig_idx as usize).map(std::mem::take))
            .collect()
    } else {
        all_reads
    };
    Ok(ReorderOutput {
        ordered_reads,
        forward_map: if reordering_performed {
            Some(result.forward_map)
        } else {
            None
        },
        reverse_map: if reordering_performed {
            Some(result.reverse_map)
        } else {
            None
        },
        reordering_performed,
    })
}

/// Everything the parallel compress-and-write core needs, prepared on the
/// calling thread (parse + reorder already done).
struct PreparedCompression {
    chunks: Vec<Vec<ReadRecord>>,
    flags: u64,
    total_reads: usize,
    input_bytes: u64,
    total_bases: u64,
    planned_workers: usize,
    parse_ms: u64,
    reorder_ms: u64,
    reorder_map: Option<(Vec<ReadId>, Vec<ReadId>)>,
}

// =============================================================================
// CompressionPipeline
// =============================================================================

pub struct CompressionPipeline {
    config: CompressionPipelineConfig,
    stats: OperationStats,
}

impl CompressionPipeline {
    pub fn new(config: CompressionPipelineConfig) -> Self {
        Self {
            config,
            stats: OperationStats::default(),
        }
    }

    pub fn stats(&self) -> &OperationStats {
        &self.stats
    }

    /// Run compression on a single-end input file
    pub fn run(
        &mut self,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        original_filename: &str,
    ) -> Result<()> {
        let input_path = input_path.as_ref();
        self.config.validate()?;
        let start = Instant::now();

        // ---- Phase 1: Read all records (needed for global analysis) ----
        let t_parse = Instant::now();
        let all_reads = if input_path == Path::new("-") {
            let mut parser = open_fastq_stdin();
            parser.collect_all_within_archive_budget(self.config.memory_limit_mb)?
        } else {
            let mut parser = open_fastq(input_path)?;
            parser.collect_all_within_archive_budget(self.config.memory_limit_mb)?
        };

        if all_reads.is_empty() {
            return Err(FqcError::InvalidArgument("Input file is empty".to_string()));
        }
        let parse_ms = t_parse.elapsed().as_millis() as u64;
        let (parsed_bytes, total_bases) = fold_read_bytes(&all_reads);
        let input_bytes = input_bytes_for_paths(&[input_path], parsed_bytes);
        let total_reads = all_reads.len();
        let plan = self.config.block_plan(total_reads, total_bases);
        let block_size = plan.block_size;

        log::info!(
            "Read {} records ({} bytes); pipeline block_size={}, workers={}",
            all_reads.len(),
            input_bytes,
            block_size,
            plan.workers
        );

        // ---- Phase 1b: Global analysis (reordering) ----
        let t_reorder = Instant::now();
        if self.config.reorder.enabled() {
            log::info!("Running global analysis...");
        }
        let reorder = analyze_and_reorder(all_reads, block_size, self.config.reorder)?;
        let reorder_ms = t_reorder.elapsed().as_millis() as u64;

        let ReorderOutput {
            ordered_reads,
            forward_map,
            reverse_map,
            reordering_performed,
        } = reorder;
        let reorder_map_written = reordering_performed && self.config.save_reorder_map;
        let reorder_map = if reorder_map_written {
            Some((
                forward_map.ok_or_else(|| {
                    FqcError::Compression("Reorder map metadata missing despite reorder_map_written=true".to_string())
                })?,
                reverse_map.ok_or_else(|| {
                    FqcError::Compression("Reorder map metadata missing despite reorder_map_written=true".to_string())
                })?,
            ))
        } else {
            None
        };

        // ---- Phase 2+3: parallel compression and ordered write ----
        let flags = build_flags(
            false,
            !reordering_performed,
            self.config.quality_mode,
            self.config.id_mode,
            reorder_map_written,
            self.config.pe_layout,
            self.config.read_length_class,
            false,
        );

        let prepared = PreparedCompression {
            chunks: split_into_chunks(ordered_reads, block_size),
            flags,
            total_reads,
            input_bytes,
            total_bases,
            planned_workers: plan.workers,
            parse_ms,
            reorder_ms,
            reorder_map,
        };
        self.write_blocks_parallel(prepared, start, output_path, original_filename)
    }

    /// Run compression on paired-end input files
    pub fn run_paired(
        &mut self,
        input1_path: impl AsRef<Path>,
        input2_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        original_filename: &str,
        pe_layout: PeLayout,
    ) -> Result<()> {
        let input1_path = input1_path.as_ref();
        let input2_path = input2_path.as_ref();
        self.config.validate()?;
        let start = Instant::now();

        log::info!(
            "Reading paired-end files: {} + {}",
            input1_path.display(),
            input2_path.display()
        );
        let t_parse = Instant::now();
        let mut pe_reader = open_fastq_paired(input1_path, input2_path)?;
        let all_reads = pe_reader.collect_pairs_within_archive_budget(pe_layout, self.config.memory_limit_mb)?;
        if all_reads.is_empty() {
            return Err(FqcError::InvalidArgument("Input files are empty".to_string()));
        }
        let parse_ms = t_parse.elapsed().as_millis() as u64;

        let (parsed_bytes, _) = fold_read_bytes(&all_reads);
        let input_bytes = input_bytes_for_paths(&[input1_path, input2_path], parsed_bytes);
        self.run_pair_based(
            all_reads,
            input_bytes,
            parse_ms,
            start,
            output_path,
            original_filename,
            pe_layout,
        )
    }

    /// Run compression on interleaved paired-end input in a single file.
    pub fn run_interleaved(
        &mut self,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        original_filename: &str,
        pe_layout: PeLayout,
    ) -> Result<()> {
        let input_path = input_path.as_ref();
        self.config.validate()?;
        let start = Instant::now();

        log::info!("Reading interleaved paired-end file: {}", input_path.display());
        let t_parse = Instant::now();
        let mut parser = if input_path == Path::new("-") {
            crate::fastq::parser::InterleavedPeParser::new(open_fastq_stdin())
        } else {
            open_fastq_interleaved(input_path)?
        };
        let all_reads = parser.collect_pairs_within_archive_budget(pe_layout, self.config.memory_limit_mb)?;
        if all_reads.is_empty() {
            return Err(FqcError::InvalidArgument("Input file is empty".to_string()));
        }
        let parse_ms = t_parse.elapsed().as_millis() as u64;

        let (parsed_bytes, _) = fold_read_bytes(&all_reads);
        let input_bytes = input_bytes_for_paths(&[input_path], parsed_bytes);
        self.run_pair_based(
            all_reads,
            input_bytes,
            parse_ms,
            start,
            output_path,
            original_filename,
            pe_layout,
        )
    }

    /// Shared tail of the paired topologies: global analysis, then the same
    /// parallel compress-and-write core as single-end. Replaces the former
    /// serial single-compressor loops (behavior: identical block content and
    /// order; wall time now on par with the other topologies).
    fn run_pair_based(
        &mut self,
        all_reads: Vec<ReadRecord>,
        input_bytes: u64,
        parse_ms: u64,
        start: Instant,
        output_path: impl AsRef<Path>,
        original_filename: &str,
        pe_layout: PeLayout,
    ) -> Result<()> {
        let (_, total_bases) = fold_read_bytes(&all_reads);
        let total_reads = all_reads.len();
        let plan = self.config.block_plan(total_reads, total_bases);
        let block_size = plan.block_size;
        let planned_workers = plan.workers;

        let t_reorder = Instant::now();
        let reorder = analyze_and_reorder(all_reads, block_size, self.config.reorder)?;
        let reorder_ms = t_reorder.elapsed().as_millis() as u64;

        let ReorderOutput {
            ordered_reads,
            forward_map,
            reverse_map,
            reordering_performed,
        } = reorder;
        let reorder_map_written = reordering_performed && self.config.save_reorder_map;
        let reorder_map = if reorder_map_written {
            Some((
                forward_map.ok_or_else(|| {
                    FqcError::Compression("Reorder map metadata missing despite reorder_map_written=true".to_string())
                })?,
                reverse_map.ok_or_else(|| {
                    FqcError::Compression("Reorder map metadata missing despite reorder_map_written=true".to_string())
                })?,
            ))
        } else {
            None
        };

        let flags = build_flags(
            true,
            !reordering_performed,
            self.config.quality_mode,
            self.config.id_mode,
            reorder_map_written,
            pe_layout,
            self.config.read_length_class,
            false,
        );

        let prepared = PreparedCompression {
            chunks: split_into_chunks(ordered_reads, block_size),
            flags,
            total_reads,
            input_bytes,
            total_bases,
            planned_workers,
            parse_ms,
            reorder_ms,
            reorder_map,
        };
        self.write_blocks_parallel(prepared, start, output_path, original_filename)
    }

    /// Shared tail: hand ordered chunks to the bounded block pipeline.
    fn write_blocks_parallel(
        &mut self,
        prepared: PreparedCompression,
        start: Instant,
        output_path: impl AsRef<Path>,
        original_filename: &str,
    ) -> Result<()> {
        let PreparedCompression {
            chunks,
            flags,
            total_reads,
            input_bytes,
            total_bases,
            planned_workers,
            parse_ms,
            reorder_ms,
            reorder_map,
        } = prepared;
        let reorder_map_written = reorder_map.is_some();

        let report = block_pipeline::run(
            chunks,
            output_path,
            BlockPipelineConfig {
                num_workers: planned_workers,
                max_in_flight_blocks: self.config.max_in_flight_blocks,
                force_overwrite: self.config.force_overwrite,
                flags,
                total_reads: total_reads as u64,
                original_filename: original_filename.to_string(),
                reorder_map,
                compressor_config: self.config.block_compressor_config(),
            },
        )?;

        self.stats = OperationStats {
            total_reads: total_reads as u64,
            total_bases,
            blocks: report.blocks_written,
            input_bytes,
            output_bytes: report.output_bytes,
            elapsed_seconds: start.elapsed().as_secs_f64(),
            reorder_map_written,
            corrupted_blocks: 0,
            parse_ms,
            reorder_ms,
            process_ms: report.process_ms,
            write_ms: report.write_ms,
        };

        log::info!(
            "Compression complete: {} reads, {} blocks, {:.2}x ratio, {:.1} MB/s",
            self.stats.total_reads,
            self.stats.blocks,
            self.stats.compression_ratio(),
            self.stats.throughput_mbps(),
        );

        Ok(())
    }
}
