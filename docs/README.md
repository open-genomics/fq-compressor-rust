# fqc 文档

本目录包含 fqc (fq-compressor-rust) 项目的技术文档。

## 文档索引

| 文档 | 内容 | 适合读者 |
|------|------|----------|
| [architecture.md](architecture.md) | 项目架构、模块职责、数据流 | 新贡献者、代码审查者 |
| [format-spec.md](format-spec.md) | FQC 二进制格式规范 (v1.0) | 格式实现者、互操作开发者 |
| [algorithms.md](algorithms.md) | 压缩算法详解 (ABC / SCM / Reorder) | 算法研究者、性能优化者 |
| [development.md](development.md) | 开发指南、测试、CI/CD、发版流程 | 项目贡献者 |
| [performance.md](performance.md) | 性能调优、Profiling、基准测试 | 运维、性能工程师 |

## 快速导航

- **想了解项目结构？** → [architecture.md](architecture.md)
- **想实现兼容的 FQC 读写器？** → [format-spec.md](format-spec.md)
- **想理解 ABC 压缩原理？** → [algorithms.md](algorithms.md)
- **想开始贡献代码？** → [development.md](development.md)
- **想优化压缩/解压性能？** → [performance.md](performance.md)

## 相关文件

- [README.md](../README.md) — 项目概述与使用说明
- [CHANGELOG.md](../CHANGELOG.md) — 版本变更记录
- [Cargo.toml](../Cargo.toml) — 依赖与构建配置
