# 竞品深度对比

<div class="whitepaper-hero">
<div class="whitepaper-title">fqc 竞争格局分析</div>
<div class="whitepaper-meta">
  从功能、性能和工程维度对 FASTQ 压缩工具进行结构化比较。
</div>
</div>

## 执行摘要

FASTQ 压缩领域涵盖通用工具（gzip、zstd）、领域特定无参考压缩器（DSRC 2、Leon、FaStore、fqc）和基于参考的解决方案（CRAM、Spring）。每种工具在压缩比、随机访问、运维复杂度和部署约束之间占据不同的设计空间位置。

**fqc 的定位**：具有块索引随机访问、无参考操作和内存安全实现的领域感知压缩。它针对通用工具（方便但压缩比次优）和基于参考的压缩器（压缩比优秀但需要外部参考和复杂部署）之间的空白。

## 功能对比矩阵

<div class="comparison-matrix">

| 能力 | fqc | gzip | zstd | CRAM | DSRC 2 | Spring | FaStore | Leon |
|:-----------|:---:|:----:|:----:|:----:|:------:|:------:|:-------:|:----:|
| **核心压缩** |||||||||
| 随机访问 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| 领域感知 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> |
| 无参考 | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> |
| 流式模式 | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> |
| 双端支持 | <span class="feature-check">&#10003;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> |
| **质量值模式** |||||||||
| 无损质量值 | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> |
| 有损质量值 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> |
| 质量分箱 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| **工程** |||||||||
| 单二进制 | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> |
| 内存安全 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| 零 unsafe | <span class="feature-check">&#10003;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| Verify 命令 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| Info 命令 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| 结构化退出码 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| **格式特性** |||||||||
| 自包含索引 | <span class="feature-check">&#10003;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-partial">&#9651;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> |
| 向前兼容 | <span class="feature-check">&#10003;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| 校验和 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |

</div>

> [!TIP]
> **图例
- <span class="feature-check">&#10003;</span> 完全支持
- <span class="feature-partial">&#9651;</span> 部分/有限支持
- <span class="feature-cross">&#10007;</span> 不支持
- <span class="feature-na">&mdash;</span> 不适用**
>
## 压缩比比较

压缩比因读长、覆盖深度和参考可用性而显著变化。

### 短读段数据（Illumina，150bp）

| 工具 | 压缩比 | 说明 |
|------|--------|------|
| gzip -9 | 2.5&ndash;3.0&times; | 基准；无领域感知 |
| zstd -19 | 2.8&ndash;3.5&times; | 优于 gzip；仍无领域感知 |
| DSRC 2 | 3.0&ndash;4.0&times; | 三阶算术编码器 |
| Leon | 3.5&ndash;5.0&times; | 概率 de Bruijn 图 |
| **fqc (Archive)** | **2.4&ndash;4.0&times;** | ABC + 重排；无参考 |
| Spring | 5.0&ndash;15&times; | 需要参考基因组 |
| CRAM | 4.0&ndash;20&times; | 需要参考基因组 |

在无参考短读段数据上，fqc 的 ABC 算法配合重排接近 DSRC 2 和 Leon 的性能，同时提供它们所缺乏的随机访问能力。

### 长读段数据（PacBio/ONT，>10kb）

| 工具 | 压缩比 | 说明 |
|------|--------|------|
| gzip -9 | 1.8&ndash;2.2&times; | 长读段相似性较低 |
| zstd -19 | 2.0&ndash;2.5&times; | |
| **fqc (Streaming)** | **2.0&ndash;2.5&times;** | Zstd 支持；无重排收益 |
| DSRC 2 | 2.2&ndash;3.0&times; | |

对于长读段，基于相似性的方法收益递减。fqc 回退到 Zstd，在通用压缩器的基础上增加了块索引优势。

## 速度与资源比较

### 压缩吞吐量

| 工具 | 吞吐量 | 内存 | 并行性 |
|------|--------|------|--------|
| gzip | ~50 MB/s | 低 | 无 |
| zstd | ~200 MB/s | 中等 | 无 |
| DSRC 2 | ~30 MB/s | 中等 | 有限 |
| **fqc (Pipeline)** | **~100 MB/s** | 中等 | 完整 |
| **fqc (Archive)** | **~50 MB/s** | 高 | 部分 |
| **fqc (Streaming)** | **~80 MB/s** | 低 | 无 |

fqc 的 Pipeline 模式通过分阶段并行实现有竞争力的吞吐量。Archive 模式通过全局分析牺牲吞吐量以换取压缩比。

### 解压吞吐量

所有测试工具在现代硬件上均达到 >100 MB/s 解压速度。解压的瓶颈通常是 I/O 而非 CPU。

## 工程质量比较

### 部署复杂度

| 工具 | 依赖 | 安装 | 容器大小 |
|------|------|------|----------|
| gzip | 无 | 系统包 | ~100 KB |
| zstd | 无 | 系统包 | ~500 KB |
| **fqc** | **无** | **单二进制** | **~2.4 MB** |
| DSRC 2 | C++ 运行时 | 从源码编译 | ~5 MB |
| Spring | C++ 运行时、Python | 编译 + 配置 | ~15 MB |
| CRAM | C 运行时、htslib | 包管理器 | ~10 MB |

fqc 的单静态二进制文件消除了依赖地狱，这是生物信息学流程中的常见痛点，不同生态系统的工具（C++、Python、Java）必须共存。

### 正确性保证

| 工具 | 内存安全 | 模糊测试 | 形式规范 |
|------|----------|----------|----------|
| gzip | 否 (C) | 部分 | 否 |
| zstd | 部分 (C) | 是 | 否 |
| **fqc** | **是 (Rust)** | **计划中** | **是 (.fqc 规范)** |
| DSRC 2 | 否 (C++) | 未知 | 否 |
| Spring | 否 (C++) | 未知 | 否 |

Rust 的所有权模型在编译时消除数据竞争、释放后使用和缓冲区溢出。对于处理潜在恶意输入（公共数据集）的生物信息学工具，这是安全优势。

## 使用场景建议

### 选择 fqc 的场景：

- 需要对归档测序数据进行**随机访问**
- 在**无参考环境**中操作（宏基因组学、de novo）
- 重视**运维简单性**（单二进制，无附属文件）
- 生产流程需要**内存安全**
- 希望**灵活的执行模式**而不产生格式碎片

### 选择基于参考的工具（Spring、CRAM）的场景：

- 拥有**高质量的参考基因组**
- **最大压缩比**是首要目标
- 可以容忍**部署复杂度**

### 选择通用工具（zstd）的场景：

- **速度至上**，压缩比次要
- 需要对任意数据进行**流式压缩**
- 领域感知**无收益**（例如已压缩数据）

## 结论

fqc 在 FASTQ 压缩领域占据独特位置。它提供与 DSRC 2 和 Leon 竞争的领域感知压缩比，同时增加了现有工具均未结合提供的块索引随机访问、运维工具和内存安全。无参考设计使其适用于新兴的测序应用（宏基因组学、单细胞、纳米孔），其中参考基因组不可用或不完整。

对单二进制部署、结构化退出码和格式稳定性的工程强调反映了在生产环境中维护生物信息学流程的经验教训。
