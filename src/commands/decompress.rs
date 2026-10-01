// =============================================================================
// fqc-rust - Decompress Command
// =============================================================================
// Thin CLI layer: option validation and routing. The decompression
// orchestration lives in `pipeline::decompression` (threaded pipeline) and
// `pipeline::decompression_classic` (sequential/split-pe/original-order).
// =============================================================================

use crate::error::{FqcError, Result};
use crate::pipeline::decompression::{DecompressionPipeline, DecompressionPipelineConfig};
use crate::pipeline::decompression_classic::SequentialDecompressor;
use crate::pipeline::PipelineStats;
use crate::types::*;

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
    /// Memory limit in MB (`0` = automatic, still finite).
    pub memory_limit_mb: usize,
}

impl DecompressOptions {
    pub fn placeholder_record(&self, block_id: u32, read_idx: usize) -> ReadRecord {
        let placeholder_seq = self.corrupted_placeholder.clone().unwrap_or_else(|| "N".to_string());
        let quality = "!".repeat(placeholder_seq.len());
        ReadRecord {
            id: format!("corrupted_block{}_read{}", block_id, read_idx),
            comment: String::new(),
            sequence: placeholder_seq,
            quality,
        }
    }
}

// =============================================================================
// DecompressCommand
// =============================================================================

pub struct DecompressCommand {
    opts: DecompressOptions,
}

impl DecompressCommand {
    pub fn new(opts: DecompressOptions) -> Self {
        Self { opts }
    }

    pub fn execute(self) -> i32 {
        let start = std::time::Instant::now();

        match self.run() {
            Ok(stats) => {
                let elapsed = start.elapsed();
                if self.opts.show_progress {
                    print_summary(&stats, elapsed.as_secs_f64());
                }
                0
            }
            Err(e) => {
                eprintln!("Decompression failed: {e}");
                e.exit_code_num()
            }
        }
    }

    fn run(&self) -> Result<PipelineStats> {
        self.validate_options()?;

        let config = DecompressionPipelineConfig {
            num_threads: self.opts.threads,
            range_start: self.opts.range_start,
            range_end: self.opts.range_end,
            original_order: self.opts.original_order,
            header_only: self.opts.header_only,
            skip_corrupted: self.opts.skip_corrupted,
            corrupted_placeholder: self.opts.corrupted_placeholder.clone(),
            force_overwrite: self.opts.force_overwrite,
            split_pe: self.opts.split_pe,
            memory_limit_mb: self.opts.memory_limit_mb,
            ..Default::default()
        };

        // The threaded pipeline covers the plain decompress case; split-pe
        // and original-order restore run on the sequential orchestrator.
        if self.opts.use_pipeline && !self.opts.original_order && !self.opts.split_pe {
            log::info!("Using pipeline decompression mode");
            let mut pipeline = DecompressionPipeline::new(config);
            pipeline.run(&self.opts.input_path, &self.opts.output_path)?;
            Ok(pipeline.stats().clone())
        } else {
            let mut sequential = SequentialDecompressor::new(config);
            sequential.run(&self.opts.input_path, &self.opts.output_path)
        }
    }

    fn validate_options(&self) -> Result<()> {
        if !std::path::Path::new(&self.opts.input_path).exists() {
            return Err(FqcError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", self.opts.input_path),
            )));
        }
        if self.opts.split_pe && self.opts.output_path == "-" {
            return Err(FqcError::InvalidArgument(
                "--split-pe cannot be used with stdout output".to_string(),
            ));
        }
        if self.opts.range_start > 0 && self.opts.range_end > 0 && self.opts.range_start > self.opts.range_end {
            return Err(FqcError::InvalidArgument(format!(
                "Invalid read range: start {} is greater than end {}",
                self.opts.range_start, self.opts.range_end
            )));
        }
        Ok(())
    }
}

fn print_summary(stats: &PipelineStats, elapsed_seconds: f64) {
    println!("\n=== Decompression Summary ===");
    println!("  Total reads:       {}", stats.total_reads);
    println!("  Total bases:       {}", stats.total_bases);
    println!("  Blocks processed:  {}", stats.total_blocks);
    if stats.corrupted_blocks > 0 {
        println!("  Corrupted blocks:  {}", stats.corrupted_blocks);
    }
    println!("  Input size:        {} bytes", stats.input_bytes);
    println!("  Output size:       {} bytes", stats.output_bytes);
    println!("  Elapsed time:      {elapsed_seconds:.2} s");
    let throughput = if elapsed_seconds == 0.0 {
        0.0
    } else {
        (stats.output_bytes as f64 / 1_048_576.0) / elapsed_seconds
    };
    println!("  Throughput:        {throughput:.2} MB/s");
    println!(
        "  Stage timings:    parse {:.0} ms | process {:.0} ms | write {:.0} ms",
        stats.parse_ms as f64, stats.process_ms as f64, stats.write_ms as f64
    );
    println!("=============================");
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
            start_str
                .parse()
                .map_err(|_| FqcError::Parse(format!("Invalid range: {s}")))?
        };

        let end: u64 = if end_str.is_empty() {
            0
        } else {
            end_str
                .parse()
                .map_err(|_| FqcError::Parse(format!("Invalid range: {s}")))?
        };

        Ok((start, end))
    } else {
        let n: u64 = s.parse().map_err(|_| FqcError::Parse(format!("Invalid range: {s}")))?;
        Ok((n, n))
    }
}
