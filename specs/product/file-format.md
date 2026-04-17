# FQC File Format Specification

**Status**: ✅ Implemented  
**Version**: 1.0

## Overview

FQC is a block-indexed binary format for storing compressed FASTQ data. It supports random access to individual blocks and optional read reorder mapping for restoring original order.

**Implementation**:
- Structure definitions: `src/format.rs`
- Reading: `src/fqc_reader.rs`
- Writing: `src/fqc_writer.rs`

## File Layout

```
┌─────────────────────────────┐
│   Magic Header (9 bytes)    │
├─────────────────────────────┤
│   Global Header (variable)   │
├─────────────────────────────┤
│   Block 0                   │
├─────────────────────────────┤
│   Block 1                   │
├─────────────────────────────┤
│   ...                       │
├─────────────────────────────┤
│   Block N-1                 │
├─────────────────────────────┤
│   Reorder Map (optional)     │
├─────────────────────────────┤
│   Block Index               │
├─────────────────────────────┤
│   File Footer (32 bytes)    │
└─────────────────────────────┘
```

---

## Magic Header (9 bytes)

| Offset | Size | Value | Description |
|--------|------|-------|-------------|
| 0 | 4 | `\x89FQC` | Magic signature (high-bit detects binary corruption) |
| 4 | 2 | `\r\n` | DOS line ending detection |
| 6 | 1 | `\x1a` | Ctrl-Z (prevents DOS `type` command output) |
| 7 | 1 | `\n` | Unix line ending detection |
| 8 | 1 | `0x01` | Format major version |

**Design**: Inspired by PNG file signature, detects multiple common file transfer corruptions.

---

## Global Header

| Field | Type | Description |
|-------|------|-------------|
| header_size | u32 | Total header size in bytes |
| flags | u64 | Bit flags (see below) |
| compression_algo | u8 | Reserved (0) |
| checksum_type | u8 | 1 = xxHash64 |
| reserved | u16 | Reserved (0) |
| total_read_count | u64 | Total number of reads in archive |
| filename_len | u16 | Original filename length |
| filename | [u8] | Original filename (UTF-8) |
| timestamp | u64 | Creation time (Unix timestamp) |

### Flags (u64)

| Bit | Name | Description |
|-----|------|-------------|
| 0 | IS_PAIRED | Paired-end data |
| 1 | HAS_REORDER_MAP | Archive contains reorder map |
| 2-3 | QUALITY_MODE | 0=Lossless, 1=Illumina8Bin, 2=Discard |
| 4-5 | ID_MODE | 0=Exact, 1=StripComment, 2=Discard |
| 6-7 | LENGTH_CLASS | 0=Short, 1=Medium, 2=Long |
| 8-9 | PE_LAYOUT | 0=Interleaved, 1=Consecutive |

---

## Block Header

Each compressed block starts with a block header:

| Field | Type | Description |
|-------|------|-------------|
| block_id | u32 | Sequential block identifier |
| uncompressed_count | u32 | Number of reads in block |
| uniform_read_length | u16 | Uniform read length (0 if variable) |
| codec_seq | u8 | Sequence codec (0=ABC, 1=Zstd) |
| codec_qual | u8 | Quality codec (0=SCM-O2, 1=SCM-O1, 2=Discard) |
| ids_compressed_size | u32 | Compressed ID data size |
| seq_compressed_size | u32 | Compressed sequence data size |
| qual_compressed_size | u32 | Compressed quality data size |
| aux_size | u32 | Auxiliary data size |

Followed by 4 compressed data sections: IDs, Sequences, Quality, Aux.

### Codec Selection

| codec_seq Value | Algorithm | Use Case |
|-----------------|-----------|----------|
| 0 | ABC (consensus + delta) | Short reads (< 300bp) |
| 1 | Zstd (length-prefixed) | Medium/long reads |

See [Compression Algorithms RFC](../rfc/0002-compression-algorithms.md) for details.

---

## Reorder Map (Optional)

Present when `HAS_REORDER_MAP` flag is set. Contains bidirectional mapping between archive order and original order.

| Field | Type | Description |
|-------|------|-------------|
| map_size | u64 | Total map segment size |
| num_reads | u64 | Number of reads |
| forward_map | [varint] | Forward map (original → archive) |
| reverse_map | [varint] | Reverse map (archive → original) |

### ZigZag Varint Encoding

Differences between adjacent map entries use ZigZag encoding (handles negative deltas) + unsigned varint encoding for compact storage:

```
delta = current_id - previous_id
zigzag = (delta << 1) ^ (delta >> 63)    // maps negative to positive
varint: 7 bits/byte, MSB=1 means more bytes follow
```

**Implementation**: `src/reorder_map.rs`

---

## Block Index

Array of block offsets for random access:

| Field | Type | Description |
|-------|------|-------------|
| num_blocks | u32 | Number of blocks |
| entries | [BlockIndexEntry] | Index entries array |

Each `BlockIndexEntry`:

| Field | Type | Description |
|-------|------|-------------|
| offset | u64 | File offset of block start |
| archive_id_start | u64 | First read ID in block (archive order) |
| read_count | u32 | Number of reads in block |

---

## File Footer (32 bytes)

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 0 | 8 | u64 | Block Index offset |
| 8 | 8 | u64 | xxHash64 checksum of all block data |
| 16 | 8 | u64 | Reserved |
| 24 | 4 | [u8;4] | Tail magic `FQC\0` |
| 28 | 4 | u32 | Footer size (32) |

---

## Byte Order

All multi-byte integers stored in **little-endian** order.

## Checksums

xxHash64 used for data integrity verification. Footer checksum covers all block data (block headers + compressed data).

Verification available via `fqc verify` command.

---

## Version Compatibility

| Format Version | fqc Version | Status |
|----------------|-------------|--------|
| 1 | 0.1.x | Current |

Future versions will maintain backward compatibility where possible. Format version bumps require code changes and test updates.

## Related Documents

- [Core Compression Spec](./core-compression.md)
- [CLI Commands Spec](./cli-commands.md)
- [Format Specification (docs)](../../docs/specs/format-spec.md)
