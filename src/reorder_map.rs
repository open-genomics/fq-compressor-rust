// =============================================================================
// fqc-rust - Reorder Map Module
// =============================================================================
// Implements bidirectional mapping for read reordering support.
//
// - Forward Map: original_id -> archive_id (for querying original position)
// - Reverse Map: archive_id -> original_id (for original order output)
// - Delta + Varint compression encoding (~2 bytes/read per map)
// - Chunk concatenation with offset accumulation (divide-and-conquer mode)
// - Map validation and consistency checks
// =============================================================================

use crate::algo::block_compressor::{delta_decode_ids, delta_encode_ids, encode_varint};
use crate::error::{FqcError, Result};
use crate::types::ReadId;

// =============================================================================
// Constants
// =============================================================================

pub const REORDER_MAP_VERSION: u32 = 1;
pub const MAX_VARINT_BYTES: usize = 10;
pub const TARGET_BYTES_PER_READ: f64 = 4.0;

// =============================================================================
// ZigZag Varint Encoding/Decoding
// =============================================================================

/// Encode a signed 64-bit integer as zigzag varint
pub fn encode_signed_varint(value: i64) -> Vec<u8> {
    let zigzag = ((value << 1) ^ (value >> 63)) as u64;
    encode_varint(zigzag)
}

/// Decode a zigzag varint to signed 64-bit integer
pub fn decode_signed_varint(data: &[u8]) -> Option<(i64, usize)> {
    let mut zigzag: u64 = 0;
    let mut shift: u32 = 0;
    let mut consumed = 0;

    for &byte in data {
        zigzag |= ((byte & 0x7F) as u64) << shift;
        shift += 7;
        consumed += 1;
        if (byte & 0x80) == 0 {
            let value = ((zigzag >> 1) as i64) ^ (-((zigzag & 1) as i64));
            return Some((value, consumed));
        }
        if shift >= 64 {
            return None;
        }
    }
    None
}

// =============================================================================
// ReorderMapData
// =============================================================================

/// In-memory representation of the bidirectional reorder map
#[derive(Debug, Clone, Default)]
pub struct ReorderMapData {
    forward_map: Vec<ReadId>, // original_id -> archive_id
    reverse_map: Vec<ReadId>, // archive_id -> original_id
}

impl ReorderMapData {
    /// Construct from forward and reverse maps
    pub fn new(forward_map: Vec<ReadId>, reverse_map: Vec<ReadId>) -> Self {
        Self {
            forward_map,
            reverse_map,
        }
    }

    /// Build from a reverse map (archive_order), computing forward map automatically
    pub fn from_reverse_map(reverse_map: Vec<ReadId>) -> Self {
        let n = reverse_map.len();
        let mut forward_map = vec![0u64; n];
        for (archive_id, &orig_id) in reverse_map.iter().enumerate() {
            if (orig_id as usize) < n {
                forward_map[orig_id as usize] = archive_id as ReadId;
            }
        }
        Self {
            forward_map,
            reverse_map,
        }
    }

    /// Build an identity map (no reordering)
    pub fn identity(total_reads: usize) -> Self {
        let map: Vec<ReadId> = (0..total_reads as u64).collect();
        Self {
            forward_map: map.clone(),
            reverse_map: map,
        }
    }

    // =========================================================================
    // Query Operations
    // =========================================================================

    /// Get archive ID for an original read ID
    pub fn get_archive_id(&self, original_id: ReadId) -> ReadId {
        self.forward_map
            .get(original_id as usize)
            .copied()
            .unwrap_or(original_id)
    }

    /// Get original ID for an archive read ID
    pub fn get_original_id(&self, archive_id: ReadId) -> ReadId {
        self.reverse_map.get(archive_id as usize).copied().unwrap_or(archive_id)
    }

    pub fn total_reads(&self) -> u64 {
        self.forward_map.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.forward_map.is_empty()
    }

    /// Check if maps are consistent (same size, valid inverse relationship)
    pub fn is_valid(&self) -> bool {
        if self.forward_map.len() != self.reverse_map.len() {
            return false;
        }
        let n = self.forward_map.len();
        for i in 0..n {
            let archive_id = self.forward_map[i] as usize;
            if archive_id >= n {
                return false;
            }
            if self.reverse_map[archive_id] != i as u64 {
                return false;
            }
        }
        true
    }

    // =========================================================================
    // Raw Access
    // =========================================================================

    pub fn forward_map(&self) -> &[ReadId] {
        &self.forward_map
    }

    pub fn reverse_map(&self) -> &[ReadId] {
        &self.reverse_map
    }

    // =========================================================================
    // Serialization
    // =========================================================================

    /// Serialize the reorder map to compressed bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let forward_encoded = delta_encode_ids(&self.forward_map);
        let reverse_encoded = delta_encode_ids(&self.reverse_map);

        let forward_compressed = zstd::bulk::compress(&forward_encoded, 3)
            .map_err(|e| FqcError::Compression(format!("Reorder map forward compress: {e}")))?;
        let reverse_compressed = zstd::bulk::compress(&reverse_encoded, 3)
            .map_err(|e| FqcError::Compression(format!("Reorder map reverse compress: {e}")))?;

        use byteorder::{LittleEndian, WriteBytesExt};
        let mut buf = Vec::new();
        buf.write_u32::<LittleEndian>(REORDER_MAP_VERSION)?;
        buf.write_u64::<LittleEndian>(self.forward_map.len() as u64)?;
        buf.write_u64::<LittleEndian>(forward_compressed.len() as u64)?;
        buf.write_u64::<LittleEndian>(reverse_compressed.len() as u64)?;
        buf.extend_from_slice(&forward_compressed);
        buf.extend_from_slice(&reverse_compressed);

