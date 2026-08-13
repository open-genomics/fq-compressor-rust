# 二进制格式规范

本文档定义 `.fqc` 归档格式的二进制结构，供实现者和工具开发者参考。

本规范描述 `fqc-indexed/v2` 格式族。`fqc` 产品名和 `.fqc` 后缀由
C++ 实现 (`fqc-sequential/v2`) 和 Rust 实现 (`fqc-indexed/v2`) 共享，
两者通过 archive magic 区分，不通过文件后缀区分。

## 文件布局概览

```mermaid
flowchart TD
    subgraph 文件结构
        A["Magic Header<br/>(9 bytes)"]
        B["Global Header<br/>(变长, 最小34 bytes)"]
        C["Block 0"]
        D["Block 1"]
        E["..."]
        F["Block N"]
        G["Reorder Map<br/>(可选, 变长)"]
        H["Block Index<br/>(变长)"]
        I["File Footer<br/>(32 bytes)"]
    end
    
    A --> B --> C --> D --> E --> F --> G --> H --> I
    
```

## Magic Header

文件以固定魔数开头，用于快速识别文件类型和格式族。

### 字段布局

| 偏移 | 大小 | 字段 | 值 | 描述 |
|------|------|------|-----|------|
| 0 | 8 | Magic Bytes | `0x89 46 51 43 0D 0A 1A 0A` | indexed 格式族标识 |
| 8 | 1 | Version | `0x20` (v2.0) | 格式版本号 |

### 格式族标识

indexed 格式族的 magic 为 `89 46 51 43 0D 0A 1A 0A`（PNG 风格签名）。
C++ sequential 格式族使用不同的 magic (`46 51 43 56 32 0D 0A 1A`)。
两个格式的 reader 必须通过完整 magic 区分格式族，遇到对方 magic 时返回
明确的 unsupported format family 错误，不尝试互相解码。

### 版本编码

版本号以单个字节编码：

```
Version = (Major << 4) | Minor

v2.0 = (2 << 4) | 0 = 0x20
```

## Global Header

全局头包含归档的元数据信息。

### 字段布局

| 偏移 | 大小 | 字段 | 类型 | 描述 |
|------|------|------|------|------|
| 0 | 4 | header_size | u32 LE | 头部总大小 |
| 4 | 8 | flags | u64 LE | 模式标志位 |
| 12 | 1 | compression_algo | u8 | 压缩算法 |
| 13 | 1 | checksum_type | u8 | 校验和类型 |
| 14 | 2 | reserved | u16 LE | 保留字段 (必须为 0) |
| 16 | 8 | total_read_count | u64 LE | 总读段数 |
| 24 | 2 | filename_len | u16 LE | 文件名长度 |
| 26 | N | original_filename | String | 原始文件名 |
| 26+N | 8 | timestamp | u64 LE | 时间戳 |

**最小大小**: 34 bytes

### 标志位定义

| 标志 | 位位置 | 描述 |
|------|--------|------|
| `IS_PAIRED` | 0 | 是否双端测序 |
| `PRESERVE_ORDER` | 1 | 是否保留原始顺序 |
| (未使用) | 2 | 保留，必须为 0 |
| `QUALITY_MODE` | 3-4 | 质量模式 (0-3) |
| `ID_MODE` | 5-6 | ID 模式 (0-3) |
| `HAS_REORDER_MAP` | 7 | 是否包含重排映射 |
| `PE_LAYOUT` | 8-9 | 双端布局 (0-3) |
| `READ_LENGTH_CLASS` | 10-11 | 读长类别 (0-3) |
| `STREAMING_MODE` | 12 | 流式模式标志 |

## Block 结构

每个 Block 包含一个 Block Header 和多个压缩流。

```mermaid
flowchart TD
    subgraph Block结构
        A["Block Header<br/>(104 bytes)"]
        B["Stream: IDs"]
        C["Stream: Seq"]
        D["Stream: Qual"]
        E["Stream: Aux"]
    end
    
    A --> B --> C --> D --> E
    
```

### Block Header

**固定大小**: 104 bytes

