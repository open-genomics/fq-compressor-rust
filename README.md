# fqc

[![License](https://img.shields.io/badge/license-GPL--3.0-green)](https://www.gnu.org/licenses/gpl-3.0.en.html)

`fqc` 是一个用 Rust 编写的 FASTQ 压缩工具，围绕块索引的 `.fqc` 归档格式构建。
它将短读 ABC 路径、Zstd 支撑的中/长读压缩与质量分编码整合进单一 CLI，支持压缩、解压、检视与校验。

> **Format family**: `fqc-indexed/v2` — command `fqc`, extension `.fqc`,
> magic `89 46 51 43 0D 0A 1A 0A`. Distinct from C++ `fqc-sequential/v2`
> (magic `46 51 43 56 32 0D 0A 1A`); the other family's magic is rejected with an
> explicit unsupported-format-family error. Extension alone cannot select a decoder.

## 为什么用它

- **FASTQ 感知的归档格式**，而非通用的压缩数据块
- **块级元数据**，支持检视、校验与部分流式工作流
- **单一二进制 CLI**，提供 `compress`、`decompress`、`info`、`verify`
- **内存安全的 Rust 实现**，MSRV 固定为 **1.75.0**，零 `unsafe`

## 安装

从源码构建（需要 Rust 1.75.0+）：

```bash
git clone https://github.com/open-genomics/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release
# 或安装到 ~/.cargo/bin
cargo install --path .
```

## 快速开始

```bash
./target/release/fqc compress -i tests/data/test_se.fastq -o sample.fqc
./target/release/fqc info -i sample.fqc
./target/release/fqc verify -i sample.fqc
./target/release/fqc decompress -i sample.fqc -o sample.fastq
```

## 常用命令

```bash
fqc compress -i reads.fastq -o reads.fqc
fqc compress -i reads.fastq -o reads.fqc --pipeline
fqc compress -i reads.fastq -o reads.fqc --streaming
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc

fqc decompress -i reads.fqc -o reads.fastq
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
fqc decompress -i reads.fqc -o reads.fastq --original-order
fqc decompress -i paired.fqc -o paired.fastq --split-pe

fqc info -i reads.fqc --detailed --show-codecs
fqc verify -i reads.fqc
fqc verify -i reads.fqc --quick
```

`--memory-limit` 是全局参数（必须位于子命令之前），对 `compress` / `decompress` / `verify` 均生效。
`0` 表示自动选择有限预算（约可用内存的 75%，并受硬性结构上限约束），**并非无限内存**。
低内存压缩建议使用 `--streaming`；pipeline 模式为分阶段执行路径，archive 模式仍会全量读入以做全局分析：

```bash
fqc --memory-limit 1024 compress -i reads.fastq -o reads.fqc --streaming
fqc --memory-limit 512 decompress -i reads.fqc -o reads.fastq
fqc --memory-limit 512 verify -i reads.fqc
```

## 文档

技术文档位于 [docs/](docs/README.md)（纯 Markdown）：

- [技术白皮书](docs/whitepaper.md)与[理论基础](docs/theory.md)
- [快速开始](docs/guide/quick-start.md)与 [CLI 参考](docs/guide/cli.md)
- [架构总览](docs/architecture/index.md)与[决策记录](docs/architecture/decisions/index.md)
- [算法与 ABC 详解](docs/algorithms/index.md)
- [.fqc 格式规范](docs/reference/format-spec.md)
- [基准测试报告](docs/benchmarks/performance-report.md)

## 开发

- AI 贡献指南：[`AGENTS.md`](AGENTS.md)
- 领域语言：[`CONTEXT.md`](CONTEXT.md)
- 变更历史：[`CHANGELOG.md`](CHANGELOG.md)

校验命令：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
```

## 许可证

GPL-3.0-or-later，见 [LICENSE](LICENSE)。
