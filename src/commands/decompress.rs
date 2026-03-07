// =============================================================================
// fqc-rust - Decompress Command
// =============================================================================

use crate::algo::block_compressor::{BlockCompressor, BlockCompressorConfig, DecompressedBlockData};
use crate::error::{FqcError, Result};
use crate::fastq::parser::write_record as write_fastq_record;
use crate::format::{flags, get_id_mode, get_pe_layout, get_quality_mode, get_read_length_class};
use crate::fqc_reader::{FqcReader, BlockData};
use crate::pipeline::decompression::{DecompressionPipeline, DecompressionPipelineConfig};
use crate::types::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Arc;

// =============================================================================
// DecompressOptions
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct DecompressOptions {
    pub input_path: String,
    pub output_path: String,
    pub range_start: u64,
    pub range_end: u64,
    pub header_only: bool,
    pub original_order: bool,
    pub skip_corrupted: bool,
    pub corrupted_placeholder: Option<String>,
    pub split_pe: bool,
    pub threads: usize,
    pub show_progress: bool,
    pub force_overwrite: bool,
    pub use_pipeline: bool,
}

impl DecompressOptions {
    pub fn placeholder_record(&self, block_id: u32, read_idx: usize) -> ReadRecord {
        let placeholder_seq = self.corrupted_placeholder.clone()
            .unwrap_or_else(|| "N".to_string());
        ReadRecord {
            id: format!("corrupted_block{}_read{}", block_id, read_idx),
            sequence: placeholder_seq.clone(),
            quality: "!".repeat(placeholder_seq.len()),
        }
    }
}

// =============================================================================
// DecompressStats
// =============================================================================

#[derive(Debug, Default)]
struct DecompressStats {
    total_reads: u64,
    total_bases: u64,
    blocks_processed: u64,
    corrupted_blocks: u64,
    input_bytes: u64,
    output_bytes: u64,
    elapsed_seconds: f64,
}

enum OutputWriters {
    Single(Box<dyn Write>),
    Split {
        r1: Box<dyn Write>,
        r2: Box<dyn Write>,
        pe_layout: PeLayout,
    },
}

impl OutputWriters {
    fn write_record(
        &mut self,
        read: &ReadRecord,
        header_only: bool,
        zero_based_read_idx: u64,
        total_archive_reads: u64,
    ) -> Result<u64> {
        match self {
            Self::Single(output) => write_to_target(output.as_mut(), read, header_only),
            Self::Split { r1, r2, pe_layout } => {
                let to_r1 = match pe_layout {
                    PeLayout::Interleaved => zero_based_read_idx.is_multiple_of(2),
                    PeLayout::Consecutive => zero_based_read_idx < (total_archive_reads / 2),
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
            Self::Single(output) => output.flush().map_err(FqcError::Io),
            Self::Split { r1, r2, .. } => {
                r1.flush().map_err(FqcError::Io)?;
                r2.flush().map_err(FqcError::Io)
            }
        }
    }
}

fn write_to_target(output: &mut dyn Write, read: &ReadRecord, header_only: bool) -> Result<u64> {
    if header_only {
        writeln!(output, "@{}", read.id)?;
        Ok((read.id.len() + 2) as u64)
    } else {
        write_fastq_record(output, read)?;
        Ok(read.id.len() as u64 + read.sequence.len() as u64 + read.quality.len() as u64 + 4)
    }
}

impl DecompressStats {
    fn throughput_mbps(&self) -> f64 {
        if self.elapsed_seconds == 0.0 { return 0.0; }
        (self.output_bytes as f64 / 1_048_576.0) / self.elapsed_seconds
    }
}

// =============================================================================
// DecompressCommand
// =============================================================================

pub struct DecompressCommand {
    opts: DecompressOptions,
    stats: DecompressStats,
}

impl DecompressCommand {
    pub fn new(opts: DecompressOptions) -> Self {
        Self { opts, stats: DecompressStats::default() }
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
                eprintln!("Decompression failed: {e}");
                e.exit_code_num()
            }
        }
    }

