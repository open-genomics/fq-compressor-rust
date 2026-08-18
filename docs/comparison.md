# 竞品对比

FASTQ 压缩领域涵盖通用工具（gzip、zstd）、领域特定无参考压缩器（DSRC 2、Leon、FaStore、fqc）和基于参考的解决方案（CRAM、Spring）。

**fqc 的定位**：块索引随机访问、无参考操作、内存安全的领域感知压缩。它针对通用工具（方便但压缩比通常次优）和基于参考的压缩器（压缩比优秀但需要外部参考）之间的空白。

> **证据边界。** 下表中 gzip / zstd / DSRC 2 / Leon / Spring / CRAM 的压缩比与吞吐量为文献或经验区间，未在本仓库环境中复测。fqc 压缩比仅有微型夹具实测（见 [基准报告](benchmarks/performance-report.md) 与白皮书 7.1 节）。fqc 吞吐量没有生产规模数字。

## 功能对比

| 能力 | fqc | gzip | zstd | CRAM | DSRC 2 | Spring | FaStore | Leon |
|:-----------|:---:|:----:|:----:|:----:|:------:|:------:|:-------:|:----:|
| **核心压缩** |||||||||
| 随机访问 | yes | no | no | yes | no | no | no | no |
| 领域感知 | yes | no | no | yes | yes | yes | yes | yes |
| 无参考 | yes | yes | yes | no | yes | no | yes | yes |
| 流式模式 | yes | yes | yes | no | yes | no | yes | no |
| 双端支持 | yes | n/a | n/a | yes | yes | yes | yes | no |
| **质量值模式** |||||||||
| 无损质量值 | yes | yes | yes | yes | yes | yes | yes | yes |
| 有损质量值 | yes | no | no | yes | no | yes | yes | yes |
| 质量分箱 | yes | no | no | yes | no | yes | no | no |
| **工程** |||||||||
| 单二进制 | yes | yes | yes | yes | yes | no | yes | yes |
| 内存安全 | yes | no | no | no | no | no | no | no |
| 零 unsafe | yes | n/a | n/a | no | no | no | no | no |
| Verify 命令 | yes | no | no | no | no | no | no | no |
| Info 命令 | yes | no | no | no | no | no | no | no |
| 结构化退出码 | yes | no | no | no | no | no | no | no |
| **格式特性** |||||||||
| 自包含索引 | yes | n/a | n/a | partial | n/a | n/a | n/a | n/a |
| 向前兼容 | partial | n/a | n/a | yes | no | no | no | no |
| 校验和 | yes | no | no | yes | no | no | no | no |

`partial`：fqc 读取器只接受 major == 2。有损质量：`illumina8` 分箱、`qvz`（固定 8 级最近邻码本）和 `discard` 可用；`qvz` 不是训练过的率失真 QVZ。

## 压缩比

压缩比因读长、覆盖深度和参考可用性而显著变化。

### 短读段数据（Illumina，150bp）

| 工具 | 压缩比 | 说明 |
|------|--------|------|
| gzip -9 | 2.5-3.0x | 文献/经验区间；无领域感知 |
| zstd -19 | 2.8-3.5x | 文献/经验区间 |
| DSRC 2 | 3.0-4.0x | 文献/经验区间 |
| Leon | 3.5-5.0x | 文献/经验区间 |
| fqc (Archive) | 2.39x（微型夹具实测） | 20 条 150bp 读，2231 字节；大文件未验证 |
| Spring | 5.0-15x | 文献区间；需要参考基因组 |
| CRAM | 4.0-20x | 文献区间；需要参考基因组 |

在无参考短读上，ABC + 重排有潜力接近 DSRC 2 / Leon，但尚未在大型数据集上验证。随机访问是已实现的格式能力，不是外推的性能数字。

### 长读段数据（PacBio/ONT，>10kb）

| 工具 | 压缩比 | 说明 |
|------|--------|------|
| gzip -9 | 1.8-2.2x | 文献/经验区间 |
| zstd -19 | 2.0-2.5x | 文献/经验区间 |
| fqc (Streaming) | 未在本仓库复测 | 实现上回退到 Zstd + 块索引 |
| DSRC 2 | 2.2-3.0x | 文献/经验区间 |

对于长读段，基于相似性的方法收益递减。fqc 的增量是块索引，不是另套长读算法。

## 速度与资源

### 压缩吞吐量

| 工具 | 吞吐量 | 内存 | 并行性 |
|------|--------|------|--------|
| gzip | ~50 MB/s | 低 | 无 |
| zstd | ~200 MB/s | 中等 | 无 |
| DSRC 2 | ~30 MB/s | 中等 | 有限 |
| fqc (Pipeline) | 未验证 | 中等 | 分阶段 |
| fqc (Archive) | 未验证 | 高（全量摄入） | 部分 |
| fqc (Streaming) | 未验证 | 低 | 无 |

gzip / zstd / DSRC 2 为常见量级，不是本仓库对照实验。fqc 三种模式的相对关系（Archive 换压缩比、Streaming 换内存、Pipeline 换并发）由实现保证，绝对 MB/s 需要生产规模复测。

### 解压吞吐量

通用工具在现代硬件上常达到 >100 MB/s；fqc 解压未做对等测量。解压瓶颈通常是 I/O 而非 CPU，但这同样未经本仓库验证。

## 工程质量

### 部署复杂度

| 工具 | 依赖 | 安装 | 典型体积 |
|------|------|------|----------|
| gzip | 无 | 系统包 | ~100 KB |
| zstd | 无 | 系统包 | ~500 KB |
| fqc | 无（除 libc） | 从源码构建单二进制 | 约 2.4 MB（本仓库 release） |
| DSRC 2 | C++ 运行时 | 从源码编译 | ~5 MB |
| Spring | C++ 运行时、Python | 编译 + 配置 | ~15 MB |
| CRAM | C 运行时、htslib | 包管理器 | ~10 MB |

fqc 无预编译发布物；上表体积来自本仓库一次 release 构建，不是分发渠道保证。

### 正确性保证

| 工具 | 内存安全 | 模糊测试 | 形式规范 |
|------|----------|----------|----------|
| gzip | 否 (C) | 部分 | 否 |
| zstd | 部分 (C) | 是 | 否 |
| fqc | 是 (Rust，`unsafe_code = deny`) | 无 | 是（`docs/reference/format-spec.md` + `openspec/specs/`） |
| DSRC 2 | 否 (C++) | 未知 | 否 |
| Spring | 否 (C++) | 未知 | 否 |

## 使用场景

### 选择 fqc 的场景

- 需要对归档测序数据进行随机访问
- 在无参考环境中操作（宏基因组学、de novo）
- 重视运维简单性（单二进制，无附属索引文件）
- 希望灵活的执行模式而不产生格式碎片

### 选择基于参考的工具（Spring、CRAM）的场景

- 拥有高质量参考基因组
- 最大压缩比是首要目标
- 可以容忍部署复杂度

### 选择通用工具（zstd）的场景

- 速度优先，压缩比次要
- 需要对任意数据流式压缩
- 领域感知无收益（例如已压缩数据）

## 结论

fqc 把领域感知压缩、块索引随机访问和内存安全的 Rust 实现放在同一个 CLI 里。与 DSRC 2 / Leon 的压缩比竞争尚未被大型数据集证实；已证实的是格式能力（info / verify / 范围解压）和当前微型夹具上的 2.39x。下一步应先补证据，而不是根据未验证表格加功能。
