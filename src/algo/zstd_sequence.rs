// =============================================================================
// Zstd Sequence Compressor
// =============================================================================
//! Zstd-based compression for medium and long read sequences.

use crate::algo::compressor_traits::SequenceCompressor;
use crate::error::{FqcError, Result};
use crate::types::{encode_codec, CodecFamily, ReadRecord};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read};

// =============================================================================
// ZstdSequenceCompressor
// =============================================================================

/// Zstd-based sequence compressor for medium and long reads.
pub struct ZstdSequenceCompressor {
    level: i32,
}

impl ZstdSequenceCompressor {
    pub fn new(level: i32) -> Self {
        Self { level }
    }
}

impl SequenceCompressor for ZstdSequenceCompressor {
    fn compress(&self, reads: &[ReadRecord]) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::with_capacity(reads.len() * 200);

        for read in reads {
            buf.write_u32::<LittleEndian>(read.sequence.len() as u32)?;
            buf.extend_from_slice(read.sequence.as_bytes());
        }

        zstd::bulk::compress(&buf, self.level)
            .map_err(|e| FqcError::Compression(format!("Zstd sequence compress failed: {e}")))
    }

    fn decompress(&self, data: &[u8], read_count: u32, _uniform_length: u32, _lengths: &[u32]) -> Result<Vec<String>> {
        if data.is_empty() {
            return Ok(vec![String::new(); read_count as usize]);
        }

        let buf = zstd::stream::decode_all(data)
            .map_err(|e| FqcError::Decompression(format!("Zstd sequence decompress failed: {e}")))?;

        let mut sequences = Vec::with_capacity(read_count as usize);
        let mut cur = Cursor::new(&buf);

        for _ in 0..read_count {
            let len = cur
                .read_u32::<LittleEndian>()
                .map_err(|e| FqcError::Format(format!("Truncated sequence data: {e}")))?;
            let mut seq = vec![0u8; len as usize];
            cur.read_exact(&mut seq)
                .map_err(|e| FqcError::Format(format!("Truncated sequence bytes: {e}")))?;
            sequences.push(String::from_utf8_lossy(&seq).into_owned());
        }

        Ok(sequences)
    }

    fn codec_id(&self) -> u8 {
        encode_codec(CodecFamily::ZstdPlain, 0)
    }
}
