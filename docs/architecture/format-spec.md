# FQC Binary Format Specification

This document provides the complete specification of the `.fqc` binary format used by fqc for storing compressed FASTQ sequencing data.

## Version

| Field | Value |
|-------|-------|
| Format Major Version | 2 |
| Format Minor Version | 0 |
| Version Byte | `(2 << 4) | 0 = 0x20` |

## File Layout

```
+------------------+
|  Magic Header    |  9 bytes: [0x89, 'F', 'Q', 'C', 0x0D, 0x0A, 0x1A, 0x0A, version]
+------------------+
|  Global Header   |  Variable length (minimum 34 bytes)
+------------------+
|  Block 0         |  104-byte header + payload streams
+------------------+
|  Block 1         |  104-byte header + payload streams
+------------------+
|  ...             |
+------------------+
|  Block N         |  104-byte header + payload streams
+------------------+
|  Reorder Map     |  Optional: 32-byte header + compressed forward/reverse maps
+------------------+
|  Block Index     |  16-byte header + N × 28-byte entries
+------------------+
|  File Footer     |  32 bytes
+------------------+
```

All multi-byte integers are stored in **little-endian** byte order.

---

## 1. Magic Header

| Offset | Size | Value | Description |
|--------|------|-------|-------------|
| 0 | 8 | `[0x89, 0x46, 0x51, 0x43, 0x0D, 0x0A, 0x1A, 0x0A]` | Magic bytes (`\x89FQC\r\n\x1a\n`) |
| 8 | 1 | `0x20` | Format version (major=2, minor=0) |

**Total: 9 bytes**

### Validation

```rust
pub const MAGIC_BYTES: [u8; 8] = [0x89, b'F', b'Q', b'C', 0x0D, 0x0A, 0x1A, 0x0A];

pub fn validate_magic(data: &[u8]) -> bool {
    data.len() >= 8 && data[..8] == MAGIC_BYTES
}

pub fn is_version_compatible(version: u8) -> bool {
    let major = version >> 4;
    major == FORMAT_VERSION_MAJOR  // major == 2
}
```

---

## 2. Global Header

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0 | 4 | `header_size` | `u32` | Total header size (34 + filename length) |
| 4 | 8 | `flags` | `u64` | Archive feature flags |
| 12 | 1 | `compression_algo` | `u8` | Global compression algorithm (reserved, always 0) |
| 13 | 1 | `checksum_type` | `u8` | Checksum type (0 = XxHash64) |
| 14 | 2 | `reserved` | `u16` | Reserved (must be 0) |
| 16 | 8 | `total_read_count` | `u64` | Total FASTQ reads in archive |
| 24 | 2 | `filename_len` | `u16` | Length of original filename |
| 26 | var | `original_filename` | `utf8` | Original FASTQ filename |
| var | 8 | `timestamp` | `u64` | UNIX epoch seconds of creation |

**Minimum size: 34 bytes** (when filename is empty). Actual size: `34 + filename_len`.

### Flags Bitfield

| Bit | Mask | Name | Description |
|-----|------|------|-------------|
| 0 | `1 << 0` | `IS_PAIRED` | Archive contains paired-end data |
| 1 | `1 << 1` | `PRESERVE_ORDER` | Original read order preserved (no reordering) |
| 2 | `1 << 2` | `LEGACY_LONG_READ_MODE` | Legacy long-read mode |
| 3-4 | `0x3 << 3` | `QUALITY_MODE_MASK` | Quality mode (2 bits) |
| 5-6 | `0x3 << 5` | `ID_MODE_MASK` | ID mode (2 bits) |
| 7 | `1 << 7` | `HAS_REORDER_MAP` | Reorder map present after blocks |
| 8-9 | `0x3 << 8` | `PE_LAYOUT_MASK` | PE layout: 0=interleaved, 1=consecutive |
| 10-11 | `0x3 << 10` | `READ_LENGTH_CLASS_MASK` | Read length class: 0=short, 1=medium, 2=long |
| 12 | `1 << 12` | `STREAMING_MODE` | Archive written in streaming mode |

#### Quality Mode Values

| Value | Mode | Description |
|-------|------|-------------|
| 0 | `Lossless` | Full quality preservation |
| 1 | `Illumina8` | 8-bin quantization |
| 2 | `Qvz` | QVZ format |
| 3 | `Discard` | Quality scores removed |

#### ID Mode Values

| Value | Mode | Description |
|-------|------|-------------|
| 0 | `Exact` | Full ID preservation |
| 1 | `Tokenize` | Pattern-based tokenization |
| 2 | `Discard` | IDs removed |

#### PE Layout Values

| Value | Layout | Description |
|-------|--------|-------------|
| 0 | `Interleaved` | R1, R2, R1, R2, ... |
| 1 | `Consecutive` | All R1s, then all R2s |

#### Read Length Class Values

