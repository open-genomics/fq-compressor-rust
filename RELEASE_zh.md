# fqc 发布说明

[English](../CHANGELOG.md) | [C++ 版本 (fq-compressor)](https://github.com/LessUp/fq-compressor)

> [fq-compressor](https://github.com/LessUp/fq-compressor) 的 Rust 实现，两个版本共享相同的 `.fqc` 归档格式与 ABC/SCM 压缩算法，以 Rayon + crossbeam 替代 Intel TBB 并引入异步 I/O。

---

## [0.1.0] - 2026-03-07

**fqc 首次发布** — 高性能 FASTQ 压缩工具（Rust 实现）。

### 核心特性

#### 压缩算法

| 算法 | 说明 | 适用场景 |
|------|------|---------|
| **ABC** | 共识序列 + 增量编码 | 短读段 (< 300bp) |
| **Zstd** | 长度前缀编码 | 中/长读段 (≥ 300bp) |
| **SCM** | 统计上下文模型 + 算术编码 | 质量值压缩 |
| **ID 压缩** | 分词 + 增量编码 | 读段标识符 |

#### 处理模式

| 模式 | 说明 | 使用场景 |
|------|------|---------|
| 默认 | 批量处理 + 全局 minimizer 重排 | 标准压缩 |
| 流式 | 低内存 stdin 压缩，无全局重排 | 管道、内存受限环境 |
| 流水线 | 3 阶段 Reader→Compressor→Writer | 最大吞吐量 |

#### 功能特性

- **异步 I/O** — 后台预取与写入缓冲
- **压缩输入** — 透明解压 `.gz`、`.bz2`、`.xz`、`.zst`
- **随机访问** — 块索引归档格式，支持部分解压
- **范围提取** — 提取指定读段范围（如 `--range 1:1000`）

#### 配对端支持

- 分离文件输入（`-i R1.fastq -2 R2.fastq`）
- 交错文件输入（`--interleaved`）
- PE 存储布局（交错/连续）
- 解压分离输出（`--split-pe`）

#### 质量模式

| 模式 | 说明 | 压缩提升 |
|------|------|---------|
| 无损 | 精确保留质量值 | 基准 |
| Illumina8Bin | 8 分箱量化 | ~30% |
| 丢弃 | 全部替换为 `!` (Phred 0) | 最大 |

#### 退出码

| 码 | 名称 | 说明 |
|----|------|------|
| 0 | Success | 操作成功 |
| 1 | Usage | 无效参数或缺失文件 |
| 2 | IoError | I/O 错误 |
| 3 | FormatError | 格式错误 |
| 4 | ChecksumError | 校验和不匹配 |
| 5 | Unsupported | 不支持的编码器 |

### 测试

- **131 个测试**，覆盖 8 个测试套件
- 算法测试（ID/质量压缩器、PE 优化器）
- DNA 工具测试（编码表、反向互补）
- 端到端测试
- 二进制格式测试
- FASTQ 解析器测试
- 重排映射测试
- 往返压缩测试
- 类型定义测试

### 安装

#### 从源码构建

```bash
cargo build --release
```

二进制文件位于 `target/release/fqc`（Windows 为 `fqc.exe`）。

#### Docker

```bash
# 拉取镜像
docker pull ghcr.io/lessup/fq-compressor-rust:latest

# 或本地构建
docker build -t fqc .

# 运行
docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc
docker run --rm -v $(pwd):/data fqc decompress -i /data/reads.fqc -o /data/reads.fastq
```

### 平台支持

| 平台 | 架构 | 类型 |
|------|------|------|
| Linux | x64 | glibc、musl（静态） |
| Linux | ARM64 | glibc、musl（静态） |
| macOS | x64 | Intel Mac |
| macOS | ARM64 | Apple Silicon |
| Windows | x64 | MSVC |

### 校验文件完整性

```bash
sha256sum -c checksums-sha256.txt
```

### 快速开始

```bash
# 压缩
fqc compress -i reads.fastq -o reads.fqc

# 解压
fqc decompress -i reads.fqc -o reads.fastq

# 查看信息
fqc info -i reads.fqc

# 验证完整性
fqc verify -i reads.fqc
```

---

## 内部变更

不影响最终用户的开发和基础设施变更。

### 2026-03-10 - Workflow 深度标准化

- Pages workflow 重命名：`docs-pages.yml` → `pages.yml`
- CI workflow 统一 `permissions: contents: read` 与 `concurrency` 配置
- Pages workflow 补充 `actions/configure-pages@v5` 步骤
- Pages workflow 添加 `paths` 触发过滤，减少无效构建

---

## 版本概览

| 版本 | 日期 | 类型 | 说明 |
|------|------|------|------|
| 0.1.0 | 2026-03-07 | Major | 首次发布 |
