# 参考文献与相关工作

FASTQ 压缩领域的学术参考文献和相关工具。本页为 fqc 的设计决策提供全面的背景。

## 功能对比

fqc 与 gzip / zstd / CRAM / DSRC 2 / Spring 等的完整功能对比矩阵见[竞品深度对比](../comparison.md)。

## 学术参考文献

FASTQ 压缩领域的完整学术参考文献列表见[技术白皮书参考文献](../whitepaper.md)。

## 相关开源项目

| 项目 | 语言 | 描述 | 与 fqc 的相关性 |
|---------|----------|-------------|-----------------|
| [zstd-rs](https://github.com/gyscos/zstd-rs) | Rust | Zstandard 的 Rust 绑定 | fqc 的压缩后端 |
| [criterion](https://github.com/bheisler/criterion.rs) | Rust | 统计驱动基准测试 | fqc 的基准框架 |
| [seq_io](https://github.com/markschl/seq_io) | Rust | FASTA/FASTQ 解析库 | 替代解析器设计 |
| [Spring](https://github.com/shubhamchandak94/Spring) | C++ | 基于参考的 FASTQ 压缩器 | 主要竞品 |
| [DSRC](https://github.com/refresh-bio/DSRC) | C++ | FASTQ 压缩库 | 无参考竞品 |
| [seqtk](https://github.com/lh3/seqtk) | C | FASTQ 工具集 | 轻量级工具对比 |

## fqc 的差异化

fqc 在 FASTQ 压缩领域占据独特位置：

1. **块索引随机访问**：与基于流的压缩器（DSRC、FaStore）不同，fqc 的自包含块索引支持 O(log N) 查找，无需附属文件。

2. **组件级编码**：每个 FASTQ 组件使用独立调优的编解码器，不同于统一记录的方法。

3. **三种执行模式**：Archive、Streaming 和 Pipeline 模式产生相同的输出，允许用户在不产生格式碎片的情况下权衡内存与压缩比。

4. **运维工具**：同一二进制文件提供 compress、decompress、info 和 verify 命令 &mdash; 无需额外脚本。

5. **ABC 算法**：一种领域特定的短读段压缩算法，通过 contig 构建和差分编码利用生物读段相似性。

6. **内存安全**：Rust 的编译时保证消除了生物信息学 C/C++ 工具中常见的内存漏洞类别。

详细的竞争分析请参阅[竞品深度对比](../comparison.md)。