    fn run(&mut self) -> Result<()> {
        self.validate_options()?;

        // Pipeline mode
        if self.opts.use_pipeline && !self.opts.original_order && !self.opts.split_pe {
            return self.run_pipeline();
        }

        // Open archive
        let mut reader = FqcReader::open(&self.opts.input_path)?;

        log::info!("Archive: {} reads, {} blocks",
            reader.total_read_count(),
            reader.block_count()
        );

        // Load reorder map if needed
        if self.opts.original_order {
            if !reader.has_reorder_map() {
                return Err(FqcError::Format("Original order requested but no reorder map present".to_string()));
            }
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

        let compressor = BlockCompressor::new(block_config.clone());

        let is_paired = (flags & flags::IS_PAIRED) != 0;
        let total_archive_reads = reader.total_read_count();

        // Open output
        let mut output = if self.opts.split_pe {
            if !is_paired {
                return Err(FqcError::InvalidArgument("--split-pe requires a paired-end archive".to_string()));
            }
            if self.opts.output_path == "-" {
                return Err(FqcError::InvalidArgument("--split-pe cannot be used with stdout output".to_string()));
            }

            let (r1_path, r2_path) = derive_split_output_paths(&self.opts.output_path);
            if !self.opts.force_overwrite {
                if std::path::Path::new(&r1_path).exists() {
                    return Err(FqcError::InvalidArgument(format!(
                        "Output file already exists: {} (use -f to overwrite)",
                        r1_path
                    )));
                }
                if std::path::Path::new(&r2_path).exists() {
                    return Err(FqcError::InvalidArgument(format!(
                        "Output file already exists: {} (use -f to overwrite)",
                        r2_path
                    )));
                }
            }

            OutputWriters::Split {
                r1: Box::new(BufWriter::new(File::create(&r1_path)?)),
                r2: Box::new(BufWriter::new(File::create(&r2_path)?)),
                pe_layout: get_pe_layout(flags),
            }
        } else if self.opts.output_path == "-" {
            OutputWriters::Single(Box::new(std::io::stdout()))
        } else {
            if !self.opts.force_overwrite && std::path::Path::new(&self.opts.output_path).exists() {
                return Err(FqcError::InvalidArgument(format!(
                    "Output file already exists: {} (use -f to overwrite)",
                    self.opts.output_path
                )));
            }
            let f = File::create(&self.opts.output_path)?;
            OutputWriters::Single(Box::new(BufWriter::new(f)))
        };

        // Process blocks
        let block_count = reader.block_count();

        if self.opts.original_order && reader.reorder_forward.is_some() {
            // Original order mode: buffer all reads, then output in original order
            self.run_original_order(&mut reader, &compressor, block_count, total_archive_reads, &mut output)?;
        } else if block_count > 1 && self.opts.threads != 1 {
            // Parallel decompression: read blocks sequentially, decompress in parallel, write sequentially
            self.run_parallel(&mut reader, &block_config, block_count, total_archive_reads, &mut output)?;
        } else {
            // Normal mode: stream blocks directly to output
            let mut global_read_idx = 0u64;
            for block_id in 0..block_count {
                log::debug!("Processing block {}/{}", block_id + 1, block_count);

                match self.process_block(&mut reader, &compressor, block_id as u32, total_archive_reads, &mut global_read_idx, &mut output) {
                    Ok(()) => {
                        self.stats.blocks_processed += 1;
                    }
                    Err(e) => {
                        if self.opts.skip_corrupted {
                            log::warn!("Block {} failed, skipping: {}", block_id, e);
                            self.stats.corrupted_blocks += 1;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }

        // Get input file size
        if let Ok(meta) = std::fs::metadata(&self.opts.input_path) {
            self.stats.input_bytes = meta.len();
        }

        output.flush()?;

        log::info!("Decompression complete: {} reads, {} blocks", self.stats.total_reads, self.stats.blocks_processed);
        Ok(())
    }

    /// Parallel decompression: read blocks sequentially, decompress in parallel batches, write sequentially.
    fn run_parallel(
        &mut self,
        reader: &mut FqcReader,
        block_config: &BlockCompressorConfig,
        block_count: usize,
        total_archive_reads: u64,
        output: &mut OutputWriters,
    ) -> Result<()> {
        log::info!("Using parallel decompression ({} blocks)", block_count);

        let batch_size = (self.opts.threads.max(1) * 2).max(4).min(block_count);
        let config = Arc::new(block_config.clone());
        let skip_corrupted = self.opts.skip_corrupted;

        let mut global_read_idx = 0u64;
        let mut block_start = 0usize;

        while block_start < block_count {
            let batch_end = (block_start + batch_size).min(block_count);

            // Phase 1: Read block data sequentially
            let mut block_data_vec: Vec<(u32, BlockData)> = Vec::with_capacity(batch_end - block_start);
            for block_id in block_start..batch_end {
                match reader.read_block(block_id as u32) {
                    Ok(bd) => block_data_vec.push((block_id as u32, bd)),
                    Err(e) => {
                        if skip_corrupted {
                            log::warn!("Block {} read failed, skipping: {}", block_id, e);
                            self.stats.corrupted_blocks += 1;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            // Phase 2: Decompress in parallel
            let cfg = Arc::clone(&config);
            let results: Vec<std::result::Result<(u32, DecompressedBlockData), (u32, String)>> =
                block_data_vec.into_par_iter().map(|(bid, bd)| {
                    let comp = BlockCompressor::new((*cfg).clone());
                    let bh = &bd.header;
                    match comp.decompress_raw(
                        bh.block_id, bh.uncompressed_count, bh.uniform_read_length,
                        bh.codec_seq, bh.codec_qual,
                        &bd.ids_data, &bd.seq_data, &bd.qual_data, &bd.aux_data,
                    ) {
                        Ok(dec) => Ok((bid, dec)),
                        Err(e) => Err((bid, format!("{}", e))),
                    }
                }).collect();

            // Phase 3: Write results sequentially (sorted by block_id)
            let mut sorted: Vec<_> = results;
            sorted.sort_by_key(|r| match r {
                Ok((bid, _)) => *bid,
                Err((bid, _)) => *bid,
            });

            for result in sorted {
                match result {
                    Ok((_bid, decompressed)) => {
                        for read in &decompressed.reads {
                            self.emit_read(output, read, global_read_idx, total_archive_reads)?;
                            global_read_idx += 1;
                        }
                        self.stats.blocks_processed += 1;
                    }
                    Err((bid, msg)) => {
                        if skip_corrupted {
                            log::warn!("Block {} decompress failed, skipping: {}", bid, msg);
                            self.stats.corrupted_blocks += 1;
                        } else {
                            return Err(FqcError::Decompression(format!("Block {} failed: {}", bid, msg)));
                        }
                    }
                }
            }

            block_start = batch_end;
        }

        Ok(())
    }

    fn process_block(
        &mut self,
        reader: &mut FqcReader,
        compressor: &BlockCompressor,
        block_id: u32,
        total_archive_reads: u64,
        global_read_idx: &mut u64,
        output: &mut OutputWriters,
    ) -> Result<()> {
        let block_data = reader.read_block(block_id)?;
        let bh = &block_data.header;

        let decompressed = compressor.decompress_raw(
            bh.block_id,
            bh.uncompressed_count,
            bh.uniform_read_length,
            bh.codec_seq,
            bh.codec_qual,
            &block_data.ids_data,
            &block_data.seq_data,
            &block_data.qual_data,
            &block_data.aux_data,
        )?;

        for read in &decompressed.reads {
            let read_idx = *global_read_idx;
            *global_read_idx += 1;
            self.emit_read(output, read, read_idx, total_archive_reads)?;
        }

        Ok(())
    }

    /// Decompress all blocks, then output reads in original order using the reorder map.
    fn run_original_order(
        &mut self,
        reader: &mut FqcReader,
        compressor: &BlockCompressor,
        block_count: usize,
        total_archive_reads: u64,
        output: &mut OutputWriters,
    ) -> Result<()> {
        log::info!("Restoring original read order...");

        let forward_map = reader.reorder_forward.clone()
            .ok_or_else(|| FqcError::Format("Forward reorder map missing".to_string()))?;
        let total_reads = forward_map.len();

        // Buffer all reads in archive order
        let mut all_reads: Vec<ReadRecord> = Vec::with_capacity(total_reads);

        for block_id in 0..block_count {
            let block_data = reader.read_block(block_id as u32)?;
            let bh = &block_data.header;

            match compressor.decompress_raw(
                bh.block_id, bh.uncompressed_count, bh.uniform_read_length,
                bh.codec_seq, bh.codec_qual,
                &block_data.ids_data, &block_data.seq_data,
                &block_data.qual_data, &block_data.aux_data,
            ) {
                Ok(decompressed) => {
                    all_reads.extend(decompressed.reads);
                    self.stats.blocks_processed += 1;
                }
                Err(e) => {
                    if self.opts.skip_corrupted {
                        log::warn!("Block {} failed, skipping: {}", block_id, e);
                        self.stats.corrupted_blocks += 1;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        // Reorder: forward_map[original_id] = archive_id
        // So to output in original order, iterate original_id 0..N
        // and output all_reads[forward_map[original_id]]
        for (original_id, &fwd) in forward_map.iter().enumerate().take(total_reads) {
            let archive_id = fwd as usize;
            if archive_id >= all_reads.len() { continue; }

            let read = &all_reads[archive_id];
            self.emit_read(output, read, original_id as u64, total_archive_reads)?;
        }

        Ok(())
    }

    fn emit_read(
        &mut self,
        output: &mut OutputWriters,
        read: &ReadRecord,
        zero_based_read_idx: u64,
        total_archive_reads: u64,
    ) -> Result<()> {
        let current_id = zero_based_read_idx + 1;

        if self.opts.range_end > 0 {
            if current_id < self.opts.range_start || current_id > self.opts.range_end {
                return Ok(());
            }
        } else if self.opts.range_start > 0 && current_id < self.opts.range_start {
            return Ok(());
        }

        let bytes_written = output.write_record(read, self.opts.header_only, zero_based_read_idx, total_archive_reads)?;
        self.stats.total_reads += 1;
        self.stats.total_bases += read.sequence.len() as u64;
        self.stats.output_bytes += bytes_written;
        Ok(())
    }

    /// Pipeline mode: 3-stage Reader→Decompressor→Writer with backpressure
    fn run_pipeline(&mut self) -> Result<()> {
        log::info!("Using pipeline decompression mode");

        if self.opts.output_path != "-" && !self.opts.force_overwrite
            && std::path::Path::new(&self.opts.output_path).exists()
        {
            return Err(FqcError::InvalidArgument(format!(
                "Output file already exists: {} (use -f to overwrite)",
                self.opts.output_path
            )));
        }

        let pipeline_config = DecompressionPipelineConfig {
            num_threads: self.opts.threads,
            range_start: self.opts.range_start,
            range_end: self.opts.range_end,
            original_order: false,
            header_only: self.opts.header_only,
            skip_corrupted: self.opts.skip_corrupted,
            split_pe: false,
            ..Default::default()
        };

        let mut pipeline = DecompressionPipeline::new(pipeline_config);
        pipeline.run(&self.opts.input_path, &self.opts.output_path, None)?;

        let stats = pipeline.stats();
        self.stats.total_reads = stats.total_reads;
        self.stats.blocks_processed = stats.total_blocks as u64;
        self.stats.output_bytes = stats.output_bytes;
        self.stats.input_bytes = stats.input_bytes;

        log::info!("Pipeline decompression complete! {} reads, {:.1} MB/s",
            stats.total_reads, stats.throughput_mbps());
        Ok(())
    }

    fn validate_options(&self) -> Result<()> {
        if !std::path::Path::new(&self.opts.input_path).exists() {
            return Err(FqcError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", self.opts.input_path),
            )));
        }
        if self.opts.split_pe && self.opts.output_path == "-" {
            return Err(FqcError::InvalidArgument("--split-pe cannot be used with stdout output".to_string()));
        }
        Ok(())
    }

    fn print_summary(&self) {
        println!("\n=== Decompression Summary ===");
        println!("  Total reads:       {}", self.stats.total_reads);
        println!("  Total bases:       {}", self.stats.total_bases);
        println!("  Blocks processed:  {}", self.stats.blocks_processed);
        if self.stats.corrupted_blocks > 0 {
            println!("  Corrupted blocks:  {}", self.stats.corrupted_blocks);
        }
        println!("  Input size:        {} bytes", self.stats.input_bytes);
        println!("  Output size:       {} bytes", self.stats.output_bytes);
        println!("  Elapsed time:      {:.2} s", self.stats.elapsed_seconds);
        println!("  Throughput:        {:.2} MB/s", self.stats.throughput_mbps());
        println!("=============================");
    }
}

fn derive_split_output_paths(output_path: &str) -> (String, String) {
    if let Some(dot_pos) = output_path.rfind('.') {
        let (base, ext) = output_path.split_at(dot_pos);
        (format!("{}_R1{}", base, ext), format!("{}_R2{}", base, ext))
    } else {
        (format!("{}_R1", output_path), format!("{}_R2", output_path))
    }
}

// =============================================================================
// Range Parsing
// =============================================================================

pub fn parse_range(s: &str) -> Result<(u64, u64)> {
    if s.is_empty() {
        return Ok((0, 0));
    }

    if let Some(colon_pos) = s.find(':') {
        let start_str = &s[..colon_pos];
        let end_str = &s[colon_pos + 1..];

        let start: u64 = if start_str.is_empty() {
            1
        } else {
            start_str.parse().map_err(|_| FqcError::Parse(format!("Invalid range: {s}")))?
        };

        let end: u64 = if end_str.is_empty() {
            0
        } else {
            end_str.parse().map_err(|_| FqcError::Parse(format!("Invalid range: {s}")))?
        };

        Ok((start, end))
    } else {
        let n: u64 = s.parse().map_err(|_| FqcError::Parse(format!("Invalid range: {s}")))?;
        Ok((n, n))
    }
}
