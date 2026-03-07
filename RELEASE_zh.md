# fqc 发布说明

> **🔗 C++ 原版**: 本项目是 [fq-compressor（C++ 实现）](../fq-compressor/README.zh-CN.md) 的 Rust 重写版本。C++ 版本采用 Intel TBB 并行和 C++23，详见 [英文 README](../fq-compressor/README.md)。

## [0.1.0] - 2026-03-07

**fqc 首次发布** — 高性能 FASTQ 压缩工具（Rust 实现）。

### 功能特性

- **ABC 压缩算法** — 基于共识序列的增量编码，适用于短读段（< 300bp）
- **Zstd 压缩** — 中/长读段使用 Zstd 压缩，带长度前缀编码
- **SCM 质量压缩** — 统计上下文模型 + 算术编码，高效压缩质量分数
- **全局读段重排序** — 基于 minimizer 的读段重排序，提升压缩比
- **随机访问** — 块索引归档格式，支持高效部分解压
- **并行处理** — 基于 Rayon 的并行块压缩/解压
- **管线模式** — 3 阶段 Reader→Compressor→Writer 管线，支持反压（`--pipeline`）
- **异步 I/O** — 后台预取与写入缓冲，提升吞吐
- **流式模式** — 从标准输入低内存压缩，无需全局重排
- **无损与有损** — 支持无损、Illumina 8-bin 分箱、丢弃质量分数三种模式
- **压缩输入** — 透明解压 `.gz`、`.bz2`、`.xz`、`.zst` 格式的 FASTQ 文件
- **双端测序** — 支持交错（interleaved）与分文件双端模式
- **内存预算** — 自动检测系统内存，动态分块处理大型数据集
- **退出码映射** — 所有 CLI 命令统一退出码（0-5）

### 测试

- 97 个测试，覆盖 6 个测试套件
- 端到端往返测试、格式测试、解析器测试、重排序映射测试、类型测试

### 安装

#### 从源码构建

```bash
cargo build --release
```

生成的二进制文件位于 `target/release/fqc`（Windows 上为 `fqc.exe`）。

#### Docker

```bash
# 从 GitHub Container Registry 拉取
docker pull ghcr.io/lessup/fq-compressor-rust:latest

# 或本地构建
docker build -t fqc .

# 运行（挂载数据目录）
docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc
docker run --rm -v $(pwd):/data fqc decompress -i /data/reads.fqc -o /data/reads.fastq
```

### 快速开始

```bash
# 压缩
fqc compress -i reads.fastq -o reads.fqc

# 解压
fqc decompress -i reads.fqc -o reads.fastq

# 查看归档信息
fqc info -i reads.fqc

# 验证完整性
fqc verify -i reads.fqc
```

### 平台支持

| 平台 | 文件 | 说明 |
|---|---|---|
| Linux x64 | `fqc-v0.1.0-x86_64-unknown-linux-gnu.tar.gz` | 动态链接 (glibc) |
| Linux x64 (静态) | `fqc-v0.1.0-x86_64-unknown-linux-musl.tar.gz` | **静态链接，任意 Linux 可运行** |
| Linux arm64 | `fqc-v0.1.0-aarch64-unknown-linux-gnu.tar.gz` | 动态链接 (glibc) |
| Linux arm64 (静态) | `fqc-v0.1.0-aarch64-unknown-linux-musl.tar.gz` | **静态链接，任意 Linux 可运行** |
| Windows x64 | `fqc-v0.1.0-x86_64-pc-windows-msvc.zip` | |
| macOS x64 | `fqc-v0.1.0-x86_64-apple-darwin.tar.gz` | Intel Mac |
| macOS arm64 | `fqc-v0.1.0-aarch64-apple-darwin.tar.gz` | Apple Silicon |

### 校验文件完整性

```bash
sha256sum -c checksums-sha256.txt
```
