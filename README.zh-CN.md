# fqc - 高性能 FASTQ 压缩器

[![CI](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml)
[![Docs](https://img.shields.io/badge/Docs-GitBook-blue?logo=github)](https://lessup.github.io/fq-compressor-rust/)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![MSRV](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)

[English](README.md) | 简体中文 | [C++ 版本 (fq-compressor)](https://github.com/LessUp/fq-compressor)

> **fq-compressor** 的 Rust 实现，两个版本共享相同的 `.fqc` 归档格式与 ABC/SCM 压缩算法。
> Rust 版本以 Rayon + crossbeam 替代 Intel TBB，并引入异步 I/O。

一个用 Rust 编写的高性能 FASTQ 压缩器，采用 ABC（Alignment-Based Compression）算法处理短读段，Zstd 处理中长读段。

## ✨ 特性

| 类别 | 功能 |
|------|------|
| **压缩** | ABC（共识+增量）用于短读段，Zstd 用于长读段 |
| **质量值** | SCM（统计上下文模型）+ 算术编码 |
| **性能** | 并行处理、3 阶段流水线、异步 I/O |
| **灵活性** | 流式模式、有损/无损质量、随机访问 |
| **兼容性** | 配对端支持、压缩输入（gz/bz2/xz/zst）|

<details>
<summary><b>📋 完整特性列表</b></summary>

- **ABC 算法** — 短读段（< 300bp）基于共识序列的增量编码，高压缩比
- **Zstd 压缩** — 中/长读段使用长度前缀编码
- **SCM 质量压缩** — 统计上下文模型 + 算术编码，高效压缩质量分数
- **全局读段重排** — 基于 minimizer 的读段重排序，提升压缩比
- **随机访问** — 块索引归档格式，支持高效部分解压
- **并行处理** — 基于 Rayon 的并行块压缩/解压
- **流水线模式** — 3 阶段 Reader→Compressor→Writer 流水线，支持背压（`--pipeline`）
- **异步 I/O** — 后台预取与写入缓冲，提升吞吐
- **流式模式** — 从标准输入低内存压缩，无需全局重排（`--streaming`）
- **无损与有损** — 支持无损、Illumina 8-bin 分箱、丢弃质量分数三种模式
- **压缩输入** — 透明解压 `.gz`、`.bz2`、`.xz`、`.zst` 格式的 FASTQ 文件
- **配对端** — 支持交错（interleaved）与分文件配对端模式
- **内存预算** — 自动检测系统内存，动态分块处理大型数据集

</details>

## 📊 性能

| 模式 | 压缩速度 | 解压速度 | 压缩比 |
|------|---------|---------|--------|
| 默认 | ~10 MB/s | ~55 MB/s | 3.9x |
| 流水线 | ~12 MB/s | ~60 MB/s | 3.9x |

*测试环境：Intel Core i7-9700 @ 3.00GHz（8 核），2.27M 条 Illumina reads（511 MB 未压缩）*

### 压缩策略

| 读段长度 | 序列编码器 | 质量编码器 | 全局排序 |
|----------|-----------|-----------|---------|
| 短 (<300bp) | ABC（共识 + Delta） | SCM Order-2 | ✅ 是 |
| 中 (300bp-10kbp) | Zstd | SCM Order-2 | ❌ 否 |
| 长 (>10kbp) | Zstd | SCM Order-1 | ❌ 否 |

## 📦 安装

### 从源码构建

```bash
# 克隆并构建
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release

# 二进制文件位置
./target/release/fqc --help
```

### Docker

```bash
# 从 GitHub Container Registry 拉取
docker pull ghcr.io/lessup/fq-compressor-rust:latest

# 或本地构建
docker build -t fqc .

# 运行（挂载数据目录）
docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc
```

### 预编译二进制

从 [GitHub Releases](https://github.com/LessUp/fq-compressor-rust/releases) 下载：
- Linux（x64、ARM64）— glibc 和 musl（静态链接）
- macOS（Intel、Apple Silicon）
- Windows x64

## 🚀 快速开始

### 压缩

```bash
# 基本压缩（自动检测读段长度）
fqc compress -i reads.fastq -o reads.fqc

# 指定压缩级别（1-9）
fqc compress -i reads.fastq -o reads.fqc -l 9

# 流式模式（低内存，从 stdin）
cat reads.fastq | fqc compress --streaming -i - -o reads.fqc

# 流水线模式（3 阶段并行流水线）
fqc compress -i reads.fastq -o reads.fqc --pipeline

# 配对端（分离文件）
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc

# 配对端（交错单文件）
fqc compress -i interleaved.fastq -o paired.fqc --interleaved

# 压缩输入（自动检测）
fqc compress -i reads.fastq.gz -o reads.fqc
fqc compress -i reads.fastq.bz2 -o reads.fqc

# 丢弃质量分数（最小输出）
fqc compress -i reads.fastq -o reads.fqc --lossy-quality discard

# 强制长读段模式
fqc compress -i long_reads.fastq -o reads.fqc --long-read-mode long

# 覆盖已存在文件
fqc compress -i reads.fastq -o reads.fqc -f
```

### 解压

```bash
# 完整解压
fqc decompress -i reads.fqc -o reads.fastq

# 提取读段范围（1-based，包含边界）
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
fqc decompress -i reads.fqc -o subset.fastq --range 100:    # 从 100 到末尾

# 输出到 stdout
fqc decompress -i reads.fqc -o -

# 仅输出头部（ID）
fqc decompress -i reads.fqc -o headers.txt --header-only

# 恢复原始顺序（需要 reorder map）
fqc decompress -i reads.fqc -o reads.fastq --original-order

# 分离配对端到不同文件
fqc decompress -i paired.fqc -o output.fastq --split-pe
# 生成 output_R1.fastq 和 output_R2.fastq

# 流水线模式解压
fqc decompress -i reads.fqc -o reads.fastq --pipeline

# 跳过损坏块而非失败
fqc decompress -i reads.fqc -o reads.fastq --skip-corrupted
```

### 信息与验证

```bash
# 可读摘要
fqc info -i reads.fqc

# JSON 输出
fqc info -i reads.fqc --json

# 详细块索引
fqc info -i reads.fqc --detailed

# 显示每块编码器信息
fqc info -i reads.fqc --show-codecs

# 验证归档完整性
fqc verify -i reads.fqc

# 详细验证（逐块进度）
fqc verify -i reads.fqc --verbose

# 快速验证（仅头部 + 尾部）
fqc verify -i reads.fqc --quick
```

## 📁 FQC 文件格式

```
┌─────────────────────┐
│   Magic Header (9B) │  "\x89FQC\r\n\x1a\n" + 版本号
├─────────────────────┤
│   Global Header     │  标志位、读段数、文件名、时间戳
├─────────────────────┤
│   Block 0           │  块头 + ID + 序列 + 质量 + 辅助数据
├─────────────────────┤
│   Block 1           │
├─────────────────────┤
│   ...               │
├─────────────────────┤
│   Reorder Map (可选)│  正向 + 反向映射（delta + varint 编码）
├─────────────────────┤
│   Block Index       │  随机访问偏移量
├─────────────────────┤
│   File Footer (32B) │  索引偏移、校验和、魔数尾
└─────────────────────┘
```

完整规范见 [format-spec.md](docs/gitbook/zh/format-spec.md)。

## 🏗️ 项目结构

```
src/
├── main.rs              # CLI 入口（clap derive）、命令分发
├── lib.rs               # 库根，重导出所有模块
├── error.rs             # FqcError 枚举（11 变体）+ ExitCode 映射（0-5）
├── types.rs             # 核心类型：ReadRecord、QualityMode、IdMode、PeLayout
├── format.rs            # FQC 二进制格式：魔数、GlobalHeader、BlockHeader、Footer
├── fqc_reader.rs        # 归档读取器（块索引 + 随机访问）
├── fqc_writer.rs        # 归档写入器（块索引 + finalize）
├── reorder_map.rs       # 双向读段重排映射（ZigZag delta + varint）
├── algo/
│   ├── block_compressor.rs  # ABC 算法（共识 + delta）+ Zstd 编解码
│   ├── dna.rs               # DNA 编码表 + 反向互补
│   ├── global_analyzer.rs   # 基于 Minimizer 的全局读段重排
│   ├── quality_compressor.rs # SCM order-1/2 算术编码（质量值）
│   ├── id_compressor.rs     # ID 分词 + delta 编码
│   └── pe_optimizer.rs      # 配对端互补优化
├── commands/
│   ├── compress.rs      # CompressCommand：默认/流式/流水线模式
│   ├── decompress.rs    # DecompressCommand：顺序/并行/重排
│   ├── info.rs          # 归档信息显示（文本/JSON/详细）
│   └── verify.rs        # 逐块完整性验证
├── common/
│   └── memory_budget.rs # 系统内存检测（Win/Linux/macOS）
├── fastq/
│   └── parser.rs        # FASTQ 解析器（验证、统计、配对端）
├── io/
│   ├── async_io.rs      # AsyncReader/AsyncWriter 缓冲池
│   └── compressed_stream.rs # 透明 gz/bz2/xz/zst 解压
└── pipeline/
    ├── mod.rs           # PipelineControl、PipelineStats、ReadChunk
    ├── compression.rs   # 3 阶段 Reader→Compressor→Writer（crossbeam）
    └── decompression.rs # 3 阶段 Reader→Decompressor→Writer
```

## 🧪 测试

```bash
# 运行全部 131 个测试
cargo test --lib --tests

# 运行特定测试套件
cargo test --test test_algo         # 19 个算法测试
cargo test --test test_dna          # 15 个 DNA 工具测试
cargo test --test test_e2e          # 15 个端到端测试
cargo test --test test_format       # 15 个格式测试
cargo test --test test_parser       # 19 个解析器测试
cargo test --test test_reorder_map  # 23 个重排映射测试
cargo test --test test_roundtrip    # 14 个往返测试
cargo test --test test_types        # 11 个类型测试

# 代码检查
cargo clippy --all-targets          # 必须通过，0 警告
cargo fmt --all -- --check          # 必须通过
```

## 📚 文档

- **GitBook**: [https://lessup.github.io/fq-compressor-rust/](https://lessup.github.io/fq-compressor-rust/)
  - [English](docs/gitbook/en/README.md) | [中文](docs/gitbook/zh/README.md)
- **命令行参考**: [docs/gitbook/zh/cli-reference.md](docs/gitbook/zh/cli-reference.md)
- **架构设计**: [docs/gitbook/zh/architecture.md](docs/gitbook/zh/architecture.md)
- **算法详解**: [docs/gitbook/zh/algorithms.md](docs/gitbook/zh/algorithms.md)

## 🤝 贡献

欢迎贡献！请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解指南。

- [行为准则](CODE_OF_CONDUCT.md)
- [开发指南](CONTRIBUTING.md#development-setup)
- [Pull Request 流程](CONTRIBUTING.md#pull-request-process)

## 📄 许可证

本项目采用 GNU General Public License v3.0 许可证 — 详情见 [LICENSE](LICENSE) 文件。

## 🔗 相关项目

- [fq-compressor](https://github.com/LessUp/fq-compressor) — 原始 C++ 实现
- [Spring](https://github.com/shubhamchandak94/Spring) — ABC 算法参考论文
