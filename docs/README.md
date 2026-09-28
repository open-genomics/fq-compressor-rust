# fqc 技术文档

按 Diátaxis 四象限组织——按你的目标选入口，而不是线性通读。

## 教程（学习导向：第一次跑通）

- [快速开始](tutorials/quick-start.md) —— 从构建到压缩、检视、校验、解压的完整一遍

## 操作指南（问题导向：完成具体任务）

- [安装](how-to/installation.md) —— 源码构建与本地安装
- [如何选择执行模式](how-to/modes.md) —— archive / streaming / pipeline 的场景选型

## 参考（信息导向：查证事实）

- [CLI 参考](reference/cli.md) —— 全部命令与选项
- [.fqc 二进制格式规范](reference/format-spec.md) —— 归档布局、头部、流与索引
- [基准测试报告](reference/performance-report.md) —— 微型夹具实测
- [热点测量报告](reference/hotspot-report.md) —— 阶段计时与优化前后对比
- [真实语料压缩/吞吐](reference/real-corpus.md) —— ENA 公开切片 round-trip 与 C++ 横向参考
- [参考文献与相关工作](reference/bibliography.md) —— 学术文献与相关项目

## 解释（理解导向：为什么这样设计）

- [技术白皮书](explanation/whitepaper.md) —— 设计目标、整体架构与关键结果
- [理论基础](explanation/theory.md) —— FASTQ 压缩的信息论基础
- [竞品对比](explanation/comparison.md) —— 与 gzip / zstd / CRAM / DSRC 2 / Spring 等
- [架构总览](explanation/architecture/overview.md) —— 分层与归档模型
- [性能路线图](explanation/architecture/performance-roadmap.md) —— 瓶颈与优化方向
- [算法总览](explanation/algorithms/overview.md) —— 序列/质量/ID 路径决策
- [ABC 算法详解](explanation/algorithms/abc-deep-dive.md) —— contig 构建与增量编码

## 决策记录

架构决策的「为什么」记录为 Agent Note，见仓库根目录 `.agents/notes/implemented/architecture/`（原 `docs/architecture/decisions/` 已并入）。