| Value | Class | Description |
|-------|-------|-------------|
| 0 | `Short` | All reads < 300bp (ABC algorithm) |
| 1 | `Medium` | Reads 300bp-10KB (Zstd) |
| 2 | `Long` | Reads > 10KB (Zstd) |

---

## 3. Block Format

Each block consists of a 104-byte header followed by four payload streams.

### Block Header (104 bytes)

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0 | 4 | `header_size` | `u32` | Always 104 |
| 4 | 4 | `block_id` | `u32` | Zero-based block index |
| 8 | 1 | `checksum_type` | `u8` | 0 = XxHash64 |
| 9 | 1 | `codec_ids` | `u8` | ID stream codec (4-bit family + 4-bit version) |
| 10 | 1 | `codec_seq` | `u8` | Sequence stream codec |
| 11 | 1 | `codec_qual` | `u8` | Quality stream codec |
| 12 | 1 | `codec_aux` | `u8` | Aux stream codec |
| 13 | 1 | `reserved1` | `u8` | Must be 0 |
| 14 | 2 | `reserved2` | `u16` | Must be 0 |
| 16 | 8 | `block_xxhash64` | `u64` | XxHash64 of uncompressed block |
| 24 | 4 | `uncompressed_count` | `u32` | Number of reads in block |
| 28 | 4 | `uniform_read_length` | `u32` | Common read length, or 0 |
| 32 | 8 | `compressed_size` | `u64` | Total payload bytes |
| 40 | 8 | `offset_ids` | `u64` | ID stream offset from payload start |
| 48 | 8 | `offset_seq` | `u64` | Sequence stream offset |
| 56 | 8 | `offset_qual` | `u64` | Quality stream offset |
| 64 | 8 | `offset_aux` | `u64` | Aux stream offset |
| 72 | 8 | `size_ids` | `u64` | ID stream size |
| 80 | 8 | `size_seq` | `u64` | Sequence stream size |
| 88 | 8 | `size_qual` | `u64` | Quality stream size |
| 96 | 8 | `size_aux` | `u64` | Aux stream size |

### Payload Streams

Immediately after the block header:

```
+------------+
| ID Stream  |  size_ids bytes
+------------+
| Seq Stream |  size_seq bytes
+------------+
| Qual Stream|  size_qual bytes
+------------+
| Aux Stream |  size_aux bytes
+------------+
```

Offsets are relative to the first byte after the block header.

### Codec Family Codes

| Code | Family | Description |
|------|--------|-------------|
| 0x0 | `Raw` | Uncompressed / placeholder |
| 0x1 | `AbcV1` | ABC consensus + delta |
| 0x2 | `ScmV1` | SCM arithmetic coding (Order-2) |
| 0x3 | `DeltaLzma` | Delta + LZMA |
| 0x4 | `DeltaZstd` | Delta + Zstd |
| 0x5 | `DeltaVarint` | Delta + varint |
| 0x6 | `OverlapV1` | Overlap compression |
| 0x7 | `ZstdPlain` | Plain Zstd |
| 0x8 | `ScmOrder1` | SCM (Order-1) |
| 0xE | `External` | External codec |
| 0xF | `Reserved` | Reserved |

Codec byte encoding: `codec = (family << 4) | (version & 0x0F)`

### Typical Codec Assignments

| Read Class | IDs | Sequences | Quality | Aux |
|------------|-----|-----------|---------|-----|
| Short | `0x40` | `0x10` | `0x20` | `0x50` |
| Medium | `0x40` | `0x70` | `0x20` | `0x50` |
| Long | `0x40` | `0x70` | `0x80` | `0x50` |
| Discard qual | `0x40` | `0x10/0x70` | `0x00` | `0x50` |

---

## 4. Reorder Map (Optional)

Present only when the `HAS_REORDER_MAP` flag is set.

### Reorder Map Header (32 bytes)

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0 | 4 | `header_size` | `u32` | Always 32 |
| 4 | 4 | `version` | `u32` | Map version (always 1) |
| 8 | 8 | `total_reads` | `u64` | Total reads in archive |
| 16 | 8 | `forward_map_size` | `u64` | Compressed forward map size |
| 24 | 8 | `reverse_map_size` | `u64` | Compressed reverse map size |

### Compressed Map Data

```
+------------------+
| Forward Map Data |  forward_map_size bytes (zstd + delta + varint)
+------------------+
| Reverse Map Data |  reverse_map_size bytes (zstd + delta + varint)
+------------------+
```

Each map is encoded as: `zstd(delta_encode(varint_encode(map)))`

---

## 5. Block Index

Located at the end of the file, pointed to by `footer.index_offset`.

### Index Header (16 bytes)

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0 | 4 | `header_size` | `u32` | Always 16 |
| 4 | 4 | `entry_size` | `u32` | Always 28 |
| 8 | 8 | `num_blocks` | `u64` | Number of blocks |

### Index Entry (28 bytes each)

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0 | 8 | `offset` | `u64` | File offset of block header |
| 8 | 8 | `compressed_size` | `u64` | Total block size (header + payload) |
| 16 | 8 | `archive_id_start` | `u64` | First read archive ID in block |
| 24 | 4 | `read_count` | `u32` | Number of reads in block |

