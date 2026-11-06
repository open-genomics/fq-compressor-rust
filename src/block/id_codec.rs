// =============================================================================
// fqc - Delta/varint id codec
// =============================================================================
//! Delta + zig-zag + varint encoding for read-id sequences.

use crate::error::{FqcError, Result};

fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
    out
}

pub fn delta_encode_ids(ids: &[u64]) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut prev = 0i64;
    for &id in ids {
        let delta = id as i64 - prev;
        prev = id as i64;
        let zigzag = ((delta as u64) << 1) ^ ((delta >> 63) as u64);
        buf.extend_from_slice(&encode_varint(zigzag));
    }
    buf
}

pub fn delta_decode_ids(data: &[u8], count: u64) -> Result<Vec<u64>> {
    let expected = usize::try_from(count)
        .map_err(|_| FqcError::Format("delta_decode_ids count exceeds platform limits".to_string()))?;
    let mut ids = Vec::with_capacity(expected);
    let mut i = 0usize;
    let mut prev = 0i64;

    for id_index in 0..expected {
        let mut zigzag = 0u64;
        let mut terminated = false;
        for byte_index in 0..10 {
            let byte = *data
                .get(i)
                .ok_or_else(|| FqcError::Format(format!("delta_decode_ids: truncated varint at id {id_index}")))?;
            i += 1;
            let payload = u64::from(byte & 0x7F);
            if byte_index == 9 && payload > 1 {
                return Err(FqcError::Format(format!(
                    "delta_decode_ids: varint overflows u64 at id {id_index}"
                )));
            }
            zigzag |= payload << (byte_index * 7);
            if (byte & 0x80) == 0 {
                terminated = true;
                break;
            }
        }
        if !terminated {
            return Err(FqcError::Format(format!(
                "delta_decode_ids: unterminated varint at id {id_index}"
            )));
        }
        let delta = ((zigzag >> 1) as i64) ^ (-((zigzag & 1) as i64));
        prev = prev
            .checked_add(delta)
            .ok_or_else(|| FqcError::Format(format!("delta_decode_ids: delta overflows at id {id_index}")))?;
        if prev < 0 {
            return Err(FqcError::Format(format!(
                "delta_decode_ids: decoded negative id at position {id_index}"
            )));
        }
        ids.push(prev as u64);
    }

    if i != data.len() {
        return Err(FqcError::Format(format!(
            "delta_decode_ids: trailing bytes after {count} ids"
        )));
    }

    Ok(ids)
}
