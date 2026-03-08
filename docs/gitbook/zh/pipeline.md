# 并行流水线

## 概述

fqc 提供可选的 3 阶段流水线模式（`--pipeline`），通过 crossbeam 有界通道实现 I/O 与计算的重叠执行。

## 架构

```
┌────────┐  channel  ┌────────────┐  channel  ┌────────┐
│ Reader │──────────▶│ Compressor │──────────▶│ Writer │
│ (串行) │  bounded  │  (并行)    │  bounded  │ (串行) │
└────────┘           └────────────┘           └────────┘
```

### 阶段 1: Reader（串行）

- 顺序读取 FASTQ 输入
- 将读段按 block_size 切分为块
- 通过有界通道发送块

### 阶段 2: Compressor（并行）

- 接收 Reader 发送的块
- 使用 Rayon 线程池并行压缩各块
- 短读段用 ABC，中长读段用 Zstd
- 将压缩后的块发送给 Writer

### 阶段 3: Writer（串行）

- 接收压缩后的块
- 顺序写入输出 FQC 文件
- 使用 `AsyncWriter` 写后缓冲（4MB buffer, depth 4）
- 构建块索引并写入 footer

## 背压机制

有界通道确保内存可控：

- Compressor 太慢 → Reader 阻塞（通道满）
- Writer 太慢 → Compressor 阻塞（通道满）
- 通道容量可配置（默认：2× 线程数）

## 何时使用 Pipeline 模式

| 场景 | 默认模式 | Pipeline |
|------|---------|----------|
| 小文件 (< 100MB) | ✓ 更简单 | 开销不值得 |
| 大文件 (> 1GB) | 够用 | ✓ 吞吐更高 |
| NVMe/SSD 存储 | 够用 | ✓ I/O 重叠有帮助 |
| HDD 存储 | 够用 | ✓ I/O 重叠帮助更大 |
| 流式输入 | 不适用 | ✓ 天然适合 |

## 用法

```bash
# 压缩
fqc compress -i reads.fastq -o reads.fqc --pipeline

# 解压
fqc decompress -i reads.fqc -o reads.fastq --pipeline
```

## 实现

- **压缩流水线**: `src/pipeline/compression.rs`
- **解压流水线**: `src/pipeline/decompression.rs`
- **AsyncWriter**: `src/io/async_io.rs`
- **通道**: crossbeam-channel 有界通道
