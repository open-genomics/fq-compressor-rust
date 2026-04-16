# 性能调优指南

> 另请参阅：[algorithms.md](algorithms.md)（算法细节）、[architecture.md](architecture.md)（Pipeline 架构）

## 构建优化

### Release 构建（默认）

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

针对当前 CPU 架构的最大性能：

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

启用 CPU 特定 SIMD 指令（AVX2、SSE4.2 等），可提升压缩/解压吞吐。

### Release with Debug Info

用于性能分析（保留符号表）：

```bash
cargo build --profile release-with-debug
```

### 精简二进制

禁用不需要的输入格式以减小二进制体积和编译时间：

```bash
# 仅 Zstd（不支持 gz/bz2/xz 输入）
cargo build --release --no-default-features

# 仅 gzip 输入支持
cargo build --release --no-default-features --features gz
```

---

## 运行时调优

### 线程数

fqc 使用 Rayon 进行并行块处理：

```bash
# 指定 4 线程
fqc compress -i reads.fastq -o reads.fqc -t 4

# 使用所有可用核心（默认）
fqc compress -i reads.fastq -o reads.fqc
```

### Pipeline 模式

`--pipeline` 启用 3 阶段流水线（读取器 → 压缩器 → 写入器），通过 crossbeam 有界通道实现背压：

```bash
fqc compress -i reads.fastq -o reads.fqc --pipeline
fqc decompress -i reads.fqc -o reads.fastq --pipeline
```

Pipeline 模式优势：
- I/O 与计算重叠执行
- AsyncWriter 提供写后缓冲（4MB buffer, depth 4）
- 在 NVMe/SSD 存储上吞吐更高

### 内存控制

#### Memory Budget（自动）

fqc 内置内存预算系统（`src/common/memory_budget.rs`）：

1. **系统内存检测** — 自动获取可用物理内存
2. **ChunkingStrategy** — 根据内存预算和读段大小动态计算最优块大小
3. **auto_memory_budget** — 默认使用系统可用内存的 75%

#### 手动限制

```bash
# 限制为 4 GB
fqc compress -i large.fastq -o large.fqc --memory-limit 4096
```

### 压缩级别

级别越高 = 压缩比越好但更慢：

```bash
fqc compress -i reads.fastq -o reads.fqc -l 1   # 快速
fqc compress -i reads.fastq -o reads.fqc -l 3   # 默认
fqc compress -i reads.fastq -o reads.fqc -l 9   # 最大压缩
```

### 块大小

更大的块提升压缩比但增加内存使用：

```bash
fqc compress -i reads.fastq -o reads.fqc --block-size 50000
```

默认块大小（按读段长度分类）：

| 读段类型 | 默认块大小 | 说明 |
|----------|-----------|------|
| 短读段 (< 300bp) | 10,000 reads | ABC 需要足够样本构建共识 |
| 中等读段 (300bp – 10kbp) | 1,000 reads | 平衡内存与压缩比 |
| 长读段 (> 10kbp) | 100 reads | 避免内存暴涨 |

---

## 性能分析

### CPU Profiling (Linux)

```bash
# 构建带符号的 release
cargo build --profile release-with-debug

# perf 采样
perf record -g ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
perf report

# 火焰图
cargo install flamegraph
flamegraph -- ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

### CPU Profiling (macOS)

```bash
cargo build --profile release-with-debug

# Instruments
xcrun xctrace record --template "Time Profiler" --launch -- \
  ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

### 内存 Profiling

```bash
# Valgrind (Linux)
valgrind --tool=massif ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
ms_print massif.out.*

# DHAT (堆分析)
cargo install dhat
```

---

## 基准测试

### 快速基准

```bash
# 计时压缩
time fqc compress -i reads.fastq -o reads.fqc

# 对比 pipeline vs 默认模式
time fqc compress -i reads.fastq -o reads_default.fqc
time fqc compress -i reads.fastq -o reads_pipeline.fqc --pipeline

# 查看压缩比
fqc info -i reads.fqc --detailed
```

### Bench Profile

`Cargo.toml` 中配置了 bench profile：

```toml
[profile.bench]
inherits = "release"
debug = true
lto = "thin"
```

### 对比其他工具

```bash
# Spring
time spring -c -i reads.fastq -o reads.spring

# fqzcomp
time fqzcomp reads.fastq reads.fqz

# fqc
time fqc compress -i reads.fastq -o reads.fqc

# 比较文件大小
ls -lh reads.spring reads.fqz reads.fqc
```

---

## 性能提示

1. **短读段** — 启用 reorder（默认开启），显著提升 ABC 压缩比
2. **大文件** — 使用 `--pipeline` 模式，I/O 与计算重叠
3. **内存受限** — 减小 `--block-size` 或设置 `--memory-limit`
4. **最大吞吐** — Native CPU 构建 + `--pipeline` + 充足线程
5. **最大压缩** — `-l 9` + `--lossy-quality illumina8` + 大块大小
6. **流式处理** — `--streaming` 模式禁用全局排序，适合 stdin/管道
