// =============================================================================
// fqc-rust - Compress Command
// =============================================================================

use crate::engine::compression_request::{CompressionExecutionMode, CompressionInputTopology, CompressionRequest};
use crate::error::{FqcError, Result};
use crate::types::*;
use std::path::PathBuf;

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
            id_mode: IdMode::Tokenize,
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
    /// Normalize CompressOptions into a CompressionRequest.
    ///
    /// This method converts the CLI-facing compression options into the
    /// normalized request type introduced in Task 1. It handles:
    /// - Execution mode selection (streaming > pipeline > archive)
    /// - Input topology detection (stdin, paired, interleaved, single)
    /// - Parameter forwarding to the request
    ///
    /// The normalization keeps current semantics explicit:
    /// - streaming_mode takes priority over use_pipeline
    /// - stdin input is represented with Stdin topology, while execution mode still
    ///   follows the selected flags
    pub fn to_request(&self) -> CompressionRequest {
        // Determine execution mode
        let mode = if self.streaming_mode {
            CompressionExecutionMode::Streaming
        } else if self.use_pipeline {
            CompressionExecutionMode::Pipeline
        } else {
            CompressionExecutionMode::Archive
        };

        // Normalize input topology
        let input = if self.input_path == "-" {
            // Stdin input: if interleaved, include archive_layout
            CompressionInputTopology::Stdin {
                archive_layout: if self.interleaved { Some(self.pe_layout) } else { None },
            }
        } else if let Some(ref input2_path) = self.input2_path {
            // Paired files
            CompressionInputTopology::PairedFiles {
                input_path_r1: PathBuf::from(&self.input_path),
                input_path_r2: PathBuf::from(input2_path),
                archive_layout: self.pe_layout,
            }
        } else if self.interleaved {
            // Interleaved file
            CompressionInputTopology::InterleavedFile {
                input_path: PathBuf::from(&self.input_path),
                archive_layout: self.pe_layout,
            }
        } else {
            // Single file
            CompressionInputTopology::SingleFile {
                input_path: PathBuf::from(&self.input_path),
            }
        };

        CompressionRequest {
            mode,
            input,
            output_path: PathBuf::from(&self.output_path),
            level: self.level,
            quality_mode: self.quality_mode,
            id_mode: self.id_mode,
            enable_reorder: self.enable_reorder,
            requested_read_length_class: self.read_length_class,
            threads: self.threads,
            memory_limit_mb: self.memory_limit_mb,
            force_overwrite: self.force_overwrite,
            show_progress: self.show_progress,
            block_size: self.block_size,
            max_block_bases: self.max_block_bases,
            scan_all_lengths: self.scan_all_lengths,
        }
    }
}

// =============================================================================
// CompressStats
// =============================================================================

use crate::engine::compression_engine::ProcessingStats;

#[derive(Debug, Default)]
struct CompressStats {
    /// Inner processing stats from engine
    inner: ProcessingStats,
    elapsed_seconds: f64,
}

impl CompressStats {
    fn compression_ratio(&self) -> f64 {
        self.inner.compression_ratio()
    }

    fn bits_per_base(&self) -> f64 {
        self.inner.bits_per_base()
    }

    fn throughput_mbps(&self) -> f64 {
        if self.elapsed_seconds == 0.0 {
            return 0.0;
        }
        (self.inner.input_bytes as f64 / 1_048_576.0) / self.elapsed_seconds
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

        // All modes now route through the engine
        use crate::engine::compression_engine::CompressionEngine;
        let request = self.opts.to_request();
        let outcome = CompressionEngine::new().run(request)?;

        // Update stats from outcome
        self.stats.inner = outcome.stats.clone();

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

    fn print_summary(&self) {
        println!("\n=== Compression Summary ===");
        println!("  Total reads:       {}", self.stats.inner.total_reads);
        println!("  Total bases:       {}", self.stats.inner.total_bases);
        println!("  Blocks written:    {}", self.stats.inner.blocks_written);
        println!("  Output size:       {} bytes", self.stats.inner.output_bytes);
        println!("  Compression ratio: {:.2}x", self.stats.compression_ratio());
        println!("  Bits per base:     {:.3}", self.stats.bits_per_base());
        println!("  Elapsed time:      {:.2} s", self.stats.elapsed_seconds);
        println!("  Throughput:        {:.2} MB/s", self.stats.throughput_mbps());
        println!("===========================");
    }
}
