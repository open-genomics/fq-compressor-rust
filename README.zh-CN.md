# fqc - 高性能 FASTQ 压缩器

[English](README.md) | 简体中文 | [C++ 版本 (fq-compressor)](https://github.com/LessUp/fq-compressor)

> **fq-compressor** 的 Rust 实现，两个版本共享相同的 `.fqc` 归档格式与 ABC/SCM 压缩算法。
> Rust 版本以 Rayon + crossbeam 替代 Intel TBB，并引入异步 I/O。

## 特性

- **ABC 算法** — 短读长 (<300bp) 基于共识的 delta 编码，高压缩比
- **Zstd 压缩** — 中/长读长使用长度前缀编码
- **SCM 质量值压缩** — 统计上下文模型 + 算术编码
- **全局读长重排** — 基于 minimizer 的读长重排以提升压缩率
- **随机访问** — 块索引归档格式，支持高效部分解压
- **并行处理** — 基于 Rayon 的并行块压缩/解压
- **流水线模式** — 3 阶段 Reader→Compressor→Writer 流水线 (`--pipeline`)
- **异步 I/O** — 后台预取和写入缓冲
- **流式模式** — 低内存 stdin 压缩，无需全局重排
- **压缩输入** — 透明解压 `.gz`、`.bz2`、`.xz`、`.zst` FASTQ 文件
- **配对端** — 支持交错和分离文件的配对端模式

## 安装

```bash
cargo build --release
```

二进制文件位于 `target/release/fqc`。

### Docker

```bash
docker pull ghcr.io/lessup/fq-compressor-rust:latest
docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc
```

## 基本使用

```bash
# 压缩
fqc compress -i reads.fastq -o reads.fqc

# 解压
fqc decompress -i reads.fqc -o reads.fastq

# 查看归档信息
fqc info -i reads.fqc

# 校验完整性
fqc verify -i reads.fqc

# 流水线模式
fqc compress -i reads.fastq -o reads.fqc --pipeline

# 配对端（分离文件）
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o reads.fqc

# 范围解压
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
```

## 压缩策略

| 读长长度 | 序列编码 | 质量值编码 | 重排 |
|----------|----------|-----------|------|
| 短 (<300bp) | ABC (共识 + delta) | SCM Order-2 | 是 |
| 中 (300bp-10kbp) | Zstd | SCM Order-2 | 否 |
| 长 (>10kbp) | Zstd | SCM Order-1 | 否 |

## 测试

```bash
cargo test                          # 全部 131 个测试
cargo test --test test_e2e          # 15 端到端测试
cargo test --test test_roundtrip    # 14 往返压缩测试
```

## 项目结构

```
src/
├── algo/                   # 压缩算法
│   ├── block_compressor.rs # ABC + Zstd 块压缩/解压
│   ├── dna.rs              # DNA 编码表 + 反向互补
│   ├── global_analyzer.rs  # Minimizer 读长重排
│   └── quality_compressor.rs # SCM 算术编码
├── commands/               # CLI 命令实现
├── pipeline/               # 3 阶段压缩/解压流水线
├── io/                     # 异步 I/O + 压缩流
├── format.rs               # FQC 二进制格式结构
├── fqc_reader.rs           # 归档读取器（随机访问）
├── fqc_writer.rs           # 归档写入器（块索引）
└── error.rs                # FqcError 枚举 + 退出码映射
```

## 许可证

见 LICENSE 文件。
