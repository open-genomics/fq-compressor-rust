# 发布说明

本文档提供 fqc 项目的版本发布历史（中文）。

---

## 快速链接

- [English Version](CHANGELOG.md)
- [版本发布说明](changelog/zh/)
  - [v0.1.1](changelog/zh/v0.1.1.md) — 文档与 CI 改进
  - [v0.1.0](changelog/zh/v0.1.0.md) — 首个稳定版本

---

## [Unreleased]

## [0.1.1] - 2026-04-16

### 新增

#### 文档
- SECURITY.md 安全策略文档，包含漏洞报告流程
- GitBook 词汇表 (GLOSSARY.md)，解释领域术语
- GitBook anchors 和 search-pro 插件
- 增强 book.json 配置侧边栏链接和 PDF 设置

#### 安全
- docker.yml 中增加 Trivy 容器扫描
- 发布产物同时提供 SHA256 和 SHA512 校验和

#### CI/CD
- 文档变更的 PR 预览工作流
- CI 汇总任务，提供统一状态报告
- quality.yml 中新增文档检查任务
- 失败时收集测试日志便于调试

### 修复
- 性能文档中默认压缩级别从 3 更正为 6（中英文）
- Docker 工作流权限以支持安全扫描
- Pages 工作流 configure-pages 步骤

### 变更
- 更新 package.json 添加 docs:clean 和 docs:check 脚本
- 增强 CI 工作流，失败时收集日志
- 质量工作流增加质量门汇总

---

## [0.1.0] - 2026-03-07

### 核心亮点

fqc 首个稳定版本 — 用 Rust 编写的高性能 FASTQ 压缩器。完整移植了 C++ fq-compressor，功能对等，共享相同的 `.fqc` 归档格式。

### 压缩算法

| 算法 | 目标 | 方法 |
|-----------|--------|--------|
| ABC | 短读段 (< 300bp) | 共识序列 + 增量编码 |
| Zstd | 中/长读段 (≥ 300bp) | 长度前缀 + Zstd |
| SCM | 质量值 | 一阶/二阶算术编码 |

### 处理模式

- **默认模式**: 批量处理，全局 minimizer 重排序
- **流式模式**: 低内存 stdin，无重排序
- **流水线模式**: 3 阶段带背压，追求吞吐

### 主要特性

- 异步 I/O，后台预取/写后缓冲
- 透明解压 `.gz/.bz2/.xz/.zst` 输入
- 块索引格式，支持随机访问
- 配对端支持（分离文件/交错文件）
- 三种质量模式：无损 / Illumina8Bin / 丢弃

### 平台支持

预编译二进制文件：
- Linux (x64, ARM64) — glibc 和 musl (静态)
- macOS (Intel, Apple Silicon)
- Windows x64

### Docker

官方镜像: `ghcr.io/lessup/fq-compressor-rust:latest`

---

## 版本历史

| 版本 | 日期 | 类型 | 描述 |
|---------|------|------|-------------|
| [v0.1.1](changelog/zh/v0.1.1.md) | 2026-04-16 | 补丁版本 | 文档、安全和 CI 改进 |
| [v0.1.0](changelog/zh/v0.1.0.md) | 2026-03-07 | 重大版本 | 首个稳定版本 |

---

[Unreleased]: https://github.com/LessUp/fq-compressor-rust/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/LessUp/fq-compressor-rust/releases/tag/v0.1.1
[0.1.0]: https://github.com/LessUp/fq-compressor-rust/releases/tag/v0.1.0
