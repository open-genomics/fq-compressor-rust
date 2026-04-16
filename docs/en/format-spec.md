# FQC File Format Specification

> Version: 1.0
>
> See also: [algorithms.md](algorithms.md) (compression algorithms), [architecture.md](architecture.md) (implementation modules)

## Overview

FQC is a block-indexed binary format for storing compressed FASTQ data. Supports random access to individual blocks, and optional read reordering mapping to restore original order.

Implementation is in `src/format.rs` (struct definitions), `src/fqc_reader.rs` (reading), `src/fqc_writer.rs` (writing).

## File Layout

```
┌─────────────────────────────┐
│   Magic Header (9 bytes)    │
├─────────────────────────────┤
│   Global Header (variable)  │
├─────────────────────────────┤
│   Block 0                   │
├─────────────────────────────┤
│   Block 1                   │
├─────────────────────────────┤
│   ...                       │
├─────────────────────────────┤
│   Block N-1                 │
├─────────────────────────────┤
│   Reorder Map (optional)    │
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
| 0 | 4 | `\x89FQC` | Magic signature (high-bit detects binary transmission corruption) |
| 4 | 2 | `\r\n` | DOS line ending detection |
| 6 | 1 | `\x1a` | Ctrl-Z (prevents DOS `type` command output) |
| 7 | 1 | `\n` | Unix line ending detection |
| 8 | 1 | `0x01` | Format major version |

Design inspired by PNG file signature, detecting multiple common file transmission corruptions simultaneously.

## Global Header

| Field | Type | Description |
|-------|------|-------------|
| header_size | u32 | Total header size (bytes) |
| flags | u64 | Bit flags (see table below) |
| compression_algo | u8 | Reserved (0) |
| checksum_type | u8 | 1 = xxHash64 |
| reserved | u16 | Reserved (0) |
| total_read_count | u64 | Total read count in archive |
| filename_len | u16 | Original filename length |
| filename | [u8] | Original filename (UTF-8) |
| timestamp | u64 | Creation time (Unix timestamp) |

### Flags (u64)

| Bit | Name | Description |
|-----|------|-------------|
| 0 | IS_PAIRED | Paired-end data |
| 1 | HAS_REORDER_MAP | Archive contains reordering mapping |
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
| uncompressed_count | u32 | Read count within block |
| uniform_read_length | u16 | Uniform read length (0 for variable length) |
| codec_seq | u8 | Sequence codec (0=ABC, 1=Zstd) |
| codec_qual | u8 | Quality codec (0=SCM-O2, 1=SCM-O1, 2=Discard) |
| ids_compressed_size | u32 | Compressed ID data size |
| seq_compressed_size | u32 | Compressed sequence data size |
| qual_compressed_size | u32 | Compressed quality data size |
| aux_size | u32 | Auxiliary data size |

After the header, 4 compressed data segments follow: IDs, Sequences, Quality, Aux.

### Codec Selection

| codec_seq value | Algorithm | Use Case |
|-----------------|-----------|----------|
| 0 | ABC (consensus + delta) | Short reads (< 300bp) |
| 1 | Zstd (length-prefixed) | Medium/long reads |

See [algorithms.md](algorithms.md) for details.

---

## Reorder Map (Optional)

Present when `HAS_REORDER_MAP` flag is set. Contains bidirectional mapping between archive order and original order.

| Field | Type | Description |
|-------|------|-------------|
| map_size | u64 | Total mapping segment size |
| num_reads | u64 | Read count |
| forward_map | [varint] | ZigZag delta-encoded forward mapping (original → archive) |
| reverse_map | [varint] | ZigZag delta-encoded reverse mapping (archive → original) |

### ZigZag Varint Encoding

Adjacent mapping entry differences use ZigZag encoding (handles negative differences) + unsigned varint encoding for compact storage:

```
delta = current_id - previous_id
zigzag = (delta << 1) ^ (delta >> 63)    // Maps negatives to positives
varint: 7 bits/byte, MSB=1 indicates continuation
```

Implementation is in `src/reorder_map.rs`.

---

## Block Index

Block offset array supporting random access:

| Field | Type | Description |
|-------|------|-------------|
| num_blocks | u32 | Block count |
| entries | [BlockIndexEntry] | Index entry array |

Each `BlockIndexEntry`:

| Field | Type | Description |
|-------|------|-------------|
| offset | u64 | Block start file offset |
| archive_id_start | u64 | First read ID in block (archive order) |
| read_count | u32 | Read count within block |

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

All multi-byte integers use **little-endian** storage.

## Checksums

xxHash64 is used for data integrity verification. The checksum in Footer covers all block data (block headers + compressed data).

Archive integrity can be verified via `fqc verify` command.
