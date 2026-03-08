# FQC 文件格式

> Version: 1.0

## 概述

FQC 是一种块索引二进制格式，用于存储压缩后的 FASTQ 数据。支持随机访问单个块，以及可选的读段重排映射来恢复原始顺序。

## 文件布局

```
┌─────────────────────────────┐
│   Magic Header (9 bytes)    │
├─────────────────────────────┤
│   Global Header (变长)       │
├─────────────────────────────┤
│   Block 0                   │
├─────────────────────────────┤
│   Block 1                   │
├─────────────────────────────┤
│   ...                       │
├─────────────────────────────┤
│   Block N-1                 │
├─────────────────────────────┤
│   Reorder Map (可选)         │
├─────────────────────────────┤
│   Block Index               │
├─────────────────────────────┤
│   File Footer (32 bytes)    │
└─────────────────────────────┘
```

## Magic Header (9 bytes)

| 偏移 | 大小 | 值 | 说明 |
|------|------|-----|------|
| 0 | 4 | `\x89FQC` | 魔数签名（高位检测二进制传输损坏） |
| 4 | 2 | `\r\n` | DOS 行尾检测 |
| 6 | 1 | `\x1a` | Ctrl-Z（阻止 DOS `type` 命令输出） |
| 7 | 1 | `\n` | Unix 行尾检测 |
| 8 | 1 | `0x01` | 格式主版本号 |

设计参考 PNG 文件签名，可同时检测多种常见的文件传输损坏。

## Global Header

| 字段 | 类型 | 说明 |
|------|------|------|
| header_size | u32 | 头部总大小（字节） |
| flags | u64 | 位标志（见下表） |
| compression_algo | u8 | 保留 (0) |
| checksum_type | u8 | 1 = xxHash64 |
| reserved | u16 | 保留 (0) |
| total_read_count | u64 | 归档中读段总数 |
| filename_len | u16 | 原始文件名长度 |
| filename | [u8] | 原始文件名 (UTF-8) |
| timestamp | u64 | 创建时间 (Unix 时间戳) |

### Flags (u64)

| 位 | 名称 | 说明 |
|----|------|------|
| 0 | IS_PAIRED | 配对端数据 |
| 1 | HAS_REORDER_MAP | 归档包含重排映射 |
| 2-3 | QUALITY_MODE | 0=Lossless, 1=Illumina8Bin, 2=Discard |
| 4-5 | ID_MODE | 0=Exact, 1=StripComment, 2=Discard |
| 6-7 | LENGTH_CLASS | 0=Short, 1=Medium, 2=Long |
| 8-9 | PE_LAYOUT | 0=Interleaved, 1=Consecutive |

## Block Header

| 字段 | 类型 | 说明 |
|------|------|------|
| block_id | u32 | 顺序块标识符 |
| uncompressed_count | u32 | 块内读段数量 |
| uniform_read_length | u16 | 统一读段长度（变长为 0） |
| codec_seq | u8 | 序列编解码器 (0=ABC, 1=Zstd) |
| codec_qual | u8 | 质量编解码器 (0=SCM-O2, 1=SCM-O1, 2=Discard) |
| ids_compressed_size | u32 | 压缩后 ID 数据大小 |
| seq_compressed_size | u32 | 压缩后序列数据大小 |
| qual_compressed_size | u32 | 压缩后质量数据大小 |
| aux_size | u32 | 辅助数据大小 |

块头之后依次为 4 个压缩数据段：IDs、Sequences、Quality、Aux。

## Block Index

| 字段 | 类型 | 说明 |
|------|------|------|
| num_blocks | u32 | 块数量 |
| entries | [BlockIndexEntry] | 索引条目数组 |

每个 `BlockIndexEntry`：

| 字段 | 类型 | 说明 |
|------|------|------|
| offset | u64 | 块起始的文件偏移量 |
| archive_id_start | u64 | 该块中第一个读段 ID（归档顺序） |
| read_count | u32 | 块内读段数量 |

## File Footer (32 bytes)

| 偏移 | 大小 | 类型 | 说明 |
|------|------|------|------|
| 0 | 8 | u64 | Block Index 偏移量 |
| 8 | 8 | u64 | 所有块数据的 xxHash64 校验和 |
| 16 | 8 | u64 | 保留 |
| 24 | 4 | [u8;4] | 尾部魔数 `FQC\0` |
| 28 | 4 | u32 | Footer 大小 (32) |

## 字节序

所有多字节整数使用**小端序** (little-endian) 存储。

## 校验和

使用 xxHash64 进行数据完整性校验。Footer 中的校验和覆盖所有块数据。可通过 `fqc verify` 命令验证归档完整性。
