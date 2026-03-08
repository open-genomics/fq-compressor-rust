# 架构设计

## 概述

fqc 是一个高性能 FASTQ 压缩器，采用分层模块化架构。核心设计围绕**块级压缩**展开：数据被分割为固定大小的块，每个块独立压缩，支持随机访问。

## 数据流

### 压缩流程

```
FASTQ 输入
    │
    ▼
┌─────────────┐     ┌──────────────────┐
│ FASTQ Parser │────▶│ Global Analyzer  │  (可选) Minimizer 排序
│  fastq/      │     │  global_analyzer │  生成 ReorderMap
└─────────────┘     └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │  Block Partition  │  按 block_size 切分
                    └────────┬─────────┘
                             │ (并行)
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐
        │ Block 0  │  │ Block 1  │  │ Block N  │
        │ compress │  │ compress │  │ compress │
        └────┬─────┘  └────┬─────┘  └────┬─────┘
             │              │              │
             ▼              ▼              ▼
        ┌─────────────────────────────────────┐
        │           FQC Writer                │  写入 header + blocks
        │  → Block Index + Footer + Checksum  │  + index + footer
        └─────────────────────────────────────┘
```

### 解压流程

```
FQC 文件
    │
    ▼
┌──────────────┐
│  FQC Reader  │  读取 header + block index
└──────┬───────┘
       │ (随机访问 or 顺序)
       ▼
┌──────────────┐     ┌─────────────────┐
│ Block Decomp │────▶│ Reorder Restore │  (可选) 恢复原始顺序
└──────────────┘     └────────┬────────┘
                              ▼
                        FASTQ 输出
```

### Pipeline 模式

Pipeline 模式使用 3 阶段流水线，通过 crossbeam 有界通道实现背压：

```
┌────────┐  channel  ┌────────────┐  channel  ┌────────┐
│ Reader │──────────▶│ Compressor │──────────▶│ Writer │
│ (串行) │  bounded  │  (并行)    │  bounded  │ (串行) │
└────────┘           └────────────┘           └────────┘
```

## 模块结构

```
src/
├── main.rs                    # CLI 入口 (clap)
├── lib.rs                     # 库入口 (pub mod 导出)
│
├── algo/                      # 压缩算法
│   ├── block_compressor.rs    # 块压缩/解压 (ABC + Zstd 双路径)
│   ├── dna.rs                 # 共享 DNA 编码表 + 反向互补
│   ├── global_analyzer.rs     # 全局读段分析 + minimizer 排序
│   ├── id_compressor.rs       # 读段 ID 压缩 (Exact/StripComment/Discard)
│   ├── pe_optimizer.rs        # 配对端反向互补优化
│   └── quality_compressor.rs  # 质量分数 SCM 算术编码
│
├── commands/                  # CLI 子命令实现
│   ├── compress.rs            # compress (默认/流式/pipeline)
│   ├── decompress.rs          # decompress (顺序/并行/reorder)
│   ├── info.rs                # info (归档信息展示)
│   └── verify.rs              # verify (完整性校验)
│
├── common/
│   └── memory_budget.rs       # 系统内存检测 + 动态分块策略
│
├── fastq/
│   └── parser.rs              # FASTQ 解析器 (SE/PE/交错/采样/验证)
│
├── io/
│   ├── async_io.rs            # 异步 I/O (预读/写后缓冲)
│   └── compressed_stream.rs   # 透明解压流 (.gz/.bz2/.xz/.zst)
│
├── pipeline/
│   ├── compression.rs         # 3 阶段压缩流水线
│   └── decompression.rs       # 3 阶段解压流水线
│
├── error.rs                   # FqcError 枚举 + ExitCode 映射 (0-5)
├── format.rs                  # FQC 二进制格式结构体 (header/block/footer)
├── fqc_reader.rs              # FQC 归档读取器 (随机访问)
├── fqc_writer.rs              # FQC 归档写入器 (块索引)
├── reorder_map.rs             # 双向重排映射 (ZigZag varint)
└── types.rs                   # 核心类型与常量
```

## 关键设计决策

1. **块独立性** — 每个块可独立压缩/解压，支持随机访问和并行处理
2. **编解码器分离** — 序列/质量/ID 使用独立的编解码器和压缩流
3. **双路径策略** — 短读段用 ABC（高压缩比），中长读段用 Zstd（通用）
4. **背压流水线** — 有界通道防止内存溢出，适应不同 I/O 速度
5. **unsafe deny** — 全局禁止 unsafe 代码（仅 Windows FFI 例外）
