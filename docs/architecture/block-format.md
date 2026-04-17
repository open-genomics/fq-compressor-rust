# FQC Block Header Format Specification

This document specifies the FQC block header format, which describes the structure and metadata of each compressed block within an `.fqc` archive.

## Overview

Every block in an FQC archive begins with a fixed-size header that describes the codecs used, stream layout, sizes, and checksums. Block headers enable random-access decompression by providing all necessary metadata to locate and decode each stream independently.

## Block Header Layout

The block header is **104 bytes** fixed-size, with all integers stored in **little-endian** byte order.

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0 | 4 | `header_size` | `u32` | Header size in bytes (always 104 for v2) |
| 4 | 4 | `block_id` | `u32` | Zero-based block index |
| 8 | 1 | `checksum_type` | `u8` | Checksum algorithm (0 = XxHash64) |
| 9 | 1 | `codec_ids` | `u8` | Codec for ID stream (4-bit family + 4-bit version) |
| 10 | 1 | `codec_seq` | `u8` | Codec for sequence stream |
| 11 | 1 | `codec_qual` | `u8` | Codec for quality stream |
| 12 | 1 | `codec_aux` | `u8` | Codec for aux stream (read lengths) |
| 13 | 1 | `reserved1` | `u8` | Reserved (must be 0) |
| 14 | 2 | `reserved2` | `u16` | Reserved (must be 0) |
| 16 | 8 | `block_xxhash64` | `u64` | XxHash64 checksum of uncompressed block data |
| 24 | 4 | `uncompressed_count` | `u32` | Number of reads in this block |
| 28 | 4 | `uniform_read_length` | `u32` | Read length if all reads are same length, 0 otherwise |
| 32 | 8 | `compressed_size` | `u64` | Total payload size (all four streams combined) |
| 40 | 8 | `offset_ids` | `u64` | Byte offset of ID stream from payload start |
| 48 | 8 | `offset_seq` | `u64` | Byte offset of sequence stream from payload start |
| 56 | 8 | `offset_qual` | `u64` | Byte offset of quality stream from payload start |
| 64 | 8 | `offset_aux` | `u64` | Byte offset of aux stream from payload start |
| 72 | 8 | `size_ids` | `u64` | Size of ID stream in bytes |
| 80 | 8 | `size_seq` | `u64` | Size of sequence stream in bytes |
| 88 | 8 | `size_qual` | `u64` | Size of quality stream in bytes |
| 96 | 8 | `size_aux` | `u64` | Size of aux stream in bytes |

## Payload Layout

Immediately following the 104-byte block header are the four payload streams, concatenated in order:

```
+------------------+
|  Block Header    |  104 bytes
+------------------+
|  ID Stream       |  size_ids bytes
+------------------+
|  Sequence Stream |  size_seq bytes
+------------------+
|  Quality Stream  |  size_qual bytes
+------------------+
|  Aux Stream      |  size_aux bytes
+------------------+
```

Stream offsets (`offset_*`) are relative to the first byte after the block header. The sizes and offsets must satisfy:

```
compressed_size = size_ids + size_seq + size_qual + size_aux
offset_ids = 0
offset_seq = size_ids
offset_qual = size_ids + size_seq
offset_aux = size_ids + size_seq + size_qual
```

## Codec Encoding

Each codec byte encodes a 4-bit family and a 4-bit version:

```
codec_byte = (family << 4) | (version & 0x0F)
```

### Codec Families

| Family Code | Name | Description |
|-------------|------|-------------|
| `0x0` | `Raw` | Uncompressed / placeholder |
| `0x1` | `AbcV1` | ABC consensus + delta encoding (v1) |
| `0x2` | `ScmV1` | Statistical Context Model (v1) |
| `0x3` | `DeltaLzma` | Delta encoding + LZMA |
| `0x4` | `DeltaZstd` | Delta encoding + Zstd |
| `0x5` | `DeltaVarint` | Delta encoding + varint |
| `0x6` | `OverlapV1` | Overlap compression (v1) |
| `0x7` | `ZstdPlain` | Plain Zstd compression |
| `0x8` | `ScmOrder1` | SCM with Order-1 context |
| `0xE` | `External` | External codec |
| `0xF` | `Reserved` | Reserved for future use |

