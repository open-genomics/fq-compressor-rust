# FQC File Format Specification

Version: 1.0

## Overview

FQC is a block-indexed binary format for compressed FASTQ data. It supports random access to individual blocks and optional read reorder maps for restoring original order.

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

## Magic Header (9 bytes)

| Offset | Size | Value | Description |
|--------|------|-------|-------------|
| 0 | 4 | `\x89FQC` | Magic signature (high bit set to detect binary transfer corruption) |
| 4 | 2 | `\r\n` | DOS line ending check |
| 6 | 1 | `\x1a` | Ctrl-Z (stops DOS `type` command) |
| 7 | 1 | `\n` | Unix line ending check |
| 8 | 1 | `0x01` | Format major version |

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
| timestamp | u64 | Unix timestamp of creation |

### Flags (u64)

| Bit | Name | Description |
|-----|------|-------------|
| 0 | IS_PAIRED | Paired-end data |
| 1 | HAS_REORDER_MAP | Archive contains a reorder map |
| 2-3 | QUALITY_MODE | 0=Lossless, 1=Illumina8Bin, 2=Discard |
| 4-5 | ID_MODE | 0=Exact, 1=StripComment, 2=Discard |
| 6-7 | LENGTH_CLASS | 0=Short, 1=Medium, 2=Long |
| 8-9 | PE_LAYOUT | 0=Interleaved, 1=Consecutive |

## Block Header

Each compressed block starts with a header:

| Field | Type | Description |
|-------|------|-------------|
| block_id | u32 | Sequential block identifier |
| uncompressed_count | u32 | Number of reads in this block |
| uniform_read_length | u16 | Uniform length (0 if variable) |
| codec_seq | u8 | Sequence codec (0=ABC, 1=Zstd) |
| codec_qual | u8 | Quality codec (0=SCM-O2, 1=SCM-O1, 2=Discard) |
| ids_compressed_size | u32 | Compressed ID data size |
| seq_compressed_size | u32 | Compressed sequence data size |
| qual_compressed_size | u32 | Compressed quality data size |
| aux_size | u32 | Auxiliary data size |

Following the header are the compressed data sections: IDs, Sequences, Quality, Aux.

## Reorder Map (optional)

Present when `HAS_REORDER_MAP` flag is set. Contains bidirectional mapping between archive order and original order.

| Field | Type | Description |
|-------|------|-------------|
| map_size | u64 | Total map section size |
| num_reads | u64 | Number of reads |
| forward_map | [varint] | ZigZag delta-encoded forward map (original → archive) |
| reverse_map | [varint] | ZigZag delta-encoded reverse map (archive → original) |

### ZigZag Varint Encoding

Deltas between consecutive map entries are encoded using ZigZag encoding (to handle negative deltas) followed by unsigned varint encoding for compact storage.

## Block Index

Array of block offsets for random access:

| Field | Type | Description |
|-------|------|-------------|
| num_blocks | u32 | Number of blocks |
| entries | [BlockIndexEntry] | Array of index entries |

Each `BlockIndexEntry`:

| Field | Type | Description |
|-------|------|-------------|
| offset | u64 | File offset of block start |
| archive_id_start | u64 | First read ID (archive order) in this block |
| read_count | u32 | Number of reads in block |

## File Footer (32 bytes)

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 0 | 8 | u64 | Block index offset |
| 8 | 8 | u64 | xxHash64 checksum of all blocks |
| 16 | 8 | u64 | Reserved |
| 24 | 4 | [u8;4] | Magic tail `FQC\0` |
| 28 | 4 | u32 | Footer size (32) |

## Byte Order

All multi-byte integers are stored in **little-endian** byte order.

## Checksums

xxHash64 is used for data integrity verification. The checksum in the footer covers all block data (headers + compressed data).
