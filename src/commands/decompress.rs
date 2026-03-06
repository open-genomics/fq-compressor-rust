// =============================================================================
// fqc-rust - Decompress Command
// =============================================================================

use crate::algo::block_compressor::BlockCompressor;
use crate::error::{FqcError, Result};
use crate::fastq::parser::write_record;
use crate::format::{get_id_mode, get_quality_mode, get_read_length_class};
use crate::fqc_reader::FqcReader;
use crate::types::*;
use std::fs::File;
use std::io::{BufWriter, Write};

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
                1
            }
        }
    }

    fn run(&mut self) -> Result<()> {
        self.validate_options()?;

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
        let block_config = crate::algo::block_compressor::BlockCompressorConfig {
            read_length_class: get_read_length_class(flags),
            quality_mode: get_quality_mode(flags),
            id_mode: get_id_mode(flags),
            ..Default::default()
        };

        let compressor = BlockCompressor::new(block_config);

        // Open output
        let mut output: Box<dyn Write> = if self.opts.output_path == "-" {
            Box::new(std::io::stdout())
        } else {
            if !self.opts.force_overwrite && std::path::Path::new(&self.opts.output_path).exists() {
                return Err(FqcError::InvalidArgument(format!(
                    "Output file already exists: {} (use -f to overwrite)",
                    self.opts.output_path
                )));
            }
            let f = File::create(&self.opts.output_path)?;
            Box::new(BufWriter::new(f))
        };

        // Process blocks
        let block_count = reader.block_count();

        if self.opts.original_order && reader.reorder_forward.is_some() {
            // Original order mode: buffer all reads, then output in original order
            self.run_original_order(&mut reader, &compressor, block_count, &mut output)?;
        } else {
            // Normal mode: stream blocks directly to output
            let mut global_read_idx = 0u64;
            for block_id in 0..block_count {
                log::debug!("Processing block {}/{}", block_id + 1, block_count);

                match self.process_block(&mut reader, &compressor, block_id as u32, &mut global_read_idx, &mut output) {
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

    fn process_block(
        &mut self,
        reader: &mut FqcReader,
        compressor: &BlockCompressor,
        block_id: u32,
        global_read_idx: &mut u64,
        output: &mut dyn Write,
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
            // Range filtering
            let current_id = *global_read_idx + 1; // 1-based
            *global_read_idx += 1;

            if self.opts.range_end > 0 {
                if current_id < self.opts.range_start || current_id > self.opts.range_end {
                    continue;
                }
            } else if self.opts.range_start > 0 && current_id < self.opts.range_start {
                continue;
            }

            // Write record
            if self.opts.header_only {
                writeln!(output, "@{}", read.id)?;
            } else {
                write_record(output, read)?;
            }

            self.stats.total_reads += 1;
            self.stats.total_bases += read.sequence.len() as u64;
            self.stats.output_bytes += read.id.len() as u64
                + read.sequence.len() as u64
                + read.quality.len() as u64
                + 4; // @, +, 2x\n
        }

        Ok(())
    }

    /// Decompress all blocks, then output reads in original order using the reorder map.
    fn run_original_order(
        &mut self,
        reader: &mut FqcReader,
        compressor: &BlockCompressor,
        block_count: usize,
        output: &mut dyn Write,
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
        for original_id in 0..total_reads {
            let archive_id = forward_map[original_id] as usize;
            if archive_id >= all_reads.len() { continue; }

            let read = &all_reads[archive_id];

            if self.opts.header_only {
                writeln!(output, "@{}", read.id)?;
            } else {
                write_record(output, read)?;
            }

            self.stats.total_reads += 1;
            self.stats.total_bases += read.sequence.len() as u64;
            self.stats.output_bytes += read.id.len() as u64
                + read.sequence.len() as u64
                + read.quality.len() as u64 + 4;
        }

        Ok(())
    }

    fn validate_options(&self) -> Result<()> {
        if !std::path::Path::new(&self.opts.input_path).exists() {
            return Err(FqcError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", self.opts.input_path),
            )));
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
