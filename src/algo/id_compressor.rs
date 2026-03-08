// =============================================================================
// fqc-rust - ID Compressor (Tokenize + Delta + Zstd)
// =============================================================================

use crate::error::{FqcError, Result};

// =============================================================================
// Constants
// =============================================================================

const MAGIC_EXACT: u8 = 0x01;
const MAGIC_TOKENIZE: u8 = 0x02;
const MAGIC_DISCARD: u8 = 0x03;
const DELIMITERS: &[u8] = b":_/| \t";
const MIN_PATTERN_MATCH_RATIO: f64 = 0.95;

// =============================================================================
// Token Types
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenType {
    Static = 0,
    DynamicInt = 1,
    DynamicString = 2,
    Delimiter = 3,
}

impl TokenType {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => TokenType::Static,
            1 => TokenType::DynamicInt,
            2 => TokenType::DynamicString,
            3 => TokenType::Delimiter,
            _ => TokenType::DynamicString,
        }
    }
}

#[derive(Debug, Clone)]
struct Token {
    ttype: TokenType,
    value: String,
    int_value: i64,
}

// =============================================================================
// IDPattern
// =============================================================================

#[derive(Debug, Clone, Default)]
struct IDPattern {
    token_types: Vec<TokenType>,
    static_values: Vec<String>,
    delimiters: Vec<u8>,
    num_dynamic_ints: usize,
    num_dynamic_strings: usize,
}

// =============================================================================
// Tokenizer
// =============================================================================

fn is_delimiter(c: u8) -> bool {
    DELIMITERS.contains(&c)
}

fn try_parse_int(s: &str) -> Option<i64> {
    s.parse::<i64>().ok()
}

fn tokenize(id: &str) -> Vec<Token> {
    let bytes = id.as_bytes();
    let mut tokens = Vec::with_capacity(16);
    let mut pos = 0;
    let mut token_start = 0;

    while pos < bytes.len() {
        if is_delimiter(bytes[pos]) {
            if pos > token_start {
                let s = &id[token_start..pos];
                if let Some(iv) = try_parse_int(s) {
                    // Treat leading-zero integers as strings to preserve exact formatting
                    if s.len() > 1 && s.starts_with('0') {
                        tokens.push(Token {
                            ttype: TokenType::DynamicString,
                            value: s.to_string(),
                            int_value: 0,
                        });
                    } else {
                        tokens.push(Token {
                            ttype: TokenType::DynamicInt,
                            value: s.to_string(),
                            int_value: iv,
                        });
                    }
                } else {
                    tokens.push(Token {
                        ttype: TokenType::DynamicString,
                        value: s.to_string(),
                        int_value: 0,
                    });
                }
            }
            tokens.push(Token {
                ttype: TokenType::Delimiter,
                value: String::from(bytes[pos] as char),
                int_value: 0,
            });
            pos += 1;
            token_start = pos;
        } else {
            pos += 1;
        }
    }

    if pos > token_start {
        let s = &id[token_start..pos];
        if let Some(iv) = try_parse_int(s) {
            // Treat leading-zero integers as strings to preserve exact formatting
            if s.len() > 1 && s.starts_with('0') {
                tokens.push(Token {
                    ttype: TokenType::DynamicString,
                    value: s.to_string(),
                    int_value: 0,
                });
            } else {
                tokens.push(Token {
                    ttype: TokenType::DynamicInt,
                    value: s.to_string(),
                    int_value: iv,
                });
            }
        } else {
            tokens.push(Token {
                ttype: TokenType::DynamicString,
                value: s.to_string(),
                int_value: 0,
            });
        }
    }

    tokens
}

// =============================================================================
// Pattern Detection
// =============================================================================

fn detect_pattern(ids: &[&str]) -> Option<IDPattern> {
    if ids.is_empty() {
        return None;
    }

    let first_tokens = tokenize(ids[0]);
    if first_tokens.is_empty() {
        return None;
    }

    let mut pattern = IDPattern::default();
    for t in &first_tokens {
        pattern.token_types.push(t.ttype);
        match t.ttype {
            TokenType::Static => pattern.static_values.push(t.value.clone()),
            TokenType::Delimiter => pattern.delimiters.push(t.value.as_bytes()[0]),
            TokenType::DynamicInt => pattern.num_dynamic_ints += 1,
            TokenType::DynamicString => pattern.num_dynamic_strings += 1,
        }
    }

    // Check how many IDs match
    let mut match_count = 0usize;
    for id in ids {
        let tokens = tokenize(id);
        if tokens.len() != pattern.token_types.len() {
            continue;
        }

        let mut ok = true;
        let mut si = 0;
        let mut di = 0;
        for (i, t) in tokens.iter().enumerate() {
            let expected = pattern.token_types[i];
            if t.ttype != expected {
                // Allow int/string flexibility
                if !((expected == TokenType::DynamicInt && t.ttype == TokenType::DynamicString)
                    || (expected == TokenType::DynamicString && t.ttype == TokenType::DynamicInt))
                {
                    ok = false;
                    break;
                }
            }
            if expected == TokenType::Static {
                if si >= pattern.static_values.len() || t.value != pattern.static_values[si] {
                    ok = false;
                    break;
                }
                si += 1;
            }
            if expected == TokenType::Delimiter {
                if di >= pattern.delimiters.len() || t.value.as_bytes().first() != Some(&pattern.delimiters[di]) {
                    ok = false;
                    break;
                }
                di += 1;
            }
        }
        if ok {
            match_count += 1;
        }
    }

    let ratio = match_count as f64 / ids.len() as f64;
    if ratio < MIN_PATTERN_MATCH_RATIO {
        return None;
    }
    Some(pattern)
}

