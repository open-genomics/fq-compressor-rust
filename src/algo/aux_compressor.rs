// =============================================================================
// Aux Compressor Implementation
// =============================================================================
//! Auxiliary data (read lengths) compression using delta encoding + varint + Zstd.

use crate::algo::compressor_traits::AuxCompressor;
use crate::error::{FqcError, Result};
use crate::types::{encode_codec, CodecFamily, ReadRecord};

// =============================================================================
// DeltaVarintAuxCompressor
// =============================================================================

/// Aux compressor using delta encoding + varint + Zstd.
pub struct DeltaVarintAuxCompressor {
    zstd_level: i32,
}

impl DeltaVarintAuxCompressor {
    pub fn new(zstd_level: i32) -> Self {
        Self { zstd_level }
    }
}

impl AuxCompressor for DeltaVarintAuxCompressor {
    fn compress(&self, reads: &[ReadRecord]) -> Result<(Vec<u8>, u32)> {
        if reads.is_empty() {
            return Ok((Vec::new(), 0));
        }

        let first_len = reads[0].sequence.len();
        let is_uniform = reads.iter().all(|r| r.sequence.len() == first_len);

        if is_uniform {
            return Ok((Vec::new(), first_len as u32));
        }

        let mut buf: Vec<u8> = Vec::with_capacity(reads.len() * 4);
        let mut prev_len = 0i32;

        for read in reads {
            let len = read.sequence.len() as i32;
            let delta = len - prev_len;
            prev_len = len;

            let mut zigzag = ((delta << 1) ^ (delta >> 31)) as u32;
            loop {
                let byte = (zigzag & 0x7F) as u8;
                zigzag >>= 7;
                if zigzag != 0 {
                    buf.push(byte | 0x80);
                } else {
                    buf.push(byte);
                    break;
                }
            }
        }

        let compressed = zstd::bulk::compress(&buf, self.zstd_level)
            .map_err(|e| FqcError::Compression(format!("Aux Zstd compress failed: {e}")))?;

        Ok((compressed, 0))
    }

    fn decompress(&self, data: &[u8], read_count: u32) -> Result<Vec<u32>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let buf = zstd::stream::decode_all(data)
            .map_err(|e| FqcError::Decompression(format!("Aux Zstd decompress failed: {e}")))?;

        let mut lengths = Vec::with_capacity(read_count as usize);
        let mut i = 0usize;
        let mut prev_len = 0i32;

        while i < buf.len() && lengths.len() < read_count as usize {
            let mut zigzag = 0u32;
            let mut shift = 0u32;

            for _ in 0..5 {
                if i >= buf.len() {
                    break;
                }
                let byte = buf[i];
                i += 1;
                zigzag |= ((byte & 0x7F) as u32) << shift;
                shift += 7;
                if (byte & 0x80) == 0 {
                    break;
                }
            }

            let delta = ((zigzag >> 1) as i32) ^ (-((zigzag & 1) as i32));
            let len = prev_len + delta;
            prev_len = len;
            lengths.push(len as u32);
        }

        Ok(lengths)
    }

    fn codec_id(&self) -> u8 {
        encode_codec(CodecFamily::DeltaVarint, 0)
    }
}
