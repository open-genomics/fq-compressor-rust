// =============================================================================
// fqc-rust - Compression Engine
// =============================================================================
//! Compression execution engine and outcome types.
//!
//! Mode-specific orchestration lives in sibling runner modules. This module
//! owns only the public entry point, validation boundary and shared outcome.

mod common;
mod full_ingest_runner;
mod streaming_runner;

use crate::error::Result;
use crate::plan::compress::{CompressionRequest, ValidatedCompressPlan};
use crate::plan::CompressionExecutionMode;
use crate::stats::OperationStats;
use crate::types::ReadLengthClass;

/// Outcome of a compression operation.
///
/// The outcome carries only operation metadata plus the shared statistics.
/// Counts and byte sizes live in exactly one place: [`OperationStats`].
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionOutcome {
    /// Execution mode used.
    pub mode: CompressionExecutionMode,
    /// Detected read length class.
    pub detected_read_length_class: ReadLengthClass,
    /// Processing statistics.
    pub stats: OperationStats,
}

/// Compression execution engine.
#[derive(Default)]
pub struct CompressionEngine;

impl CompressionEngine {
    /// Create a new compression engine.
    pub fn new() -> Self {
        Self
    }

    /// Validate a request and dispatch to the selected execution runner.
    pub fn run(&self, request: CompressionRequest) -> Result<CompressionOutcome> {
        let plan = ValidatedCompressPlan::new(request)?;
        let request = plan.request();
        let start = std::time::Instant::now();
        let mut outcome = match plan.mode() {
            mode @ (CompressionExecutionMode::Archive | CompressionExecutionMode::Pipeline) => {
                full_ingest_runner::run(request, mode)
            }
            CompressionExecutionMode::Streaming => streaming_runner::run(request),
        }?;
        outcome.stats.elapsed_seconds = start.elapsed().as_secs_f64();
        Ok(outcome)
    }
}
