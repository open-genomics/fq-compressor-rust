// =============================================================================
// fqc - High-performance FASTQ compressor
// =============================================================================
//! # fqc - High-performance FASTQ compressor
//!
//! Block-indexed compression for FASTQ files with random access support.
//!
//! ## Features
//! - ABC algorithm for short reads with consensus-based compression
//! - Zstd compression for long reads
//! - Paired-end read handling with automatic detection
//! - Pipe mode for low-memory scenarios
//! - Multiple input formats: gzip, bzip2, xz, zstd
//!
//! ## Commands
//! - `compress` / `c`: Compress FASTQ to .fqc archive
//! - `decompress` / `d`: Decompress .fqc archive to FASTQ
//! - `info` / `i`: Display archive metadata
//! - `verify` / `v`: Verify archive integrity
//!
//! ## Example
//! ```bash
//! fqc compress input.fastq -o archive.fqc
//! fqc decompress archive.fqc -o output.fastq
//! fqc verify archive.fqc
//! ```

#![allow(missing_docs)]
// Public API fields may not be used internally but are available to consumers
#![allow(dead_code)]

pub mod algo;
pub mod archive_traits;
pub mod commands;
pub mod common;
pub mod error;
pub mod fastq;
pub mod format;
pub mod fqc_reader;
pub mod fqc_writer;
pub mod io;
pub mod pipeline;
pub mod types;

pub use algo::compressor_traits::{AuxCompressor, IdCompressor, QualityCompressor, SequenceCompressor};
