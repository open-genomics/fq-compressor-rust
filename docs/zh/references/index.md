# 参考文献与相关工作

FASTQ 压缩领域的学术参考文献和相关工具。本页为 fqc 的设计决策提供全面的背景。

## 功能对比

<div class="comparison-matrix">

| 功能 | fqc | gzip | zstd | CRAM | DSRC 2 | Spring |
|---------|:---:|:----:|:----:|:----:|:------:|:------:|
| 随机访问 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-partial">&#9651;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| 领域感知 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> |
| 流式模式 | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> |
| Rust 原生 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| 零 unsafe | <span class="feature-check">&#10003;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| 有损质量值 | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| 双端支持 | <span class="feature-check">&#10003;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> |
| 单二进制 | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> |

</div>

::: tip 图例
- <span class="feature-check">&#10003;</span> 完全支持
- <span class="feature-partial">&#9651;</span> 部分/有限支持
- <span class="feature-cross">&#10007;</span> 不支持
- <span class="feature-na">&mdash;</span> 不适用（通用工具）
:::

## 学术参考文献

### FASTQ 格式

<ol class="reference-list">
  <li>
    <span class="ref-number">[1]</span>
    <span class="ref-authors">Cock, P.J.A. 等</span>
    <span class="ref-title">"The Sanger FASTQ file format for sequences with quality scores, and the Solexa/Illumina FASTQ variant."</span>
    <span class="ref-journal">Nucleic Acids Research</span>, 38(6), 1767&ndash;1771 (2010).
    <a href="https://doi.org/10.1093/nar/gkp1137" class="ref-link">DOI</a>
  </li>
</ol>

### FASTQ 压缩工具

<ol class="reference-list" start="2">
  <li>
    <span class="ref-number">[2]</span>
    <span class="ref-authors">Hsi-Yang Fritz, M. 等</span>
    <span class="ref-title">"Efficient storage of high throughput DNA sequencing data using reference-based compression."</span>
    <span class="ref-journal">Genome Research</span>, 21(5), 734&ndash;740 (2011).
    <a href="https://doi.org/10.1101/gr.114819.110" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; CRAM 格式</span>
  </li>
  <li>
    <span class="ref-number">[3]</span>
    <span class="ref-authors">Deorowicz, S. &amp; Grabowski, S.</span>
    <span class="ref-title">"Compression of DNA sequence reads in FASTQ format."</span>
    <span class="ref-journal">Bioinformatics</span>, 27(6), 860&ndash;862 (2011).
    <a href="https://doi.org/10.1093/bioinformatics/btr013" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; DSRC</span>
  </li>
  <li>
    <span class="ref-number">[4]</span>
    <span class="ref-authors">Deorowicz, S. &amp; Grabowski, S.</span>
    <span class="ref-title">"Robust relative compression of genomes with random access."</span>
    <span class="ref-journal">Bioinformatics</span>, 29(22), 2886&ndash;2892 (2013).
    <a href="https://doi.org/10.1093/bioinformatics/btt505" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; DSRC 2</span>
  </li>
  <li>
    <span class="ref-number">[5]</span>
    <span class="ref-authors">Deorowicz, S. 等</span>
    <span class="ref-title">"FaStore: a space-saving solution for raw sequencing data."</span>
    <span class="ref-journal">Bioinformatics</span>, 33(18), 2845&ndash;2852 (2017).
    <a href="https://doi.org/10.1093/bioinformatics/btx316" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[6]</span>
    <span class="ref-authors">Malysa, G. &amp; Hernaez, M.</span>
    <span class="ref-title">"qvz: lossy compression of quality values."</span>
    <span class="ref-journal">Bioinformatics</span>, 31(19), 3122&ndash;3129 (2015).
    <a href="https://doi.org/10.1093/bioinformatics/btv338" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[7]</span>
    <span class="ref-authors">Patro, R. &amp; Kingsford, C.</span>
    <span class="ref-title">"Spring: a next-generation compressor for FASTQ data."</span>
    <span class="ref-journal">Bioinformatics</span>, 35(14), i194&ndash;i202 (2019).
    <a href="https://doi.org/10.1093/bioinformatics/btz345" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[8]</span>
    <span class="ref-authors">Benoit, G. 等</span>
    <span class="ref-title">"Leon: lossless and reference-free compression of FASTQ data."</span>
    <span class="ref-journal">BMC Bioinformatics</span>, 16, S3 (2015).
    <a href="https://doi.org/10.1186/1471-2105-16-S5-S3" class="ref-link">DOI</a>
  </li>
