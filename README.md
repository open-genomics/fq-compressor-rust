# fqc

[![License](https://img.shields.io/badge/license-GPL--3.0-green)](https://www.gnu.org/licenses/gpl-3.0.en.html)

`fqc` 是一个用 Rust 编写的 FASTQ 压缩工具，围绕块索引的 `.fqc` 归档格式构建。
它将短读 ABC 路径、Zstd 支撑的中/长读压缩与质量分编码整合进单一 CLI，支持压缩、解压、检视与校验。

> **格式族：`fqc-indexed/v2`**。`fqc` 与 `.fqc` 是两个**同名、不同格式族**的产品：
> 本仓库（Rust）与 [`fq-compressor`](https://github.com/open-genomics/fq-compressor)（C++）
> 各自实现自己的 `fqc` 二进制与 `.fqc` 归档，magic 不同、互不兼容、不能互相解码。

| 仓库 | 实现语言 | 格式族 ID | 完整 magic | 访问模型 |
|---|---|---|---|---|
| [open-genomics/fq-compressor-rust](https://github.com/open-genomics/fq-compressor-rust)（本仓库） | Rust | `fqc-indexed/v2` | `89 46 51 43 0D 0A 1A 0A` | 块索引归档；支持检视/校验/部分流式 |
| [open-genomics/fq-compressor](https://github.com/open-genomics/fq-compressor) | C++23 | `fqc-sequential/v2` | `46 51 43 56 32 0D 0A 1A`（`FQCV2\r\n\x1A`） | 顺序流式归档；不支持随机访问/按区间提取 |

扩展名 `.fqc` 不能判定格式：reader 必须检查 archive magic，两个实现以显式的
unsupported-format-family 错误拒绝对方的 magic，不能互相解码。

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

> **同名二进制 `PATH` 覆盖风险**：两个实现都安装名为 `fqc` 的二进制。若两者同时
> 进入 `PATH`，后安装者（或 `PATH` 中更靠前的目录）会覆盖另一个，请用 `which fqc`
> 确认实际调用的实现。

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
fqc compress -i reads.fastq -o reads.fqc --id-mode exact
fqc compress -i reads.fastq -o reads.fqc --lossy-quality qvz
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
`0` 表示自动有限预算（约可用内存的 75%，带硬性结构上限），**并非无限内存**。
archive 压缩在摄入时按该预算估计峰值，超限会在创建 `.fqc` 前失败并提示 `--streaming`。pipeline 仍是分段执行路径，不是严格低内存摄入：

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
