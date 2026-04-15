# fqc (fq-compressor-rust)

[![CI](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

**fqc** 是一个用 Rust 编写的高性能 FASTQ 压缩器，采用 ABC（Alignment-Based Compression）算法处理短读段，Zstd 处理中长读段。

> 这是 [fq-compressor](https://lessup.github.io/fq-compressor/) 的 Rust 实现。两个版本共享相同的 `.fqc` 归档格式与 ABC/SCM 压缩算法。Rust 版本以 Rayon + crossbeam 替代 Intel TBB，并引入异步 I/O。

## 核心特性

| 特性 | 说明 |
|------|------|
| **ABC 算法** | 共识 + Delta 编码，用于短读段 (< 300bp) |
| **Zstd 压缩** | 长度前缀编码，用于中/长读段 |
| **SCM 质量压缩** | 统计上下文模型 + 算术编码 |
| **全局重排** | 基于 Minimizer 的读段排序，提升压缩比 |
| **随机访问** | 块索引归档格式，支持部分解压 |
| **流水线模式** | 3 阶段 Reader→Compressor→Writer，带背压 |
| **异步 I/O** | 后台预读与写后缓冲 |
| **压缩输入** | 透明处理 `.gz`、`.bz2`、`.xz`、`.zst` 文件 |
| **配对端** | 支持交错和独立文件的配对端数据 |
| **内存预算** | 自动检测系统内存，动态分块 |

## 性能概览

| 模式 | 压缩速度 | 解压速度 | 压缩比 |
|------|---------|---------|--------|
| 默认 | ~10 MB/s | ~55 MB/s | 3.9x |
| 流水线 | ~12 MB/s | ~60 MB/s | 3.9x |

*测试环境：Intel Core i7-9700 @ 3.00GHz（8 核），2.27M 条 Illumina reads（511 MB 未压缩）*

## 快速开始

```bash
# 从源码安装
cargo build --release

# 压缩
fqc compress -i reads.fastq -o reads.fqc

# 解压
fqc decompress -i reads.fqc -o reads.fastq

# 查看归档信息
fqc info -i reads.fqc

# 验证完整性
fqc verify -i reads.fqc
```

## 文档目录

- [安装指南](installation.md) — 从源码构建或 Docker
- [快速开始](quickstart.md) — 压缩你的第一个 FASTQ 文件
- [命令行参考](cli-reference.md) — 所有命令和选项
- [架构设计](architecture.md) — 了解内部工作原理
  - [核心算法](algorithms.md) — ABC、SCM、ID 压缩
  - [FQC 文件格式](format-spec.md) — 二进制格式规范
  - [并行流水线](pipeline.md) — 3 阶段流水线设计
- [性能调优](performance.md) — 针对工作负载优化
- [开发指南](development.md) — 贡献代码
- [常见问题](faq.md) — 常见问题解答

## 许可证

GNU General Public License v3.0 — 见 [LICENSE](https://github.com/LessUp/fq-compressor-rust/blob/main/LICENSE)。
