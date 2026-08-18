// =============================================================================
// ID Compressor Implementations
// =============================================================================
//! ID compression using delta encoding + Zstd.

use crate::algo::compressor_traits::IdCompressor;
use crate::error::Result;
use crate::types::{encode_codec, CodecFamily, IdMode, ReadRecord};

// =============================================================================
// DeltaZstdIdCompressor
// =============================================================================

/// ID compressor using delta encoding + Zstd.
pub struct DeltaZstdIdCompressor {
    zstd_level: i32,
    id_mode: IdMode,
    id_prefix: String,
}

impl DeltaZstdIdCompressor {
    pub fn new(zstd_level: i32, id_mode: IdMode, id_prefix: String) -> Self {
        Self {
            zstd_level,
            id_mode,
            id_prefix,
        }
    }
}

impl IdCompressor for DeltaZstdIdCompressor {
    fn compress(&self, reads: &[ReadRecord]) -> Result<Vec<u8>> {
        // Use the full header for compression
        let full_headers: Vec<String> = reads
            .iter()
            .map(|r| {
                if r.comment.is_empty() {
                    r.id.clone()
                } else {
                    format!("{} {}", r.id, r.comment)
                }
            })
            .collect();
        let header_refs: Vec<&str> = full_headers.iter().map(|s| s.as_str()).collect();

        crate::algo::id_compressor::compress_ids(&header_refs, self.zstd_level, self.id_mode)
    }

    fn decompress(&self, data: &[u8], read_count: u32) -> Result<Vec<String>> {
        crate::algo::id_compressor::decompress_ids(data, read_count, &self.id_prefix)
    }

    fn codec_id(&self) -> u8 {
        if self.id_mode == IdMode::Discard {
            encode_codec(CodecFamily::Raw, 0)
        } else {
            encode_codec(CodecFamily::DeltaZstd, 0)
        }
    }
}