| 偏移 | 大小 | 字段 | 类型 | 描述 |
|------|------|------|------|------|
| 0 | 4 | header_size | u32 LE | 头部大小 (104) |
| 4 | 4 | block_id | u32 LE | 块标识符 |
| 8 | 1 | checksum_type | u8 | 校验和类型 |
| 9 | 1 | codec_ids | u8 | ID 流编解码器 |
| 10 | 1 | codec_seq | u8 | 序列流编解码器 |
| 11 | 1 | codec_qual | u8 | 质量流编解码器 |
| 12 | 1 | codec_aux | u8 | 辅助流编解码器 |
| 13 | 1 | reserved1 | u8 | 保留 (必须为 0) |
| 14 | 2 | reserved2 | u16 LE | 保留 (必须为 0) |
| 16 | 8 | block_xxhash64 | u64 LE | 块校验和 |
| 24 | 4 | uncompressed_count | u32 LE | 未压缩读段数 |
| 28 | 4 | uniform_read_length | u32 LE | 统一读长 (0表示可变) |
| 32 | 8 | compressed_size | u64 LE | 压缩后总大小 |
| 40 | 8 | offset_ids | u64 LE | ID 流偏移 |
| 48 | 8 | offset_seq | u64 LE | 序列流偏移 |
| 56 | 8 | offset_qual | u64 LE | 质量流偏移 |
| 64 | 8 | offset_aux | u64 LE | 辅助流偏移 |
| 72 | 8 | size_ids | u64 LE | ID 流大小 |
| 80 | 8 | size_seq | u64 LE | 序列流大小 |
| 88 | 8 | size_qual | u64 LE | 质量流大小 |
| 96 | 8 | size_aux | u64 LE | 辅助流大小 |

### Stream 类型

| 流类型 | 描述 | 可能使用的编解码器族 |
|--------|------|---------------------|
| IDs | 读段标识符 | Raw / DeltaVarint / DeltaZstd |
| Seq | 序列数据 | AbcV1 / ZstdPlain |
| Qual | 质量分数 | Raw / ScmV1 / ScmOrder1 |
| Aux | 辅助数据 | Raw / DeltaVarint |

## Codec 编码

编解码器以单字节标识，采用 `(family << 4) | version` 编码：

```
codec_byte = (family_nibble << 4) | (version_nibble & 0x0F)
```

高 4 位是编解码器族，低 4 位是版本。读取时先取高 nibble 确定族。

### 编解码器族表

| 族值 (高 nibble) | 族名称 | 描述 | 当前 version 0 的字节值 |
|-------------------|--------|------|------------------------|
| 0x0 | Raw | 未压缩原始数据 | `0x00` |
| 0x1 | AbcV1 | 锚基压缩 (短读段) | `0x10` |
| 0x2 | ScmV1 | SCM 质量压缩 (Order2) | `0x20` |
| 0x3 | DeltaLzma | Delta + LZMA 压缩 | `0x30` |
| 0x4 | DeltaZstd | Delta + Zstd 压缩 | `0x40` |
| 0x5 | DeltaVarint | Delta + Varint 编码 | `0x50` |
| 0x6 | OverlapV1 | 重叠压缩 | `0x60` |
| 0x7 | ZstdPlain | Zstd 直接压缩 | `0x70` |
| 0x8 | ScmOrder1 | SCM 质量压缩 (Order1) | `0x80` |
| 0xE | External | 外部编解码器 | `0xE0` |
| 0xF | Reserved | 保留 | `0xF0` |

### 族与流的映射

| 流类型 | 当前使用的族 | 场景 |
|--------|-------------|------|
| IDs | Raw, DeltaVarint, DeltaZstd | 根据 IdMode 选择 |
| Seq | AbcV1 (短读段), ZstdPlain (中/长读段) | 根据 ReadLengthClass 选择 |
| Qual | Raw (Discard), ScmV1 (Lossless Order2), ScmOrder1 (Lossless Order1) | 根据 QualityMode 选择 |
| Aux | Raw, DeltaVarint | 辅助元数据 |

读取器遇到未知族时返回 unsupported-codec 错误，不回退到默认编解码器。

## Reorder Map

重排映射存储读段重排信息，用于恢复原始顺序。

### Header (32 bytes)

