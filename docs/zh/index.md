---
layout: home

hero:
  name: fqc
  text: "Rust FASTQ 压缩工具"
  tagline: "块索引 .fqc 归档工具，用于压缩、还原、检查和验证 FASTQ 数据。"
  image:
    src: /logo.svg
    alt: fqc logo
  actions:
    - theme: brand
      text: 快速开始
      link: /zh/guide/quick-start
    - theme: alt
      text: GitHub 仓库
      link: https://github.com/LessUp/fq-compressor-rust

features:
  - icon: 🧬
    title: "FASTQ 原生设计"
    details: "序列、质量值、读段 ID 和双端布局作为独立的归档组件编码，而非隐藏在通用压缩流中。"
  - icon: 📦
    title: "块索引归档"
    details: ".fqc 容器保留每块元数据、文件尾和索引，使 info、verify 和范围提取工作流有据可依。"
  - icon: 🔍
    title: "内置操作命令"
    details: "单一二进制文件提供 compress、decompress、info、verify 命令；用户无需额外脚本即可验证归档。"
  - icon: ⚙️
    title: "显式内存模式"
    details: "默认 archive 模式全局优化，--streaming 严格控内存，--memory-limit 0 自动选择内存。"
---

## 选择你的路径

| 需求 | 命令示例 |
| --- | --- |
| 标准单端归档 | `fqc compress -i reads.fastq -o reads.fqc` |
| 低内存压缩 | `fqc compress -i reads.fastq -o reads.fqc --streaming --memory-limit 1024` |
| 双端输入 | `fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc` |
| 使用前检查归档 | `fqc verify -i reads.fqc` |
| 检查编解码器和块 | `fqc info -i reads.fqc --detailed --show-codecs` |

从 [快速开始](/zh/guide/quick-start) 入门，或直接查看 [CLI 参考](/zh/guide/cli) 了解标志和模式详情。
