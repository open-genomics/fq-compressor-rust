# FQC File Format

> Version: 1.0

## Overview

FQC is a block-indexed binary format for storing compressed FASTQ data. It supports random access to individual blocks and an optional read reorder map to restore original ordering.

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
| 0 | 4 | `\x89FQC` | Magic signature (high-bit detects binary transfer corruption) |
| 4 | 2 | `\r\n` | DOS line ending detection |
| 6 | 1 | `\x1a` | Ctrl-Z (prevents DOS `type` command output) |
| 7 | 1 | `\n` | Unix line ending detection |
| 8 | 1 | `0x01` | Format major version |

Designed after the PNG file signature to detect common file transfer corruption.

## Global Header

| Field | Type | Description |
|-------|------|-------------|
| header_size | u32 | Total header size (bytes) |
| flags | u64 | Bit flags (see below) |
| compression_algo | u8 | Reserved (0) |
| checksum_type | u8 | 1 = xxHash64 |
| reserved | u16 | Reserved (0) |
| total_read_count | u64 | Total reads in archive |
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

## Block Header

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

Four compressed data segments follow the block header: IDs, Sequences, Quality, Aux.

## Block Index

| Field | Type | Description |
|-------|------|-------------|
| num_blocks | u32 | Number of blocks |
| entries | [BlockIndexEntry] | Index entry array |

Each `BlockIndexEntry`:

| Field | Type | Description |
|-------|------|-------------|
| offset | u64 | File offset of block start |
| archive_id_start | u64 | First read ID in block (archive order) |
| read_count | u32 | Reads in block |

## File Footer (32 bytes)

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 0 | 8 | u64 | Block Index offset |
| 8 | 8 | u64 | xxHash64 checksum of all block data |
| 16 | 8 | u64 | Reserved |
| 24 | 4 | [u8;4] | Tail magic `FQC\0` |
| 28 | 4 | u32 | Footer size (32) |

## Byte Order

All multi-byte integers are stored in **little-endian** byte order.

## Checksum

xxHash64 is used for data integrity verification. The footer checksum covers all block data (block headers + compressed data). Use `fqc verify` to check archive integrity.