// =============================================================================
// Varint Encoding
// =============================================================================

fn zigzag_encode(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

fn zigzag_decode(v: u64) -> i64 {
    ((v >> 1) as i64) ^ (-((v & 1) as i64))
}

fn uvarint_encode(mut v: u64, out: &mut Vec<u8>) {
    while v >= 0x80 {
        out.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn uvarint_decode(data: &[u8], pos: &mut usize) -> u64 {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    for _ in 0..10 {
        if *pos >= data.len() {
            break;
        }
        let b = data[*pos];
        *pos += 1;
        result |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            return result;
        }
        shift += 7;
    }
    result
}

fn varint_encode(v: i64, out: &mut Vec<u8>) {
    uvarint_encode(zigzag_encode(v), out);
}

fn varint_decode(data: &[u8], pos: &mut usize) -> i64 {
    zigzag_decode(uvarint_decode(data, pos))
}

fn delta_varint_encode(values: &[i64]) -> Vec<u8> {
    let mut result = Vec::with_capacity(values.len() * 2);
    let mut prev: i64 = 0;
    for &v in values {
        varint_encode(v - prev, &mut result);
        prev = v;
    }
    result
}

fn delta_varint_decode(data: &[u8], count: usize) -> Vec<i64> {
    let mut result = Vec::with_capacity(count);
    let mut pos = 0;
    let mut prev: i64 = 0;
    for _ in 0..count {
        let delta = varint_decode(data, &mut pos);
        let v = prev + delta;
        result.push(v);
        prev = v;
    }
    result
}

// =============================================================================
// Compress IDs (Exact mode)
// =============================================================================

fn compress_exact(ids: &[&str], zstd_level: i32) -> Result<Vec<u8>> {
    let mut uncompressed = Vec::with_capacity(ids.len() * 50);
    for id in ids {
        let bytes = id.as_bytes();
        uvarint_encode(bytes.len() as u64, &mut uncompressed);
        uncompressed.extend_from_slice(bytes);
    }

    let compressed = zstd::bulk::compress(&uncompressed, zstd_level)
        .map_err(|e| FqcError::Compression(format!("ID Zstd compress failed: {e}")))?;

    let mut out = Vec::with_capacity(1 + 10 + compressed.len());
    out.push(MAGIC_EXACT);
    uvarint_encode(uncompressed.len() as u64, &mut out);
    out.extend_from_slice(&compressed);
    Ok(out)
}

fn decompress_exact(data: &[u8], num_ids: u32) -> Result<Vec<String>> {
    if num_ids == 0 {
        return Ok(Vec::new());
    }
    let mut pos = 0;
    let uncompressed_size = uvarint_decode(data, &mut pos) as usize;
    let _ = uncompressed_size;

    let uncompressed = zstd::stream::decode_all(&data[pos..])
        .map_err(|e| FqcError::Decompression(format!("ID Zstd decompress failed: {e}")))?;

    let mut ids = Vec::with_capacity(num_ids as usize);
    let mut offset = 0;
    for _ in 0..num_ids {
        let len = uvarint_decode(&uncompressed, &mut offset) as usize;
        if offset + len > uncompressed.len() {
            return Err(FqcError::Format("Truncated ID data".to_string()));
        }
        ids.push(String::from_utf8_lossy(&uncompressed[offset..offset + len]).into_owned());
        offset += len;
    }
    Ok(ids)
}

// =============================================================================
// Compress IDs (Tokenize mode)
// =============================================================================

fn compress_tokenize(ids: &[&str], pattern: &IDPattern, zstd_level: i32) -> Result<Vec<u8>> {
    let num_ids = ids.len();

    // Extract dynamic columns
    let mut int_columns: Vec<Vec<i64>> = vec![Vec::with_capacity(num_ids); pattern.num_dynamic_ints];
    let mut str_columns: Vec<Vec<String>> = vec![Vec::with_capacity(num_ids); pattern.num_dynamic_strings];

    for id in ids {
        let tokens = tokenize(id);
        let mut int_idx = 0;
        let mut str_idx = 0;
        for (i, ttype) in pattern.token_types.iter().enumerate() {
            if i >= tokens.len() {
                break;
            }
            match ttype {
                TokenType::DynamicInt => {
                    let iv = if tokens[i].ttype == TokenType::DynamicInt {
                        tokens[i].int_value
                    } else {
                        try_parse_int(&tokens[i].value).unwrap_or(0)
                    };
                    if int_idx < int_columns.len() {
                        int_columns[int_idx].push(iv);
                    }
                    int_idx += 1;
                }
                TokenType::DynamicString => {
                    if str_idx < str_columns.len() {
                        str_columns[str_idx].push(tokens[i].value.clone());
                    }
                    str_idx += 1;
                }
                _ => {}
            }
        }
        // Pad missing columns
        while int_idx < int_columns.len() {
            int_columns[int_idx].push(0);
            int_idx += 1;
        }
        while str_idx < str_columns.len() {
            str_columns[str_idx].push(String::new());
            str_idx += 1;
        }
    }

    // Build uncompressed buffer
    let mut uncompressed = Vec::new();

    // Pattern header: [num_types][types...][num_static][static_values...][num_delims][delims...]
    uvarint_encode(pattern.token_types.len() as u64, &mut uncompressed);
    for tt in &pattern.token_types {
        uncompressed.push(*tt as u8);
    }

    uvarint_encode(pattern.static_values.len() as u64, &mut uncompressed);
    for sv in &pattern.static_values {
        uvarint_encode(sv.len() as u64, &mut uncompressed);
        uncompressed.extend_from_slice(sv.as_bytes());
    }

    uvarint_encode(pattern.delimiters.len() as u64, &mut uncompressed);
    uncompressed.extend_from_slice(&pattern.delimiters);

    // Integer columns (delta-varint encoded)
    uvarint_encode(int_columns.len() as u64, &mut uncompressed);
    for col in &int_columns {
        let encoded = delta_varint_encode(col);
        uvarint_encode(encoded.len() as u64, &mut uncompressed);
        uncompressed.extend_from_slice(&encoded);
    }

    // String columns (length-prefixed)
    uvarint_encode(str_columns.len() as u64, &mut uncompressed);
    for col in &str_columns {
        for s in col {
            uvarint_encode(s.len() as u64, &mut uncompressed);
            uncompressed.extend_from_slice(s.as_bytes());
        }
    }

    let compressed = zstd::bulk::compress(&uncompressed, zstd_level)
        .map_err(|e| FqcError::Compression(format!("ID tokenize Zstd compress failed: {e}")))?;

    let mut out = Vec::with_capacity(1 + 10 + compressed.len());
    out.push(MAGIC_TOKENIZE);
    uvarint_encode(uncompressed.len() as u64, &mut out);
    out.extend_from_slice(&compressed);
    Ok(out)
}

fn decompress_tokenize(data: &[u8], num_ids: u32) -> Result<Vec<String>> {
    if num_ids == 0 {
        return Ok(Vec::new());
    }

    let mut pos = 0;
    let _uncompressed_size = uvarint_decode(data, &mut pos);

    let uncompressed = zstd::stream::decode_all(&data[pos..])
        .map_err(|e| FqcError::Decompression(format!("ID tokenize Zstd decompress failed: {e}")))?;

    let mut offset = 0;

    // Read pattern
    let num_types = uvarint_decode(&uncompressed, &mut offset) as usize;
    let mut token_types = Vec::with_capacity(num_types);
    for _ in 0..num_types {
        if offset >= uncompressed.len() {
            break;
        }
        let tt = TokenType::from_u8(uncompressed[offset]);
        offset += 1;
        token_types.push(tt);
    }

    // Static values
    let num_static = uvarint_decode(&uncompressed, &mut offset) as usize;
    let mut static_values = Vec::with_capacity(num_static);
    for _ in 0..num_static {
        let len = uvarint_decode(&uncompressed, &mut offset) as usize;
        if offset + len > uncompressed.len() {
            return Err(FqcError::Format("Truncated tokenize static data".to_string()));
        }
        static_values.push(String::from_utf8_lossy(&uncompressed[offset..offset + len]).into_owned());
        offset += len;
    }

    // Delimiters
    let num_delims = uvarint_decode(&uncompressed, &mut offset) as usize;
    let mut delimiters = Vec::with_capacity(num_delims);
    for _ in 0..num_delims {
        if offset >= uncompressed.len() {
            break;
        }
        delimiters.push(uncompressed[offset]);
        offset += 1;
    }

    // Integer columns
    let num_int_cols = uvarint_decode(&uncompressed, &mut offset) as usize;
    let mut int_columns = Vec::with_capacity(num_int_cols);
    for _ in 0..num_int_cols {
        let encoded_len = uvarint_decode(&uncompressed, &mut offset) as usize;
        if offset + encoded_len > uncompressed.len() {
            return Err(FqcError::Format("Truncated tokenize int column".to_string()));
        }
        let col = delta_varint_decode(&uncompressed[offset..offset + encoded_len], num_ids as usize);
        offset += encoded_len;
        int_columns.push(col);
    }

    // String columns
    let num_str_cols = uvarint_decode(&uncompressed, &mut offset) as usize;
    let mut str_columns: Vec<Vec<String>> = Vec::with_capacity(num_str_cols);
    for _ in 0..num_str_cols {
        let mut col = Vec::with_capacity(num_ids as usize);
        for _ in 0..num_ids {
            let len = uvarint_decode(&uncompressed, &mut offset) as usize;
            if offset + len > uncompressed.len() {
                return Err(FqcError::Format("Truncated tokenize string column".to_string()));
            }
            col.push(String::from_utf8_lossy(&uncompressed[offset..offset + len]).into_owned());
            offset += len;
        }
        str_columns.push(col);
    }

    // Reconstruct IDs
    let mut ids = Vec::with_capacity(num_ids as usize);
    for i in 0..num_ids as usize {
        let mut id = String::new();
        let mut si = 0;
        let mut di = 0;
        let mut ii = 0;
        let mut sti = 0;
        for tt in &token_types {
            match tt {
                TokenType::Static => {
                    if si < static_values.len() {
                        id.push_str(&static_values[si]);
                    }
                    si += 1;
                }
                TokenType::Delimiter => {
                    if di < delimiters.len() {
                        id.push(delimiters[di] as char);
                    }
                    di += 1;
                }
                TokenType::DynamicInt => {
                    if ii < int_columns.len() && i < int_columns[ii].len() {
                        id.push_str(&int_columns[ii][i].to_string());
                    }
                    ii += 1;
                }
                TokenType::DynamicString => {
                    if sti < str_columns.len() && i < str_columns[sti].len() {
                        id.push_str(&str_columns[sti][i]);
                    }
                    sti += 1;
                }
            }
        }
        ids.push(id);
    }

    Ok(ids)
}

// =============================================================================
// Public API
// =============================================================================

/// Compress a block of read IDs.
/// Tries tokenize mode first; falls back to exact if pattern detection fails.
pub fn compress_ids(ids: &[&str], zstd_level: i32, discard: bool) -> Result<Vec<u8>> {
    if discard {
        return Ok(vec![MAGIC_DISCARD]);
    }
    if ids.is_empty() {
        return compress_exact(ids, zstd_level);
    }

    // Try tokenize mode
    if let Some(pattern) = detect_pattern(ids) {
        if pattern.num_dynamic_ints > 0 {
            return compress_tokenize(ids, &pattern, zstd_level);
        }
    }

    // Fallback to exact
    compress_exact(ids, zstd_level)
}

/// Decompress a block of read IDs.
/// `id_prefix` is used for discard mode to generate placeholder IDs.
pub fn decompress_ids(data: &[u8], num_ids: u32, id_prefix: &str) -> Result<Vec<String>> {
    if data.is_empty() {
        return Ok(vec![String::new(); num_ids as usize]);
    }

    let magic = data[0];
    let payload = &data[1..];

    match magic {
        MAGIC_EXACT => decompress_exact(payload, num_ids),
        MAGIC_TOKENIZE => decompress_tokenize(payload, num_ids),
        MAGIC_DISCARD => Ok((1..=num_ids as u64).map(|i| format!("{}{}", id_prefix, i)).collect()),
        _ => {
            // Legacy format: len-prefixed Zstd (no magic byte)
            // Fall back to the old decompression path
            decompress_legacy(data, num_ids)
        }
    }
}

/// Legacy decompression for archives written before the magic-byte format.
fn decompress_legacy(data: &[u8], num_ids: u32) -> Result<Vec<String>> {
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::{Cursor, Read};

    let buf = zstd::stream::decode_all(data)
        .map_err(|e| FqcError::Decompression(format!("ID Zstd decompress (legacy) failed: {e}")))?;

    let mut ids = Vec::with_capacity(num_ids as usize);
    let mut cur = Cursor::new(&buf);

    for _ in 0..num_ids {
        let len = cur
            .read_u16::<LittleEndian>()
            .map_err(|e| FqcError::Format(format!("Truncated ID data: {e}")))?;
        let mut id = vec![0u8; len as usize];
        cur.read_exact(&mut id)
            .map_err(|e| FqcError::Format(format!("Truncated ID bytes: {e}")))?;
        ids.push(String::from_utf8_lossy(&id).into_owned());
    }

    Ok(ids)
}
