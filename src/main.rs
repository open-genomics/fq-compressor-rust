// =============================================================================
// fqc - High-performance FASTQ compressor with random access support
// =============================================================================
#![allow(dead_code)]

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod algo;
mod commands;
mod common;
mod error;
mod fastq;
mod format;
mod fqc_reader;
mod fqc_writer;
mod io;
mod pipeline;
mod reorder_map;
mod types;

use clap::{Parser, Subcommand};
use commands::compress::{CompressCommand, CompressOptions};
use commands::decompress::{parse_range, DecompressCommand, DecompressOptions};
use commands::info::{InfoCommand, InfoOptions};
use commands::verify::{VerifyCommand, VerifyOptions};
use types::*;

// =============================================================================
// CLI Definitions
// =============================================================================

/// fqc: High-performance FASTQ compressor with random access support.
///
/// The .fqc format achieves 0.4-0.6 bits/base using the ABC algorithm
/// for short reads and Zstd for medium/long reads.
#[derive(Parser, Debug)]
#[command(
    name = "fqc",
    version = "0.1.0",
    about = "High-performance FASTQ compressor with random access support",
    long_about = None,
)]
struct Cli {
    /// Number of threads (0 = auto-detect)
    #[arg(short = 't', long, default_value_t = 0)]
    threads: usize,

    /// Increase verbosity (-v for verbose, -vv for debug)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress non-error output
    #[arg(short = 'q', long)]
    quiet: bool,

    /// Memory limit in MB (0 = no limit)
    #[arg(long, default_value_t = 0)]
    memory_limit: usize,

    /// Disable progress display
    #[arg(long)]
    no_progress: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compress FASTQ file(s) to .fqc format
    #[command(alias = "c")]
    Compress {
        /// Input FASTQ file (use '-' for stdin)
        #[arg(short = 'i', long, required = true)]
        input: String,

        /// Second input FASTQ file for paired-end (R2)
        #[arg(short = '2', long)]
        input2: Option<String>,

        /// Output .fqc file
        #[arg(short = 'o', long, required = true)]
        output: String,

        /// Compression level (1-9)
        #[arg(short = 'l', long, default_value_t = 6, value_parser = clap::value_parser!(u8).range(1..=9))]
        level: u8,

        /// Enable global read reordering (improves compression for short reads)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        reorder: bool,

        /// Streaming mode (disables reordering, suitable for stdin)
        #[arg(long)]
        streaming: bool,

        /// Lossy quality mode: none, illumina8, qvz, discard
        #[arg(long, default_value = "none",
              value_parser = clap::builder::PossibleValuesParser::new(["none", "illumina8", "qvz", "discard"]))]
        lossy_quality: String,

        /// Read length mode: auto, short, medium, long
        #[arg(long, default_value = "auto",
              value_parser = clap::builder::PossibleValuesParser::new(["auto", "short", "medium", "long"]))]
        long_read_mode: String,

        /// Overwrite existing output file
        #[arg(short = 'f', long)]
        force: bool,

        /// Input is interleaved paired-end
        #[arg(long)]
        interleaved: bool,

        /// Maximum bases per block (for long reads, 0 = auto)
        #[arg(long, default_value_t = 0)]
        max_block_bases: usize,

        /// Scan all reads for length detection (slower but more accurate)
        #[arg(long)]
        scan_all_lengths: bool,

        /// Use pipeline mode (3-stage Reader→Compressor→Writer with backpressure)
        #[arg(long)]
        pipeline: bool,

        /// Paired-end storage layout: interleaved, consecutive
        #[arg(long, default_value = "interleaved",
              value_parser = clap::builder::PossibleValuesParser::new(["interleaved", "consecutive"]))]
        pe_layout: String,
    },

    /// Decompress .fqc file to FASTQ
    #[command(alias = "d", alias = "x")]
    Decompress {
        /// Input .fqc file
        #[arg(short = 'i', long, required = true)]
        input: String,

        /// Output FASTQ file (use '-' for stdout)
        #[arg(short = 'o', long, required = true)]
        output: String,

        /// Read range to extract (e.g. '1:1000' or '100:')
        #[arg(long)]
        range: Option<String>,

        /// Output read headers only (IDs)
        #[arg(long)]
        header_only: bool,

        /// Output reads in original order (requires reorder map)
        #[arg(long)]
        original_order: bool,

        /// Skip corrupted blocks instead of failing
        #[arg(long)]
        skip_corrupted: bool,

        /// Placeholder sequence for corrupted reads
        #[arg(long)]
        corrupted_placeholder: Option<String>,

        /// Split paired-end output to separate files
        #[arg(long)]
        split_pe: bool,

        /// Use pipeline mode (3-stage Reader→Decompressor→Writer with backpressure)
        #[arg(long)]
        pipeline: bool,

        /// Overwrite existing output file
        #[arg(short = 'f', long)]
        force: bool,
    },

    /// Display archive information
    #[command(alias = "i")]
    Info {
        /// Input .fqc file
        #[arg(short = 'i', long, required = true)]
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Show detailed block information
        #[arg(long)]
        detailed: bool,

        /// Show codec information for each block
        #[arg(long)]
        show_codecs: bool,
    },

    /// Verify archive integrity
    #[command(alias = "v")]
    Verify {
        /// Input .fqc file
        #[arg(short = 'i', long, required = true)]
        input: String,

        /// Stop on first error
        #[arg(long)]
        fail_fast: bool,

        /// Show detailed verification progress
        #[arg(long)]
        verbose: bool,

        /// Quick mode: only check magic header + footer (skip block decompression)
        #[arg(long)]
        quick: bool,
    },
}

