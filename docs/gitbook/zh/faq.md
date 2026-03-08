# 常见问题

## 一般问题

### fqc 是什么？

fqc 是一个用 Rust 编写的高性能 FASTQ 压缩器。它对短读段使用 ABC（Alignment-Based Compression）算法，对中长读段使用 Zstd 压缩。

### fqc 与 C++ 版本 (fq-compressor) 有什么区别？

两个版本共享相同的 `.fqc` 归档格式和核心算法（ABC、SCM）。Rust 版本使用 Rayon + crossbeam 替代 Intel TBB，并添加了异步 I/O。两个版本生成的归档文件可互相读取。

### fqc 支持哪些读段长度？

fqc 支持所有读段长度。短读段 (< 300bp) 通过 ABC 算法获得最佳压缩比。中读段 (300bp – 10kbp) 和长读段 (> 10kbp) 使用 Zstd 压缩。

## 压缩

### 如何获得最佳压缩比？

```bash
fqc compress -i reads.fastq -o reads.fqc -l 9 --lossy-quality illumina8 --block-size 50000
```

使用高压缩级别、有损质量量化和大块大小。

### 压缩是无损的吗？

默认是无损的。序列和质量值完全保留。使用 `--lossy-quality` 启用有损质量压缩以获得更高压缩比。

### fqc 能压缩 gzip 格式的 FASTQ 文件吗？

可以。fqc 透明解压 `.gz`、`.bz2`、`.xz` 和 `.zst` 输入文件。

### 什么是流式模式？

流式模式（`--streaming`）禁用全局读段排序，逐块处理读段。适用于 stdin/管道输入或内存有限的场景。压缩比会略低。

## 性能

### 如何加速压缩？

1. 对大文件使用 `--pipeline` 模式
2. 使用 `RUSTFLAGS="-C target-cpu=native"` 构建
3. 确保足够的线程 (`-t`)
4. 如果可以接受，使用有损质量模式

### 什么是 Pipeline 模式？

Pipeline 模式（`--pipeline`）启用 3 阶段 Reader→Compressor→Writer 流水线，实现 I/O 与计算的重叠执行。推荐用于 > 1GB 的文件。

### fqc 使用多少内存？

fqc 自动检测系统内存，默认使用约 75%。使用 `--memory-limit` 设置手动上限（单位：MB）。

## 兼容性

### 能解压 C++ 版本创建的文件吗？

可以。两个版本使用相同的 FQC 格式规范，归档文件可互相读取。

### 支持哪些平台？

- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)

## 故障排除

### bzip2/xz 编译失败

安装系统依赖：

```bash
# Debian/Ubuntu
sudo apt install libbz2-dev liblzma-dev pkg-config

# macOS
brew install xz
```

### 内存不足

减小块大小或设置内存限制：

```bash
fqc compress -i reads.fastq -o reads.fqc --block-size 1000 --memory-limit 2048
```
