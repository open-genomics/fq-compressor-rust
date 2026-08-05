# fqc

[![CI](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/LessUp/fq-compressor-rust?label=release)](https://github.com/LessUp/fq-compressor-rust/releases)
[![License](https://img.shields.io/badge/license-GPL--3.0-green)](https://www.gnu.org/licenses/gpl-3.0.en.html)
[![Docs](https://img.shields.io/badge/docs-live-4f46e5)](https://lessup.github.io/fq-compressor-rust/)

`fqc` 是一个用 Rust 编写的 FASTQ 压缩工具，围绕块索引的 `.fqc` 归档格式构建。
它将短读 ABC 路径、Zstd 支撑的中/长读压缩与质量分编码整合进单一 CLI，支持压缩、解压、检视与校验。

## 为什么用它

- **FASTQ 感知的归档格式**，而非通用的压缩数据块
- **块级元数据**，支持检视、校验与部分流式工作流
- **单一二进制 CLI**，提供 `compress`、`decompress`、`info`、`verify`
- **内存安全的 Rust 实现**，MSRV 固定为 **1.75.0**，零 `unsafe`

## 安装

从 [GitHub Releases](https://github.com/LessUp/fq-compressor-rust/releases) 下载预编译二进制，或本地构建：

```bash
cargo build --release
cargo install --path .
```

## 快速开始

```bash
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release

./target/release/fqc compress -i tests/data/test_se.fastq -o sample.fqc
./target/release/fqc info -i sample.fqc
./target/release/fqc verify -i sample.fqc
./target/release/fqc decompress -i sample.fqc -o sample.fastq
```

## 常用命令

```bash
fqc compress -i reads.fastq -o reads.fqc
fqc compress -i reads.fastq -o reads.fqc --memory-limit 0
fqc compress -i reads.fastq -o reads.fqc --pipeline
fqc compress -i reads.fastq -o reads.fqc --streaming
fqc compress -i reads.fastq -o reads.fqc --streaming --memory-limit 1024
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc

fqc decompress -i reads.fqc -o reads.fastq
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
fqc decompress -i reads.fqc -o reads.fastq --original-order
fqc decompress -i paired.fqc -o paired.fastq --split-pe

fqc info -i reads.fqc --detailed --show-codecs
fqc verify -i reads.fqc
fqc verify -i reads.fqc --quick
```

## 文档

- **项目站点：** <https://lessup.github.io/fq-compressor-rust/>
- **快速开始：** [docs/guide/quick-start.md](docs/guide/quick-start.md)
- **CLI 参考：** [docs/guide/cli.md](docs/guide/cli.md)
- **架构概述：** [docs/architecture/index.md](docs/architecture/index.md)
- **性能路线图：** [docs/architecture/performance-roadmap.md](docs/architecture/performance-roadmap.md)
- **算法：** [docs/algorithms/index.md](docs/algorithms/index.md)

`--memory-limit 0` 保留默认的自动内存选择行为。低内存场景建议使用 `--streaming`；pipeline 模式为分阶段执行路径，archive 模式仍会全量读入以做全局分析。

## 开发

本仓库使用 **OpenSpec** 作为规划与变更管理层。

- living specs：[`openspec/specs/`](openspec/specs/)
- 活跃变更目录：[`openspec/changes/`](openspec/changes/)
- AI 贡献指南：[`AGENTS.md`](AGENTS.md)

校验命令：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
npm run docs:build
```

启用本地 Git 钩子：

```bash
bash scripts/setup-hooks.sh
```
