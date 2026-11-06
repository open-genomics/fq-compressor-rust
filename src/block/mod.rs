// =============================================================================
// fqc - Neutral block domain
// =============================================================================
//! Block metadata and encoded payload shared by the codec and archive layers.
//!
//! This module deliberately knows nothing about `.fqc` file layout or about
//! which codec implementation produced a stream. Both `algo` and `archive`
//! depend on it instead of depending on each other.

pub mod id_codec;

use crate::types::{BlockId, ReadRecord};

/// Metadata needed to decode one encoded block.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BlockMeta {
    pub block_id: BlockId,
    pub read_count: u32,
    pub uniform_read_length: u32,
    pub block_checksum: u64,
    pub codec_ids: u8,
    pub codec_seq: u8,
    pub codec_qual: u8,
    pub codec_aux: u8,
}

/// One encoded block: metadata plus the four compressed streams.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EncodedBlock {
    pub meta: BlockMeta,
    pub id_stream: Vec<u8>,
    pub seq_stream: Vec<u8>,
    pub qual_stream: Vec<u8>,
    pub aux_stream: Vec<u8>,
}

impl EncodedBlock {
    pub fn total_compressed_size(&self) -> usize {
        self.id_stream.len() + self.seq_stream.len() + self.qual_stream.len() + self.aux_stream.len()
    }
}

impl std::ops::Deref for EncodedBlock {
    type Target = BlockMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl std::ops::DerefMut for EncodedBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.meta
    }
}

/// One decoded block.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DecodedBlock {
    pub reads: Vec<ReadRecord>,
}
