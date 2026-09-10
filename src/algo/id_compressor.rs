// =============================================================================
// fqc-rust - ID Compressor (Tokenize + Delta + Zstd)
// =============================================================================

use crate::error::{FqcError, Result};
use crate::types::IdMode;

// =============================================================================
// Constants
// =============================================================================

pub const ID_MAGIC_EXACT: u8 = 0x01;
pub const ID_MAGIC_TOKENIZE: u8 = 0x02;
pub const ID_MAGIC_DISCARD: u8 = 0x03;
const MAGIC_EXACT: u8 = ID_MAGIC_EXACT;
const MAGIC_TOKENIZE: u8 = ID_MAGIC_TOKENIZE;
const MAGIC_DISCARD: u8 = ID_MAGIC_DISCARD;
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
    fn try_from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(TokenType::Static),
            1 => Some(TokenType::DynamicInt),
            2 => Some(TokenType::DynamicString),
            3 => Some(TokenType::Delimiter),
            _ => None,
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
    ((v as u64) << 1) ^ ((v >> 63) as u64)
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

fn uvarint_decode(data: &[u8], pos: &mut usize) -> Result<u64> {
    let mut result = 0u64;
    for byte_index in 0..10 {
        let byte = *data
            .get(*pos)
            .ok_or_else(|| FqcError::Format("truncated ID varint".to_string()))?;
        *pos += 1;
        let payload = u64::from(byte & 0x7f);
        if byte_index == 9 && payload > 1 {
            return Err(FqcError::Format("ID varint overflows u64".to_string()));
        }
        result |= payload << (byte_index * 7);
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(FqcError::Format("unterminated ID varint".to_string()))
}

fn varint_encode(v: i64, out: &mut Vec<u8>) {
    uvarint_encode(zigzag_encode(v), out);
}

fn varint_decode(data: &[u8], pos: &mut usize) -> Result<i64> {
    Ok(zigzag_decode(uvarint_decode(data, pos)?))
}

fn delta_varint_encode(values: &[i64]) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(values.len() * 2);
    let mut prev: i64 = 0;
    for &v in values {
        let delta = v
            .checked_sub(prev)
            .ok_or_else(|| FqcError::InvalidArgument("ID integer delta overflows i64".to_string()))?;
        varint_encode(delta, &mut result);
        prev = v;
    }
    Ok(result)
}

