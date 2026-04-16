# 性能调优

> 另见：[核心算法](algorithms.md)、[并行流水线](pipeline.md)

## 构建优化

### Release 构建

```bash
cargo build --release
```

`Cargo.toml` 中的 release profile 配置：

| 选项 | 值 | 说明 |
|------|-----|------|
| `opt-level` | 3 | 最大优化 |
| `lto` | "fat" | 全量链接时优化 |
| `codegen-units` | 1 | 单 codegen unit，更好的优化 |
| `panic` | "abort" | 更小二进制，无 unwind 开销 |
| `strip` | "symbols" | 剥离调试符号 |

### Native CPU 构建

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

启用 CPU 特定 SIMD 指令（AVX2、SSE4.2 等），提升压缩/解压吞吐。

## 运行时调优

### 线程数

```bash
fqc compress -i reads.fastq -o reads.fqc -t 4    # 4 线程
fqc compress -i reads.fastq -o reads.fqc          # 全部核心（默认）
```

### Pipeline 模式

```bash
fqc compress -i reads.fastq -o reads.fqc --pipeline
```

优势：I/O 与计算重叠执行，写后缓冲，在快速存储上吞吐更高。

### 内存控制

fqc 内置内存预算系统：

1. **系统内存检测** — 自动获取可用物理内存
2. **ChunkingStrategy** — 根据内存预算和读段大小动态计算最优块大小
3. **auto_memory_budget** — 默认使用系统可用内存的 75%

```bash
# 手动限制为 4 GB
fqc compress -i large.fastq -o large.fqc --memory-limit 4096
```

### 压缩级别

```bash
fqc compress -i reads.fastq -o reads.fqc -l 1   # 快速
fqc compress -i reads.fastq -o reads.fqc -l 6   # 默认
fqc compress -i reads.fastq -o reads.fqc -l 9   # 最大压缩
```

### 块大小

| 读段类型 | 默认块大小 | 说明 |
|----------|-----------|------|
| 短 (< 300bp) | 10,000 reads | ABC 需要足够样本构建共识 |
| 中 (300bp – 10kbp) | 1,000 reads | 平衡内存与压缩比 |
| 长 (> 10kbp) | 100 reads | 避免内存暴涨 |

## Profiling

### CPU Profiling (Linux)

```bash
cargo build --profile release-with-debug
perf record -g ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
perf report

# 火焰图
cargo install flamegraph
flamegraph -- ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

### CPU Profiling (macOS)

```bash
cargo build --profile release-with-debug
xcrun xctrace record --template "Time Profiler" --launch -- \
  ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

## 性能提示

1. **短读段** — 启用 reorder（默认开启），显著提升 ABC 压缩比
2. **大文件** — 使用 `--pipeline` 模式，I/O 与计算重叠
3. **内存受限** — 减小 `--block-size` 或设置 `--memory-limit`
4. **最大吞吐** — Native CPU 构建 + `--pipeline` + 充足线程
5. **最大压缩** — `-l 9` + `--lossy-quality illumina8` + 大块大小
6. **流式处理** — `--streaming` 模式禁用全局排序，适合 stdin/管道