**Total index size**: `16 + num_blocks × 28` bytes.

---

## 6. File Footer (32 bytes)

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0 | 8 | `index_offset` | `u64` | File offset of block index |
| 8 | 8 | `reorder_map_offset` | `u64` | File offset of reorder map (0 if absent) |
| 16 | 8 | `global_checksum` | `u64` | XxHash64 of all block payloads |
| 24 | 8 | `magic_end` | `[u8; 8]` | `['F', 'Q', 'C', '_', 'E', 'O', 'F', 0x00]` |

The footer is always at `file_size - 32`.

### Validation

```rust
pub const MAGIC_END: [u8; 8] = [b'F', b'Q', b'C', b'_', b'E', b'O', b'F', 0x00];

pub fn is_valid(&self) -> bool {
    self.magic_end == MAGIC_END
}

pub fn has_reorder_map(&self) -> bool {
    self.reorder_map_offset != 0
}
```

---

## Complete File Walkthrough

### Writing an Archive

```
1. Write magic bytes (8 bytes) + version byte (1 byte)
   → offset: 9

2. Write GlobalHeader (34 + filename_len bytes)
   → offset: 9 + header_size

3. For each block:
   a. Write BlockHeader (104 bytes)
   b. Write ID stream (size_ids bytes)
   c. Write sequence stream (size_seq bytes)
   d. Write quality stream (size_qual bytes)
   e. Write aux stream (size_aux bytes)
   f. Record IndexEntry with offset and sizes
   → offset: previous + 104 + compressed_size

4. If reordering performed:
   a. Write ReorderMapHeader (32 bytes)
   b. Write compressed forward map
   c. Write compressed reverse map
   → offset: previous + 32 + fwd_size + rev_size

5. Write BlockIndex:
   a. Write index header (16 bytes)
   b. Write N index entries (28 bytes each)
   → offset: previous + 16 + N * 28

6. Write FileFooter (32 bytes)
   → offset: previous + 32 (= file_size)
```

### Reading an Archive

```
1. Read magic (8 bytes) + version (1 byte)
2. Validate magic bytes and version compatibility
3. Get file_size from file metadata
4. Seek to file_size - 32, read FileFooter
5. Seek to 9, read GlobalHeader
6. Seek to footer.index_offset, read BlockIndex
7. (Optional) Seek to footer.reorder_map_offset, read ReorderMap
```

### Random Access to a Block

```
1. Read BlockIndex (from footer.index_offset)
2. Look up entry for desired block_id
3. Seek to entry.offset
4. Read BlockHeader (104 bytes)
5. Seek to entry.offset + 104 + offset_ids, read ID stream
6. Seek to entry.offset + 104 + offset_seq, read sequence stream
7. Seek to entry.offset + 104 + offset_qual, read quality stream
8. Seek to entry.offset + 104 + offset_aux, read aux stream
9. Verify block_xxhash64 against computed checksum
```

---

## Version Compatibility

### Reader Compatibility

A reader is compatible with an archive if the **major version** matches. Minor version differences are assumed to be backward-compatible additions.

```rust
pub fn is_version_compatible(version: u8) -> bool {
    let major = version >> 4;
    major == FORMAT_VERSION_MAJOR  // major == 2
}
```

### Forward Compatibility

Both `GlobalHeader` and `BlockHeader` include `header_size` fields. Readers skip any bytes beyond the known header size, allowing new fields to be added without breaking existing readers:

```rust
// GlobalHeader: skip extra bytes
let read_so_far = GLOBAL_HEADER_MIN_SIZE + fname_len;
if header_size as usize > read_so_far {
    let extra = header_size as usize - read_so_far;
    let mut skip = vec![0u8; extra];
    r.read_exact(&mut skip)?;
}

// BlockHeader: skip extra bytes
if header_size as usize > BLOCK_HEADER_SIZE {
    let extra = header_size as usize - BLOCK_HEADER_SIZE;
    let mut skip = vec![0u8; extra];
    r.read_exact(&mut skip)?;
}
```

---

## Error Conditions

| Condition | Error Type | Description |
|-----------|------------|-------------|
| Invalid magic (first 8 bytes) | `Format` | Not an FQC file |
| Incompatible major version | `UnsupportedVersion` | Format version mismatch |
| File too small (< 41 bytes) | `Format` | Too small for valid archive |
| Invalid footer magic | `Format` | Corrupted or truncated file |
| Reserved fields non-zero | `Format` | Invalid header |
| Block index entry size < 28 | `Format` | Corrupted index |
| Checksum mismatch | `ChecksumMismatch` | Data corruption detected |

---

## Related Documents

- [Block Format Specification](./block-format.md)
- [Source Module Overview](./modules.md)
- [Reorder Map Architecture](./reorder-map.md)
- [File Format Product Spec](../../specs/product/file-format.md)