| 偏移 | 大小 | 字段 | 类型 | 描述 |
|------|------|------|------|------|
| 0 | 4 | header_size | u32 LE | 头部大小 |
| 4 | 4 | version | u32 LE | 版本号 |
| 8 | 8 | total_reads | u64 LE | 总读段数 |
| 16 | 8 | forward_map_size | u64 LE | 正向映射大小 |
| 24 | 8 | reverse_map_size | u64 LE | 反向映射大小 |

## Block Index

块索引用于快速定位和随机访问。

### Header (16 bytes)

| 偏移 | 大小 | 字段 | 类型 | 描述 |
|------|------|------|------|------|
| 0 | 4 | header_size | u32 LE | 头部大小 |
| 4 | 4 | entry_size | u32 LE | 条目大小 |
| 8 | 8 | num_blocks | u64 LE | 块数量 |

### Index Entry (28 bytes each)

| 偏移 | 大小 | 字段 | 类型 | 描述 |
|------|------|------|------|------|
| 0 | 8 | offset | u64 LE | 块在文件中的偏移 |
| 8 | 8 | compressed_size | u64 LE | 压缩后大小 |
| 16 | 8 | archive_id_start | u64 LE | 起始读段ID |
| 24 | 4 | read_count | u32 LE | 读段数量 |

## File Footer

文件尾固定 32 bytes，位于文件末尾。

| 偏移 | 大小 | 字段 | 类型 | 描述 |
|------|------|------|------|------|
| 0 | 8 | index_offset | u64 LE | 块索引偏移 |
| 8 | 8 | reorder_map_offset | u64 LE | 重排映射偏移 (0表示无) |
| 16 | 8 | global_checksum | u64 LE | 全局校验和 |
| 24 | 8 | magic_end | [u8; 8] | 结束魔数 `FQC_EOF\0` |

### 结束魔数

```
MAGIC_END = [b'F', b'Q', b'C', b'_', b'E', b'O', b'F', 0x00]
```

## 版本兼容性

### 兼容性规则

- **主版本精确匹配**: 读取器只接受主版本号等于 2 的归档。
  不存在 v1 回退解析路径。
- **前向兼容**: 主版本号不等于 2 的归档被立即拒绝，返回
  `UnsupportedVersion` 错误。
- **次版本容忍**: 同一主版本下的不同次版本被接受（当前仅 v2.0）。
- **扩展容忍**: 所有头部字段预留扩展空间，读取器跳过声明的 header_size
  超出已知字段的额外字节。

### 版本历史

| 版本 | 变更描述 |
|------|----------|
| v2.0 | 当前格式，块索引架构 |

不存在 v1.x 格式。历史文档中提到的 v1 fallback 从未实现。

## 校验和类型

| 值 | 类型 | 描述 |
|----|------|------|
| 0 | XxHash64 | 64位 xxHash (seed=0) |
| 1-255 | Reserved | 保留 |

校验和类型 `0` 表示 XxHash64，不表示"无校验和"。GlobalHeader 和
BlockHeader 中的 checksum_type 字段当前始终为 `0`。

## 实现注意事项

1. **字节序**: 所有多字节字段使用小端序 (Little-Endian)
2. **对齐**: 结构体未对齐，按紧凑布局读取
3. **字符串**: 文件名以 UTF-8 编码，长度前缀存储
4. **保留字段**: 所有保留字段必须为零，读取时应验证
5. **编解码器标识**: 使用 `(family << 4) | version` 编码，不是平面枚举

## 示例：最小有效归档

```
Offset  Size  Content
------  ----  -------
0       9     Magic Header (0x89 FQ C 0D 0A 1A 0A 0x20)
9       34    Global Header (最小配置)
43      104   Block Header
147     N     Compressed Block Data
147+N   32    File Footer
```

总最小大小: 179 + N bytes

## 冻结测试 fixture

`tests/fixtures/indexed-v2/` 包含一个由审计基线生成的冻结 archive
(`frozen.fqc`) 和原始输入 (`input.fastq`)，以及记录生成命令和哈希值的
`MANIFEST.md`。该 fixture 用于保护 decoder 兼容性：未来 reader 必须能
继续解压该 archive 并得到与原始输入完全一致的输出。
