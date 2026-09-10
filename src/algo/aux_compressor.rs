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
            let len = i32::try_from(read.sequence.len())
                .map_err(|_| FqcError::InvalidArgument("Read length exceeds auxiliary codec range".to_string()))?;
            let delta = len
                .checked_sub(prev_len)
                .ok_or_else(|| FqcError::InvalidArgument("Read-length delta overflows auxiliary codec".to_string()))?;
            prev_len = len;

            let mut zigzag = ((delta as u32) << 1) ^ ((delta >> 31) as u32);
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

        // Zigzag varints are at most 5 bytes each.
        let max_out = (read_count as usize).saturating_mul(5).saturating_add(64);
        let buf = crate::memory_budget::zstd_decompress_bounded(data, max_out, "aux lengths")?;

        let mut lengths = Vec::with_capacity(read_count as usize);
        let mut i = 0usize;
        let mut prev_len = 0i32;

        for read_index in 0..read_count as usize {
            let mut zigzag = 0u32;
            let mut shift = 0u32;
            let mut terminated = false;

            for byte_index in 0..5 {
                let byte = *buf
                    .get(i)
                    .ok_or_else(|| FqcError::Format(format!("Truncated aux varint at read {read_index}")))?;
                i += 1;
                let payload = u32::from(byte & 0x7F);
                if byte_index == 4 && payload > 0x0F {
                    return Err(FqcError::Format(format!(
                        "Aux varint overflows u32 at read {read_index}"
                    )));
                }
                zigzag |= payload << shift;
                shift += 7;
                if (byte & 0x80) == 0 {
                    terminated = true;
                    break;
                }
            }
            if !terminated {
                return Err(FqcError::Format(format!(
                    "Aux varint is unterminated at read {read_index}"
                )));
            }

            let delta = ((zigzag >> 1) as i32) ^ (-((zigzag & 1) as i32));
            let len = prev_len
                .checked_add(delta)
                .ok_or_else(|| FqcError::Format(format!("Aux length overflows at read {read_index}")))?;
            if len < 0 {
                return Err(FqcError::Format(format!("Aux length is negative at read {read_index}")));
            }
            prev_len = len;
            lengths.push(len as u32);
        }

        if i != buf.len() {
            return Err(FqcError::Format("Aux stream has trailing bytes".to_string()));
        }

        Ok(lengths)
    }

    fn codec_id(&self) -> u8 {
        encode_codec(CodecFamily::DeltaVarint, 0)
    }
}
