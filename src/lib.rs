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
//!
//! ## Library API
//!
//! The stable embedding surface is [`api`]. Other modules are public for
//! integration tests and advanced callers, but are internal API and may change
//! between minor releases.

pub mod api;

#[doc(hidden)]
pub mod algo;
#[doc(hidden)]
pub mod archive;
#[doc(hidden)]
pub mod block;
#[doc(hidden)]
pub mod commands;
#[doc(hidden)]
pub mod dna;
#[doc(hidden)]
pub mod engine;
#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod fastq;
#[doc(hidden)]
pub mod io;
#[doc(hidden)]
pub mod memory;
#[doc(hidden)]
pub mod pipeline;
#[doc(hidden)]
pub mod plan;
#[doc(hidden)]
pub mod stats;
#[doc(hidden)]
pub mod types;
#[doc(hidden)]
pub mod verify;

pub use algo::compressor_traits::{AuxCompressor, IdCompressor, QualityCompressor, SequenceCompressor};

pub use api::{compress, decompress, info, verify};