// =============================================================================
// Main
// =============================================================================

fn main() {
    let cli = Cli::parse();

    // Initialize logger
    let log_level = if cli.quiet {
        log::LevelFilter::Error
    } else if cli.verbose >= 2 {
        log::LevelFilter::Debug
    } else if cli.verbose >= 1 {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp(None)
        .format_target(false)
        .init();

    // Configure rayon thread pool
    if cli.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.threads)
            .build_global()
            .unwrap_or_else(|e| log::warn!("Failed to configure thread pool: {e}"));
        log::info!("Thread pool: {} threads", cli.threads);
    }

    let show_progress = !cli.no_progress && !cli.quiet && atty_stdout();

    let exit_code = match cli.command {
        Commands::Compress {
            input,
            input2,
            output,
            level,
            reorder,
            streaming,
            lossy_quality,
            long_read_mode,
            force,
            interleaved,
            max_block_bases,
            scan_all_lengths,
            pipeline,
            pe_layout,
        } => {
            let mut opts = CompressOptions {
                input_path: input,
                input2_path: input2,
                output_path: output,
                level,
                enable_reorder: reorder,
                streaming_mode: streaming,
                quality_mode: parse_quality_mode(&lossy_quality),
                threads: cli.threads,
                memory_limit_mb: if cli.memory_limit == 0 { 8192 } else { cli.memory_limit },
                force_overwrite: force,
                show_progress,
                pe_layout: parse_pe_layout(&pe_layout),
                interleaved,
                max_block_bases,
                scan_all_lengths,
                use_pipeline: pipeline,
                ..CompressOptions::default()
            };

            // Parse long read mode
            match long_read_mode.as_str() {
                "auto" => {
                    opts.auto_detect_length = true;
                    opts.read_length_class = None;
                }
                "short" => {
                    opts.auto_detect_length = false;
                    opts.read_length_class = Some(ReadLengthClass::Short);
                }
                "medium" => {
                    opts.auto_detect_length = false;
                    opts.read_length_class = Some(ReadLengthClass::Medium);
                }
                "long" => {
                    opts.auto_detect_length = false;
                    opts.read_length_class = Some(ReadLengthClass::Long);
                }
                _ => {}
            }

            // stdin implies streaming
            if opts.input_path == "-" && !opts.streaming_mode {
                log::warn!("stdin detected, enabling streaming mode (no reordering)");
                opts.streaming_mode = true;
                opts.enable_reorder = false;
            }

            CompressCommand::new(opts).execute()
        }

        Commands::Decompress {
            input,
            output,
            range,
            header_only,
            original_order,
            skip_corrupted,
            corrupted_placeholder,
            split_pe,
            pipeline,
            force,
        } => {
            let mut opts = DecompressOptions {
                input_path: input,
                output_path: output,
                header_only,
                original_order,
                skip_corrupted,
                corrupted_placeholder,
                split_pe,
                threads: cli.threads,
                show_progress,
                force_overwrite: force,
                use_pipeline: pipeline,
                ..DecompressOptions::default()
            };

            if let Some(r) = range {
                match parse_range(&r) {
                    Ok((start, end)) => {
                        opts.range_start = start;
                        opts.range_end = end;
                    }
                    Err(e) => {
                        eprintln!("Invalid range '{r}': {e}");
                        std::process::exit(1);
                    }
                }
            }

            DecompressCommand::new(opts).execute()
        }

        Commands::Info {
            input,
            json,
            detailed,
            show_codecs,
        } => InfoCommand::new(InfoOptions {
            input_path: input,
            json,
            detailed,
            show_codecs,
        })
        .execute(),

        Commands::Verify {
            input,
            fail_fast,
            verbose,
            quick,
        } => VerifyCommand::new(VerifyOptions {
            input_path: input,
            fail_fast,
            verbose,
            quick_mode: quick,
        })
        .execute(),
    };

    std::process::exit(exit_code);
}

// =============================================================================
// Helpers
// =============================================================================

fn parse_quality_mode(s: &str) -> QualityMode {
    match s {
        "none" | "lossless" => QualityMode::Lossless,
        "illumina8" => QualityMode::Illumina8,
        "qvz" => QualityMode::Qvz,
        "discard" => QualityMode::Discard,
        _ => QualityMode::Lossless,
    }
}

fn parse_pe_layout(s: &str) -> PeLayout {
    match s {
        "consecutive" => PeLayout::Consecutive,
        _ => PeLayout::Interleaved,
    }
}

fn atty_stdout() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}