</ol>

### 通用压缩理论

<ol class="reference-list" start="9">
  <li>
    <span class="ref-number">[9]</span>
    <span class="ref-authors">Collet, Y. &amp; Kucherawy, M.</span>
    <span class="ref-title">"Zstandard &ndash; Real-time data compression algorithm."</span>
    <span class="ref-journal">IETF RFC 8478</span> (2018).
    <a href="https://datatracker.ietf.org/doc/html/rfc8478" class="ref-link">RFC</a>
  </li>
  <li>
    <span class="ref-number">[10]</span>
    <span class="ref-authors">Ziv, J. &amp; Lempel, A.</span>
    <span class="ref-title">"A universal algorithm for sequential data compression."</span>
    <span class="ref-journal">IEEE Trans. Information Theory</span>, 23(3), 337&ndash;343 (1977).
    <a href="https://doi.org/10.1109/TIT.1977.1055714" class="ref-link">DOI</a>
  </li>
</ol>

### 信息论与基础

<ol class="reference-list" start="11">
  <li>
    <span class="ref-number">[11]</span>
    <span class="ref-authors">Shannon, C.E.</span>
    <span class="ref-title">"A mathematical theory of communication."</span>
    <span class="ref-journal">Bell System Technical Journal</span>, 27(3), 379&ndash;423 (1948).
    <a href="https://doi.org/10.1002/j.1538-7305.1948.tb01338.x" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; 香农熵，压缩基本极限</span>
  </li>
  <li>
    <span class="ref-number">[12]</span>
    <span class="ref-authors">Yu, Z. 等</span>
    <span class="ref-title">"Quality score compression improves genotyping accuracy."</span>
    <span class="ref-journal">Nature Biotechnology</span>, 38, 1184&ndash;1188 (2020).
    <a href="https://doi.org/10.1038/s41587-020-0552-1" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; 有损质量压缩与下游分析影响</span>
  </li>
  <li>
    <span class="ref-number">[13]</span>
    <span class="ref-authors">Ferragina, P. &amp; Venturini, R.</span>
    <span class="ref-title">"Compressed cache-oblivious string B-tree."</span>
    <span class="ref-journal">Theoretical Computer Science</span>, 412(29), 3555&ndash;3568 (2011).
    <a href="https://doi.org/10.1016/j.tcs.2011.02.023" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; 压缩-访问权衡理论</span>
  </li>
  <li>
    <span class="ref-number">[14]</span>
    <span class="ref-authors">Manzini, G.</span>
    <span class="ref-title">"An analysis of the Burrows-Wheeler transform."</span>
    <span class="ref-journal">Journal of the ACM</span>, 48(3), 407&ndash;430 (2001).
    <a href="https://doi.org/10.1145/382780.382782" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; BWT 理论及其压缩特性</span>
  </li>
  <li>
    <span class="ref-number">[15]</span>
    <span class="ref-authors">Koslicki, D. &amp; Falush, D.</span>
    <span class="ref-title">"Introduction to compression strategies in Bioinformatics."</span>
    <span class="ref-journal">Briefings in Bioinformatics</span>, 13(3), 305&ndash;313 (2012).
    <a href="https://doi.org/10.1093/bib/bbr073" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; DNA 压缩技术综述</span>
  </li>
</ol>

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

详细的竞争分析请参阅<a href="../comparison">竞品深度对比</a>。
