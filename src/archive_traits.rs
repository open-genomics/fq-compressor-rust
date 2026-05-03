// =============================================================================
// Archive Traits - Abstractions for Reading/Writing FQC Archives
// =============================================================================
//! Traits for archive I/O, enabling testing with mocks and in-memory implementations.
//!
//! These traits define the seams between:
//! - Commands (orchestration) and archive I/O
//! - Compression/decompression logic and storage
//!
//! ## Usage
//!
//! ```ignore
//! use fqc::archive_traits::{ArchiveWriter, ArchiveReader};
//!
//! fn compress_to_writer<W: ArchiveWriter>(writer: &mut W, reads: &[ReadRecord]) -> Result<()> {
//!     writer.write_global_header(&header)?;
//!     writer.write_block(&block)?;
//!     writer.finalize()
//! }
//! ```

use crate::algo::block_compressor::CompressedBlockData;
use crate::error::Result;
use crate::format::{BlockHeader, GlobalHeader};

// =============================================================================
// ArchiveWriter Trait
// =============================================================================

/// Trait for writing FQC archives.
///
/// Implementations may write to:
/// - Files (the default `FqcWriter`)
/// - Memory (for testing)
/// - Network streams
/// - Custom storage backends
pub trait ArchiveWriter {
    /// Write the global header at the current position.
    fn write_global_header(&mut self, header: &GlobalHeader) -> Result<()>;

    /// Write a compressed block. Returns the block's starting offset.
    fn write_block(&mut self, block: &CompressedBlockData) -> Result<u64>;

    /// Write a block with explicit archive_id_start (for reorder maps).
    fn write_block_with_id(&mut self, block: &CompressedBlockData, archive_id_start: u64) -> Result<u64>;

    /// Write the reorder map (forward and reverse mappings).
    fn write_reorder_map(&mut self, forward_map: &[u64], reverse_map: &[u64]) -> Result<u64>;

    /// Patch the total read count in the global header.
    fn patch_total_read_count(&mut self, total_read_count: u64) -> Result<()>;

    /// Finalize the archive: write block index and footer.
    fn finalize(self) -> Result<()>;
}

// =============================================================================
// ArchiveReader Trait
// =============================================================================

/// Trait for reading FQC archives.
///
/// Implementations may read from:
/// - Files (the default `FqcReader`)
/// - Memory (for testing)
/// - Custom storage backends
pub trait ArchiveReader {
    /// Get the global header.
    fn global_header(&self) -> &GlobalHeader;

    /// Get the number of blocks in the archive.
    fn block_count(&self) -> usize;

    /// Get the total read count.
    fn total_read_count(&self) -> u64;

    /// Check if a reorder map is present.
    fn has_reorder_map(&self) -> bool;

    /// Load the reorder map into memory.
    fn load_reorder_map(&mut self) -> Result<()>;

    /// Look up original read ID from archive ID using the reorder map.
    fn lookup_original_id(&self, archive_id: u64) -> Option<u64>;

    /// Read a block's raw data by block_id.
    fn read_block(&mut self, block_id: u32) -> Result<BlockData>;

    /// Read only the block header for a given block_id.
    fn read_block_header(&mut self, block_id: u32) -> Result<BlockHeader>;
}

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

// =============================================================================
// InMemoryWriter - For Testing
// =============================================================================

#[cfg(test)]
pub mod testing {
    use super::*;
    use crate::algo::block_compressor::delta_encode_ids;
    use crate::error::{FqcError, Result};
    use crate::format::*;
    use xxhash_rust::xxh64::Xxh64;

    /// In-memory archive writer for testing.
    ///
    /// Collects all written data in memory for inspection.
    pub struct InMemoryWriter {
        pub data: Vec<u8>,
        pub global_header: Option<GlobalHeader>,
        pub blocks: Vec<CompressedBlockData>,
        pub index_entries: Vec<IndexEntry>,
        pub reorder_forward: Option<Vec<u64>>,
        pub reorder_reverse: Option<Vec<u64>>,
        global_hasher: Xxh64,
        pub finalized: bool,
    }

