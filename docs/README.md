# fqc 技术文档

`fqc` 项目的纯 Markdown 技术文档。建议按以下顺序阅读。

## 总览

- [技术白皮书](whitepaper.md) —— 设计目标、整体架构与关键结果
- [理论基础](theory.md) —— FASTQ 压缩的信息论基础
- [竞品对比](comparison.md) —— 与 gzip / zstd / CRAM / DSRC 2 / Spring 等的深度对比

## 使用指南

- [快速开始](guide/quick-start.md)
- [安装](guide/installation.md)
- [CLI 参考](guide/cli.md)
- [执行模式](guide/modes.md)

## 架构与算法

- [架构总览](architecture/index.md)
- [性能路线图](architecture/performance-roadmap.md)
- [架构决策记录（ADR）](architecture/decisions/index.md)
- [算法总览](algorithms/index.md)
- [ABC 算法详解](algorithms/abc-deep-dive.md)

## 参考资料

- [.fqc 二进制格式规范](reference/format-spec.md)
- [参考文献与相关工作](reference/index.md)
- [基准测试报告](benchmarks/performance-report.md)
