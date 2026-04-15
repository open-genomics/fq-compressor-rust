// =============================================================================
// fqc - Library crate for integration tests and reuse
// =============================================================================
// Public API fields may not be used internally but are available to consumers
#![allow(dead_code)]

pub mod algo;
pub mod commands;
pub mod common;
pub mod error;
pub mod fastq;
pub mod format;
pub mod fqc_reader;
pub mod fqc_writer;
pub mod io;
pub mod pipeline;
pub mod reorder_map;
pub mod types;
