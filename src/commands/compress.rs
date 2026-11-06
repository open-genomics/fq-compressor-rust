// =============================================================================
// fqc-rust - Compress Command
// =============================================================================

use crate::error::{FqcError, Result};
use crate::plan::compress::{CompressionInputTopology, CompressionRequest, CompressionSettings};
use crate::plan::{CompressionExecutionMode, ReorderMode};
use crate::println_stdout;
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
    pub reorder: ReorderMode,
    pub streaming_mode: bool,
    pub quality_mode: QualityMode,
    pub id_mode: IdMode,
    pub threads: usize,
    pub memory_limit_mb: usize,
    pub force_overwrite: bool,
    pub show_progress: bool,
    pub read_length_class: Option<ReadLengthClass>,
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
            reorder: ReorderMode::Auto,
            streaming_mode: false,
            quality_mode: QualityMode::Lossless,
            id_mode: IdMode::Tokenize,
            threads: 0,
            memory_limit_mb: 0,
            force_overwrite: false,
            show_progress: true,
            read_length_class: None,
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

        CompressionRequest::new(mode, input, PathBuf::from(&self.output_path)).with_settings(CompressionSettings {
            level: self.level,
            quality_mode: self.quality_mode,
            id_mode: self.id_mode,
            reorder: if mode == CompressionExecutionMode::Streaming {
                ReorderMode::Never
            } else {
                self.reorder
            },
            requested_read_length_class: self.read_length_class,
            threads: self.threads,
            memory_limit_mb: self.memory_limit_mb,
            force_overwrite: self.force_overwrite,
            block_size: self.block_size,
            max_block_bases: self.max_block_bases,
            scan_all_lengths: self.scan_all_lengths,
        })
    }
}

// =============================================================================
// CompressStats
// =============================================================================

use crate::stats::OperationStats;

// =============================================================================
// CompressCommand
// =============================================================================

pub struct CompressCommand {
    opts: CompressOptions,
    stats: OperationStats,
}

impl CompressCommand {
    pub fn new(opts: CompressOptions) -> Self {
        Self {
            opts,
            stats: OperationStats::default(),
        }
    }

    pub fn execute(mut self) -> i32 {
        match self.run() {
            Ok(()) => {
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
        let request = self.opts.to_request();
        let outcome = crate::api::compress(request)?;

        self.stats = outcome.stats;

        Ok(())
    }

    fn validate_options(&self) -> Result<()> {
        if self.opts.input_path != "-" && !std::path::Path::new(&self.opts.input_path).exists() {
            return Err(FqcError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", self.opts.input_path),
            )));
        }
        Ok(())
    }

    fn print_summary(&self) {
        println_stdout!("\n=== Compression Summary ===");
        println_stdout!("  Total reads:       {}", self.stats.total_reads);
        println_stdout!("  Total bases:       {}", self.stats.total_bases);
        println_stdout!("  Blocks written:    {}", self.stats.blocks);
        println_stdout!("  Output size:       {} bytes", self.stats.output_bytes);
        println_stdout!("  Compression ratio: {:.2}x", self.stats.compression_ratio());
        println_stdout!("  Bits per base:     {:.3}", self.stats.bits_per_base());
        println_stdout!("  Elapsed time:      {:.2} s", self.stats.elapsed_seconds);
        println_stdout!("  Throughput:        {:.2} MB/s", self.stats.throughput_mbps());
        println_stdout!(
            "  Stage timings:    parse {:.0} ms | reorder {:.0} ms | process {:.0} ms | write {:.0} ms",
            self.stats.parse_ms as f64,
            self.stats.reorder_ms as f64,
            self.stats.process_ms as f64,
            self.stats.write_ms as f64
        );
        println_stdout!("===========================");
    }
}