fn delta_varint_decode(data: &[u8], count: usize) -> Result<Vec<i64>> {
    let mut result = Vec::with_capacity(count);
    let mut pos = 0;
    let mut prev: i64 = 0;
    for _ in 0..count {
        let delta = varint_decode(data, &mut pos)?;
        let v = prev
            .checked_add(delta)
            .ok_or_else(|| FqcError::Format("ID delta overflows i64".to_string()))?;
        result.push(v);
        prev = v;
    }
    if pos != data.len() {
        return Err(FqcError::Format("ID delta stream has trailing bytes".to_string()));
    }
    Ok(result)
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
    let mut pos = 0;
    let declared_size = uvarint_decode(data, &mut pos)?;
    let uncompressed_size = usize::try_from(declared_size)
        .map_err(|_| FqcError::Format("ID exact uncompressed size exceeds platform limits".to_string()))?;
    // Cap runaway declarations: at most 64 KiB per ID on average.
    let max_allowed = (num_ids as usize).saturating_mul(64 * 1024).saturating_add(1024);
    if uncompressed_size > max_allowed {
        return Err(FqcError::ResourceLimit {
            location: "id exact".to_string(),
            declared: declared_size,
            allowed: max_allowed as u64,
        });
    }
    let compressed = data
        .get(pos..)
        .ok_or_else(|| FqcError::Format("Missing ID exact compressed payload".to_string()))?;
    // The bounded helper intentionally rejects a zero ceiling.  A valid
    // zero-ID exact stream still has a zstd frame whose decoded payload is
    // empty, so give the decoder a one-byte probe ceiling and enforce the
    // declared size immediately afterwards.
    let uncompressed = crate::memory_budget::zstd_decompress_bounded(compressed, uncompressed_size.max(1), "id exact")?;
    if uncompressed.len() != uncompressed_size {
        return Err(FqcError::Format(format!(
            "ID exact payload size {} does not match declared {}",
            uncompressed.len(),
            uncompressed_size
        )));
    }

    let mut ids = Vec::with_capacity(num_ids as usize);
    let mut offset = 0;
    for _ in 0..num_ids {
        let len = usize::try_from(uvarint_decode(&uncompressed, &mut offset)?)
            .map_err(|_| FqcError::Format("ID length exceeds platform limits".to_string()))?;
        let end = offset
            .checked_add(len)
            .ok_or_else(|| FqcError::Format("ID length overflows buffer".to_string()))?;
        if end > uncompressed.len() {
            return Err(FqcError::Format("Truncated ID data".to_string()));
        }
        let id = String::from_utf8(uncompressed[offset..end].to_vec())
            .map_err(|e| FqcError::Format(format!("Invalid UTF-8 in ID data: {e}")))?;
        ids.push(id);
        offset = end;
    }
    if offset != uncompressed.len() {
        return Err(FqcError::Format("ID exact stream has trailing bytes".to_string()));
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

    // Losslessness gate: rebuild exactly as the decoder would and fall back to
    // the exact codec if any ID differs (off-pattern IDs, ambiguous parsing).
    let rebuilt = rebuild_ids_from_pattern(
        &pattern.token_types,
        &pattern.static_values,
        &pattern.delimiters,
        &int_columns,
        &str_columns,
        num_ids,
    );
    if rebuilt.iter().zip(ids.iter()).any(|(r, o)| r != o) {
        return compress_exact(ids, zstd_level);
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
        let encoded = delta_varint_encode(col)?;
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
    let mut pos = 0;
    let declared_size = uvarint_decode(data, &mut pos)?;
    let uncompressed_size = usize::try_from(declared_size)
        .map_err(|_| FqcError::Format("ID tokenize uncompressed size exceeds platform limits".to_string()))?;
    let max_allowed = (num_ids as usize).saturating_mul(64 * 1024).saturating_add(1024);
    if uncompressed_size > max_allowed {
        return Err(FqcError::ResourceLimit {
            location: "id tokenize".to_string(),
            declared: declared_size,
            allowed: max_allowed as u64,
        });
    }
    let compressed = data
        .get(pos..)
        .ok_or_else(|| FqcError::Format("Missing ID tokenize compressed payload".to_string()))?;
    let uncompressed =
        crate::memory_budget::zstd_decompress_bounded(compressed, uncompressed_size.max(1), "id tokenize")?;
    if uncompressed.len() != uncompressed_size {
        return Err(FqcError::Format(format!(
            "ID tokenize payload size {} does not match declared {}",
            uncompressed.len(),
            uncompressed_size
        )));
    }

    let mut offset = 0;

    // Read pattern
    let num_types = usize::try_from(uvarint_decode(&uncompressed, &mut offset)?)
        .map_err(|_| FqcError::Format("ID token type count exceeds platform limits".to_string()))?;
    if num_types > uncompressed.len().saturating_sub(offset) {
        return Err(FqcError::Format("Truncated tokenize token-type table".to_string()));
    }
    let mut token_types = Vec::with_capacity(num_types);
    for _ in 0..num_types {
        let raw = *uncompressed
            .get(offset)
            .ok_or_else(|| FqcError::Format("Truncated tokenize token-type table".to_string()))?;
        offset += 1;
        let tt = TokenType::try_from_u8(raw).ok_or_else(|| FqcError::Format(format!("Unknown token type {raw}")))?;
        token_types.push(tt);
    }

    // Static values
    let num_static = usize::try_from(uvarint_decode(&uncompressed, &mut offset)?)
        .map_err(|_| FqcError::Format("ID static-value count exceeds platform limits".to_string()))?;
    let expected_static = token_types.iter().filter(|&&t| t == TokenType::Static).count();
    if num_static != expected_static {
        return Err(FqcError::Format(format!(
            "Tokenize static-value count {num_static} does not match pattern {expected_static}"
        )));
    }
    let mut static_values = Vec::with_capacity(num_static);
    for _ in 0..num_static {
        let len = usize::try_from(uvarint_decode(&uncompressed, &mut offset)?)
            .map_err(|_| FqcError::Format("Tokenize static value length exceeds platform limits".to_string()))?;
        let end = offset
            .checked_add(len)
            .ok_or_else(|| FqcError::Format("Tokenize static value length overflows".to_string()))?;
        if end > uncompressed.len() {
            return Err(FqcError::Format("Truncated tokenize static data".to_string()));
        }
        static_values.push(
            String::from_utf8(uncompressed[offset..end].to_vec())
                .map_err(|e| FqcError::Format(format!("Invalid UTF-8 in tokenize static data: {e}")))?,
        );
        offset = end;
    }

    // Delimiters
    let num_delims = usize::try_from(uvarint_decode(&uncompressed, &mut offset)?)
        .map_err(|_| FqcError::Format("ID delimiter count exceeds platform limits".to_string()))?;
    let expected_delims = token_types.iter().filter(|&&t| t == TokenType::Delimiter).count();
    if num_delims != expected_delims {
        return Err(FqcError::Format(format!(
            "Tokenize delimiter count {num_delims} does not match pattern {expected_delims}"
        )));
    }
    let mut delimiters = Vec::with_capacity(num_delims);
    for _ in 0..num_delims {
        let delimiter = *uncompressed
            .get(offset)
            .ok_or_else(|| FqcError::Format("Truncated tokenize delimiter table".to_string()))?;
        offset += 1;
        if !is_delimiter(delimiter) {
            return Err(FqcError::Format(format!(
                "Invalid tokenize delimiter byte 0x{delimiter:02x}"
            )));
        }
        delimiters.push(delimiter);
    }

    // Integer columns
    let num_int_cols = usize::try_from(uvarint_decode(&uncompressed, &mut offset)?)
        .map_err(|_| FqcError::Format("ID integer-column count exceeds platform limits".to_string()))?;
    let expected_int_cols = token_types.iter().filter(|&&t| t == TokenType::DynamicInt).count();
    if num_int_cols != expected_int_cols {
        return Err(FqcError::Format(format!(
            "Tokenize integer-column count {num_int_cols} does not match pattern {expected_int_cols}"
        )));
    }
    let mut int_columns = Vec::with_capacity(num_int_cols);
    for _ in 0..num_int_cols {
        let encoded_len = usize::try_from(uvarint_decode(&uncompressed, &mut offset)?)
            .map_err(|_| FqcError::Format("Tokenize integer-column length exceeds platform limits".to_string()))?;
        let end = offset
            .checked_add(encoded_len)
            .ok_or_else(|| FqcError::Format("Tokenize integer-column length overflows".to_string()))?;
        if end > uncompressed.len() {
            return Err(FqcError::Format("Truncated tokenize int column".to_string()));
        }
        let col = delta_varint_decode(&uncompressed[offset..end], num_ids as usize)?;
        offset = end;
        int_columns.push(col);
    }

    // String columns
    let num_str_cols = usize::try_from(uvarint_decode(&uncompressed, &mut offset)?)
        .map_err(|_| FqcError::Format("ID string-column count exceeds platform limits".to_string()))?;
    let expected_str_cols = token_types.iter().filter(|&&t| t == TokenType::DynamicString).count();
    if num_str_cols != expected_str_cols {
        return Err(FqcError::Format(format!(
            "Tokenize string-column count {num_str_cols} does not match pattern {expected_str_cols}"
        )));
    }
    let mut str_columns: Vec<Vec<String>> = Vec::with_capacity(num_str_cols);
    for _ in 0..num_str_cols {
        let mut col = Vec::with_capacity(num_ids as usize);
        for _ in 0..num_ids {
            let len = usize::try_from(uvarint_decode(&uncompressed, &mut offset)?)
                .map_err(|_| FqcError::Format("Tokenize string length exceeds platform limits".to_string()))?;
            let end = offset
                .checked_add(len)
                .ok_or_else(|| FqcError::Format("Tokenize string length overflows".to_string()))?;
            if end > uncompressed.len() {
                return Err(FqcError::Format("Truncated tokenize string column".to_string()));
            }
            col.push(
                String::from_utf8(uncompressed[offset..end].to_vec())
                    .map_err(|e| FqcError::Format(format!("Invalid UTF-8 in tokenize string data: {e}")))?,
            );
            offset = end;
        }
        str_columns.push(col);
    }

    if offset != uncompressed.len() {
        return Err(FqcError::Format("ID tokenize stream has trailing bytes".to_string()));
    }

    // Reconstruct IDs using the shared rebuild path
    let ids = rebuild_ids_from_pattern(
        &token_types,
        &static_values,
        &delimiters,
        &int_columns,
        &str_columns,
        num_ids as usize,
    );

    Ok(ids)
}

// =============================================================================
// Shared Reconstruction
// =============================================================================

/// Reconstruct IDs from a detected pattern plus per-read dynamic columns.
/// Used by both the compression-side losslessness check and the decoder so
/// the two paths can never drift apart.
fn rebuild_ids_from_pattern(
    token_types: &[TokenType],
    static_values: &[String],
    delimiters: &[u8],
    int_columns: &[Vec<i64>],
    str_columns: &[Vec<String>],
    num_ids: usize,
) -> Vec<String> {
    let mut ids = Vec::with_capacity(num_ids);
    for i in 0..num_ids {
        let mut id = String::new();
        let mut si = 0;
        let mut di = 0;
        let mut ii = 0;
        let mut sti = 0;
        for tt in token_types {
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
    ids
}

// =============================================================================
// Public API
// =============================================================================

/// Compress a block of read IDs according to `id_mode`.
/// `Tokenize` tries pattern detection and falls back to exact.
pub fn compress_ids(ids: &[&str], zstd_level: i32, id_mode: IdMode) -> Result<Vec<u8>> {
    match id_mode {
        IdMode::Discard => return Ok(vec![MAGIC_DISCARD]),
        IdMode::Exact => return compress_exact(ids, zstd_level),
        IdMode::Tokenize => {}
    }
    if ids.is_empty() {
        return compress_exact(ids, zstd_level);
    }
    if let Some(pattern) = detect_pattern(ids) {
        if pattern.num_dynamic_ints > 0 {
            return compress_tokenize(ids, &pattern, zstd_level);
        }
    }
    compress_exact(ids, zstd_level)
}

/// Decompress a block of read IDs.
/// `id_prefix` is used for discard mode to generate placeholder IDs.
pub fn decompress_ids(data: &[u8], num_ids: u32, id_prefix: &str) -> Result<Vec<String>> {
    if data.is_empty() {
        return if num_ids == 0 {
            Ok(Vec::new())
        } else {
            Err(FqcError::Format("ID stream is empty for a non-empty block".to_string()))
        };
    }

    let magic = data[0];
    let payload = &data[1..];

    match magic {
        MAGIC_EXACT => decompress_exact(payload, num_ids),
        MAGIC_TOKENIZE => decompress_tokenize(payload, num_ids),
        MAGIC_DISCARD => {
            if !payload.is_empty() {
                return Err(FqcError::Format("ID discard stream has trailing bytes".to_string()));
            }
            Ok((1..=num_ids as u64).map(|i| format!("{}{}", id_prefix, i)).collect())
        }
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

    let max_out = (num_ids as usize).saturating_mul(64 * 1024).saturating_add(1024);
    let buf = crate::memory_budget::zstd_decompress_bounded(data, max_out, "id legacy")?;

    let mut ids = Vec::with_capacity(num_ids as usize);
    let mut cur = Cursor::new(&buf);

    for _ in 0..num_ids {
        let len = cur
            .read_u16::<LittleEndian>()
            .map_err(|e| FqcError::Format(format!("Truncated ID data: {e}")))?;
        let mut id = vec![0u8; len as usize];
        cur.read_exact(&mut id)
            .map_err(|e| FqcError::Format(format!("Truncated ID bytes: {e}")))?;
        ids.push(String::from_utf8(id).map_err(|e| FqcError::Format(format!("Invalid UTF-8 in legacy ID data: {e}")))?);
    }

    if cur.position() as usize != buf.len() {
        return Err(FqcError::Format("Legacy ID stream has trailing bytes".to_string()));
    }

    Ok(ids)
}