        Ok(buf)
    }

    /// Deserialize a reorder map from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        use std::io::Cursor;

        let mut cur = Cursor::new(data);
        let version = cur.read_u32::<LittleEndian>()?;
        if version != REORDER_MAP_VERSION {
            return Err(FqcError::Format(format!("Unsupported reorder map version: {version}")));
        }

        let total_reads = cur.read_u64::<LittleEndian>()?;
        let fwd_size = cur.read_u64::<LittleEndian>()? as usize;
        let rev_size = cur.read_u64::<LittleEndian>()? as usize;

        let pos = cur.position() as usize;
        if pos + fwd_size + rev_size > data.len() {
            return Err(FqcError::Format("Truncated reorder map data".to_string()));
        }

        let fwd_compressed = &data[pos..pos + fwd_size];
        let rev_compressed = &data[pos + fwd_size..pos + fwd_size + rev_size];

        let fwd_raw = zstd::stream::decode_all(fwd_compressed)
            .map_err(|e| FqcError::Decompression(format!("Reorder map forward decompress: {e}")))?;
        let rev_raw = zstd::stream::decode_all(rev_compressed)
            .map_err(|e| FqcError::Decompression(format!("Reorder map reverse decompress: {e}")))?;

        let forward_map = delta_decode_ids(&fwd_raw, total_reads)?;
        let reverse_map = delta_decode_ids(&rev_raw, total_reads)?;

        Ok(Self {
            forward_map,
            reverse_map,
        })
    }

    /// Estimate serialized size in bytes
    pub fn estimate_serialized_size(&self) -> usize {
        let n = self.forward_map.len();
        (n as f64 * TARGET_BYTES_PER_READ) as usize + 28 // header overhead
    }

    // =========================================================================
    // Chunk Concatenation (divide-and-conquer mode)
    // =========================================================================

    /// Append another chunk's reorder map with offset adjustment
    pub fn append_chunk(&mut self, other: &ReorderMapData, archive_id_offset: ReadId, original_id_offset: ReadId) {
        for &orig_id in &other.forward_map {
            self.forward_map.push(orig_id + archive_id_offset);
        }
        for &archive_id in &other.reverse_map {
            self.reverse_map.push(archive_id + original_id_offset);
        }
    }

    /// Combine multiple chunk reorder maps into a single global map
    pub fn combine_chunks(chunks: &[ReorderMapData], chunk_sizes: &[u64]) -> Self {
        let total: u64 = chunk_sizes.iter().sum();
        let mut combined = ReorderMapData {
            forward_map: Vec::with_capacity(total as usize),
            reverse_map: Vec::with_capacity(total as usize),
        };

        let mut archive_offset: u64 = 0;
        let mut original_offset: u64 = 0;

        for (i, chunk) in chunks.iter().enumerate() {
            combined.append_chunk(chunk, archive_offset, original_offset);
            if i < chunk_sizes.len() {
                archive_offset += chunk_sizes[i];
                original_offset += chunk_sizes[i];
            }
        }

        combined
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Compression statistics
    pub fn compression_stats(&self) -> Result<CompressionStats> {
        let serialized = self.serialize()?;
        let total_reads = self.forward_map.len() as u64;
        let uncompressed_size = total_reads as usize * 16; // 8 bytes per ID * 2 maps
        let compressed_size = serialized.len();

        Ok(CompressionStats {
            total_reads,
            total_compressed_size: compressed_size,
            bytes_per_read: if total_reads > 0 {
                compressed_size as f64 / total_reads as f64
            } else {
                0.0
            },
            compression_ratio: if compressed_size > 0 {
                uncompressed_size as f64 / compressed_size as f64
            } else {
                0.0
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub total_reads: u64,
    pub total_compressed_size: usize,
    pub bytes_per_read: f64,
    pub compression_ratio: f64,
}

// =============================================================================
// Validation Functions
// =============================================================================

/// Verify that forward and reverse maps are consistent inverses
pub fn verify_map_consistency(forward_map: &[ReadId], reverse_map: &[ReadId]) -> Result<()> {
    if forward_map.len() != reverse_map.len() {
        return Err(FqcError::Format(format!(
            "Map size mismatch: forward={}, reverse={}",
            forward_map.len(),
            reverse_map.len()
        )));
    }

    let n = forward_map.len();
    for (i, &fwd) in forward_map.iter().enumerate().take(n) {
        let archive_id = fwd as usize;
        if archive_id >= n {
            return Err(FqcError::Format(format!(
                "Forward map[{}]={} out of range (n={})",
                i, archive_id, n
            )));
        }
        if reverse_map[archive_id] != i as u64 {
            return Err(FqcError::Format(format!(
                "Inconsistent maps: forward[{}]={}, reverse[{}]={} (expected {})",
                i, archive_id, archive_id, reverse_map[archive_id], i
            )));
        }
    }

    Ok(())
}

/// Validate that a reorder map is a valid permutation
pub fn validate_permutation(map: &[ReadId]) -> Result<()> {
    let n = map.len();
    let mut seen = vec![false; n];
    for (i, &id) in map.iter().enumerate() {
        let id = id as usize;
        if id >= n {
            return Err(FqcError::Format(format!("Map[{}]={} out of range (n={})", i, id, n)));
        }
        if seen[id] {
            return Err(FqcError::Format(format!("Duplicate value {} in map", id)));
        }
        seen[id] = true;
    }
    Ok(())
}
