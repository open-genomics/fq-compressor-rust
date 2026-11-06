// =============================================================================
// fqc - Archive Module
// =============================================================================
//! The `.fqc` block-indexed archive: binary layout, writer and reader.
//!
//! This module owns only container bytes. Encoded block streams and codec
//! metadata live in [`crate::block`], so `archive` never imports `algo`.

pub mod format;
pub mod reader;
pub mod writer;
