# fqc

[![License](https://img.shields.io/badge/license-GPL--3.0-green)](https://www.gnu.org/licenses/gpl-3.0.en.html)
[![CI](https://github.com/open-genomics/fq-compressor-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/open-genomics/fq-compressor-rust/actions/workflows/ci.yml)

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

> **同名二进制 `PATH` 覆盖风险**：两个实现都安装名为 `fqc` 的二进制，后者（或
> `PATH` 中更靠前的目录）会覆盖前者，请用 `which fqc` 确认实际调用的实现。

## 为什么用它

- **FASTQ 感知的归档格式**，而非通用的压缩数据块
- **块级元数据**，支持检视、校验与部分流式工作流
- **单一二进制 CLI**，提供 `compress`、`decompress`、`info`、`verify`
- **内存安全的 Rust 实现**，MSRV 固定为 **1.75.0**，除 Windows 内存探测（`GlobalMemoryStatusEx` FFI，见 `src/memory_budget.rs`）外零 `unsafe`

## 快速开始

需要 Rust 1.75.0+：

```bash
cargo build --release
./target/release/fqc compress -i tests/data/test_se.fastq -o sample.fqc
./target/release/fqc info -i sample.fqc
./target/release/fqc verify -i sample.fqc
./target/release/fqc decompress -i sample.fqc -o sample.fastq
```

安装到 PATH、双端输入、`--streaming`/`--pipeline` 模式、区间解压等见
[快速开始教程](docs/tutorials/quick-start.md)；全部命令与选项（含全局 `--memory-limit`）
见 [CLI 参考](docs/reference/cli.md)。

## 文档

技术文档按 Diátaxis 组织于 [docs/](docs/README.md)：

- [教程](docs/tutorials/) 与 [操作指南](docs/how-to/) —— 跑通与完成任务
- [参考](docs/reference/) —— CLI、格式规范、基准报告
- [解释](docs/explanation/) —— 白皮书、理论、竞品对比、架构与算法

## 开发

- AI 贡献指南：[`AGENTS.md`](AGENTS.md)
- 领域语言：[`CONTEXT.md`](CONTEXT.md)
- 变更历史：[`CHANGELOG.md`](CHANGELOG.md)
- 版本策略与发布流程：[`VERSIONING.md`](VERSIONING.md)
- 决策记录：`.agents/notes/`（`npm run verify-notes` 校验）

提交 PR 前的本地门禁与流程见 [`CONTRIBUTING.md`](CONTRIBUTING.md)；CI 在
GitHub Actions 跑同一套（`.github/workflows/ci.yml`）。

## 许可证

GPL-3.0-or-later，见 [LICENSE](LICENSE)。