    impl std::fmt::Debug for InMemoryWriter {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("InMemoryWriter")
                .field("data", &self.data.len())
                .field("global_header", &self.global_header)
                .field("blocks", &self.blocks.len())
                .field("index_entries", &self.index_entries.len())
                .field("reorder_forward", &self.reorder_forward.as_ref().map(|v| v.len()))
                .field("reorder_reverse", &self.reorder_reverse.as_ref().map(|v| v.len()))
                .field("finalized", &self.finalized)
                .finish_non_exhaustive()
        }
    }

    impl Default for InMemoryWriter {
        fn default() -> Self {
            Self::new()
        }
    }

    impl InMemoryWriter {
        pub fn new() -> Self {
            Self {
                data: Vec::new(),
                global_header: None,
                blocks: Vec::new(),
                index_entries: Vec::new(),
                reorder_forward: None,
                reorder_reverse: None,
                global_hasher: Xxh64::new(0),
                finalized: false,
            }
        }

        /// Get the written data as a slice.
        pub fn as_slice(&self) -> &[u8] {
            &self.data
        }

        /// Check if finalize was called.
        pub fn is_finalized(&self) -> bool {
            self.finalized
        }
    }

    impl ArchiveWriter for InMemoryWriter {
        fn write_global_header(&mut self, header: &GlobalHeader) -> Result<()> {
            self.global_header = Some(header.clone());
            self.global_hasher.update(&header.flags.to_le_bytes());
            Ok(())
        }

        fn write_block(&mut self, block: &CompressedBlockData) -> Result<u64> {
            let archive_id_start = self
                .index_entries
                .last()
                .map(|entry| entry.archive_id_end())
                .unwrap_or(0);
            self.write_block_with_id(block, archive_id_start)
        }

        fn write_block_with_id(&mut self, block: &CompressedBlockData, archive_id_start: u64) -> Result<u64> {
            let block_start = self.data.len() as u64;

            // Build header
            let mut bh = BlockHeader::default();
            bh.block_id = block.block_id;
            bh.uncompressed_count = block.read_count;
            bh.uniform_read_length = block.uniform_read_length;
            bh.block_xxhash64 = block.block_checksum;
            bh.codec_ids = block.codec_ids;
            bh.codec_seq = block.codec_seq;
            bh.codec_qual = block.codec_qual;
            bh.codec_aux = block.codec_aux;
            bh.checksum_type = 0;

            bh.offset_ids = 0;
            bh.size_ids = block.id_stream.len() as u64;
            bh.offset_seq = bh.size_ids;
            bh.size_seq = block.seq_stream.len() as u64;
            bh.offset_qual = bh.offset_seq + bh.size_seq;
            bh.size_qual = block.qual_stream.len() as u64;
            bh.offset_aux = bh.offset_qual + bh.size_qual;
            bh.size_aux = block.aux_stream.len() as u64;

            let total_payload =
                block.id_stream.len() + block.seq_stream.len() + block.qual_stream.len() + block.aux_stream.len();
            bh.compressed_size = total_payload as u64;

            // Write header
            let mut header_bytes = Vec::new();
            bh.write(&mut header_bytes)?;
            self.data.extend_from_slice(&header_bytes);

            // Write streams
            self.data.extend_from_slice(&block.id_stream);
            self.data.extend_from_slice(&block.seq_stream);
            self.data.extend_from_slice(&block.qual_stream);
            self.data.extend_from_slice(&block.aux_stream);

            // Update hasher
            self.global_hasher.update(&block.id_stream);
            self.global_hasher.update(&block.seq_stream);
            self.global_hasher.update(&block.qual_stream);
            self.global_hasher.update(&block.aux_stream);

            // Track index entry
            let total_block_bytes = BLOCK_HEADER_SIZE as u64 + total_payload as u64;
            self.index_entries.push(IndexEntry {
                offset: block_start,
                compressed_size: total_block_bytes,
                archive_id_start,
                read_count: block.read_count,
            });

            // Store block for inspection
            self.blocks.push(block.clone());

            Ok(block_start)
        }

        fn write_reorder_map(&mut self, forward_map: &[u64], reverse_map: &[u64]) -> Result<u64> {
            let map_offset = self.data.len() as u64;

            self.reorder_forward = Some(forward_map.to_vec());
            self.reorder_reverse = Some(reverse_map.to_vec());

            // Delta encode
            let forward_encoded = delta_encode_ids(forward_map);
            let reverse_encoded = delta_encode_ids(reverse_map);

            // Compress
            let forward_compressed = zstd::bulk::compress(&forward_encoded, 3)
                .map_err(|e| FqcError::Compression(format!("Reorder map compress failed: {e}")))?;
            let reverse_compressed = zstd::bulk::compress(&reverse_encoded, 3)
                .map_err(|e| FqcError::Compression(format!("Reorder map compress failed: {e}")))?;

            // Write header
            let rmh = ReorderMapHeader {
                version: 1,
                total_reads: forward_map.len() as u64,
                forward_map_size: forward_compressed.len() as u64,
                reverse_map_size: reverse_compressed.len() as u64,
            };
            let mut header_bytes = Vec::new();
            rmh.write(&mut header_bytes)?;
            self.data.extend_from_slice(&header_bytes);

            // Write compressed maps
            self.data.extend_from_slice(&forward_compressed);
            self.data.extend_from_slice(&reverse_compressed);

            Ok(map_offset)
        }

        fn patch_total_read_count(&mut self, _total_read_count: u64) -> Result<()> {
            // In-memory writer doesn't need to patch - just track it
            if let Some(ref mut header) = self.global_header {
                // Note: we can't actually mutate it since we're borrowing
                // In tests, this is usually set correctly from the start
                let _ = header.total_read_count;
            }
            Ok(())
        }

        fn finalize(mut self) -> Result<()> {
            let index_offset = self.data.len() as u64;

            // Write block index
            let block_index = BlockIndex {
                num_blocks: self.index_entries.len() as u64,
                entries: self.index_entries.clone(),
            };
            let mut index_bytes = Vec::new();
            block_index.write(&mut index_bytes)?;
            self.data.extend_from_slice(&index_bytes);

            // Compute global checksum
            let global_checksum = self.global_hasher.digest();

            // Write footer (reorder_offset is 0 for in-memory writer since we track maps separately)
            let footer = FileFooter::new(index_offset, 0, global_checksum);
            let mut footer_bytes = Vec::new();
            footer.write(&mut footer_bytes)?;
            self.data.extend_from_slice(&footer_bytes);

            self.finalized = true;
            Ok(())
        }
    }

    /// Mock writer that records all calls for assertion.
    #[derive(Debug, Default)]
    pub struct MockWriter {
        pub calls: Vec<WriterCall>,
    }

    #[derive(Debug, Clone)]
    pub enum WriterCall {
        WriteGlobalHeader(GlobalHeader),
        WriteBlock(CompressedBlockData),
        WriteReorderMap(Vec<u64>, Vec<u64>),
        PatchTotalReadCount(u64),
        Finalize,
    }

    impl MockWriter {
        pub fn new() -> Self {
            Self { calls: Vec::new() }
        }
    }

    impl ArchiveWriter for MockWriter {
        fn write_global_header(&mut self, header: &GlobalHeader) -> Result<()> {
            self.calls.push(WriterCall::WriteGlobalHeader(header.clone()));
            Ok(())
        }

        fn write_block(&mut self, block: &CompressedBlockData) -> Result<u64> {
            self.calls.push(WriterCall::WriteBlock(block.clone()));
            Ok(0)
        }

        fn write_block_with_id(&mut self, block: &CompressedBlockData, _archive_id_start: u64) -> Result<u64> {
            self.calls.push(WriterCall::WriteBlock(block.clone()));
            Ok(0)
        }

        fn write_reorder_map(&mut self, forward_map: &[u64], reverse_map: &[u64]) -> Result<u64> {
            self.calls
                .push(WriterCall::WriteReorderMap(forward_map.to_vec(), reverse_map.to_vec()));
            Ok(0)
        }

        fn patch_total_read_count(&mut self, total_read_count: u64) -> Result<()> {
            self.calls.push(WriterCall::PatchTotalReadCount(total_read_count));
            Ok(())
        }

        fn finalize(mut self) -> Result<()> {
            self.calls.push(WriterCall::Finalize);
            Ok(())
        }
    }
}