### Typical Codec Assignments by Read Length Class

| Stream | Short Reads | Medium Reads | Long Reads |
|--------|-------------|--------------|------------|
| IDs | `DeltaZstd` (0x40) | `DeltaZstd` (0x40) | `DeltaZstd` (0x40) |
| Sequences | `AbcV1` (0x10) | `ZstdPlain` (0x70) | `ZstdPlain` (0x70) |
| Quality | `ScmV1` (0x20) | `ScmV1` (0x20) | `ScmOrder1` (0x80) |
| Aux | `DeltaVarint` (0x50) | `DeltaVarint` (0x50) | `DeltaVarint` (0x50) |

## Special Field Semantics

### `uniform_read_length`

- **Non-zero**: All reads in this block have the same length. This value stores that length in bases.
- **Zero**: Reads have variable lengths; actual lengths are stored in the aux stream.

When `uniform_read_length > 0` and `size_aux == 0`, the aux stream is omitted and the block is marked as having uniform length via `has_uniform_length()`.

### `block_xxhash64`

XxHash64 checksum computed over the original uncompressed block data. Used for integrity verification during decompression. A mismatch triggers a `ChecksumMismatch` error.

### Discarded Quality Detection

A block is considered to have discarded quality when:
- `size_qual == 0`, AND
- `codec_qual` decodes to `Raw` family

In this case, decompression generates placeholder quality strings (`"!"` repeated per read length).

### Reserved Fields

`reserved1` (u8) and `reserved2` (u16) must be zero. The reader validates this and returns a `Format` error if non-zero values are encountered.

## Forward Compatibility

The `header_size` field allows for future format extensions. If `header_size > 104`, the reader skips the extra bytes:

```rust
if header_size as usize > BLOCK_HEADER_SIZE {
    let extra = header_size as usize - BLOCK_HEADER_SIZE;
    let mut skip = vec![0u8; extra];
    r.read_exact(&mut skip)?;
}
```

This design allows new fields to be added without breaking existing readers.

## Block Index Integration

Each block's starting offset is recorded in the `BlockIndex` at the end of the file. The index entry (28 bytes per block) contains:

| Field | Size | Description |
|-------|------|-------------|
| `offset` | `u64` | File offset of block header start |
| `compressed_size` | `u64` | Total block size (header + payload) |
| `archive_id_start` | `u64` | First archive read ID in this block |
| `read_count` | `u32` | Number of reads in this block |

To locate a block:
1. Read `BlockIndex` from `footer.index_offset`
2. Look up the entry for the desired `block_id`
3. Seek to `entry.offset`
4. Read the 104-byte block header
5. Read streams at `payload_start + offset_*`

## Serialization

The `BlockHeader::write()` method serializes all fields in the order specified by the layout table above. It always writes exactly 104 bytes:

```rust
pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
    w.write_u32::<LittleEndian>(BLOCK_HEADER_SIZE as u32)?; // header_size
    w.write_u32::<LittleEndian>(self.block_id)?;
    w.write_u8(self.checksum_type)?;
    w.write_u8(self.codec_ids)?;
    w.write_u8(self.codec_seq)?;
    w.write_u8(self.codec_qual)?;
    w.write_u8(self.codec_aux)?;
    w.write_u8(self.reserved1)?;
    w.write_u16::<LittleEndian>(self.reserved2)?;
    w.write_u64::<LittleEndian>(self.block_xxhash64)?;
    // ... remaining fields
    Ok(())
}
```

## Example: Reading a Block

```rust
// Open archive
let mut reader = FqcReader::open("sample.fqc")?;

// Read block 0 header
let header = reader.read_block_header(0)?;
assert_eq!(header.header_size, 104);
assert_eq!(header.uncompressed_count, 100_000);
assert_eq!(header.uniform_read_length, 151);

// Read full block (all streams)
let block = reader.read_block(0)?;
assert_eq!(block.header.block_id, 0);
assert_eq!(block.ids_data.len() as u64, block.header.size_ids);
```

## Related Documents

- [Full FQC Format Specification](./format-spec.md)
- [Source Module Overview](./modules.md)
- [Strategy Selection](../algorithms/strategy-selection.md)
- [ABC Algorithm](../algorithms/abc.md)
- [SCM Quality Compression](../algorithms/scm.md)
