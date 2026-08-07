// =============================================================================
// Archive Traits - Shared Data Types
// =============================================================================
//! Shared data types for archive reading/writing (block payload containers).

use crate::archive::format::BlockHeader;

// =============================================================================
// BlockData
// =============================================================================

/// Raw decompressed streams for a block.
///
/// This is the data read from the archive before decompression/decoding.
#[derive(Debug, Default)]
pub struct BlockData {
    pub header: BlockHeader,
    pub ids_data: Vec<u8>,
    pub seq_data: Vec<u8>,
    pub qual_data: Vec<u8>,
    pub aux_data: Vec<u8>,
}
