# 参考文献与相关工作

FASTQ 压缩领域的学术参考文献和相关工具。本页为 fqc 的设计决策提供背景信息。

## 特性对比矩阵

<div class="comparison-matrix">

| 特性 | fqc | gzip | zstd | CRAM | DSRC 2 | Spring |
|------|:---:|:----:|:----:|:----:|:------:|:------:|
| 随机访问 | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-partial">△</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> |
| 领域感知 | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> |
| 流式模式 | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> |
| Rust 原生 | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> |
| 零 unsafe | <span class="feature-check">✓</span> | <span class="feature-na">—</span> | <span class="feature-na">—</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> |
| 有损质量值 | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> |
| 双端测序 | <span class="feature-check">✓</span> | <span class="feature-na">—</span> | <span class="feature-na">—</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> |
| 单一二进制 | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> |

</div>

::: tip 图例
- <span class="feature-check">✓</span> 完全支持
- <span class="feature-partial">△</span> 部分支持
- <span class="feature-cross">✗</span> 不支持
- <span class="feature-na">—</span> 不适用（通用工具）
:::

## 学术参考文献

### FASTQ 格式

<ol class="reference-list">
  <li>
    <span class="ref-number">[1]</span>
    <span class="ref-authors">Cock, P.J.A., Fields, C.J., Goto, N., Heuer, M.L. & Rice, P.M.</span>
    <span class="ref-title">"The Sanger FASTQ file format for sequences with quality scores, and the Solexa/Illumina FASTQ variant."</span>
    <span class="ref-journal">Nucleic Acids Research</span>, 38(6), 1767–1771 (2010).
    <a href="https://doi.org/10.1093/nar/gkp1137" class="ref-link">DOI</a>
  </li>
</ol>

### FASTQ 压缩工具

<ol class="reference-list" start="2">
  <li>
    <span class="ref-number">[2]</span>
    <span class="ref-authors">Hsi-Yang Fritz, M., Leinonen, R., Cochrane, G. & Birney, E.</span>
    <span class="ref-title">"Efficient storage of high throughput DNA sequencing data using reference-based compression."</span>
    <span class="ref-journal">Genome Research</span>, 21(5), 734–740 (2011).
    <a href="https://doi.org/10.1101/gr.114819.110" class="ref-link">DOI</a>
    <span class="ref-note">— CRAM 格式</span>
  </li>
  <li>
    <span class="ref-number">[3]</span>
    <span class="ref-authors">Deorowicz, S. & Grabowski, S.</span>
    <span class="ref-title">"Compression of DNA sequence reads in FASTQ format."</span>
    <span class="ref-journal">Bioinformatics</span>, 27(6), 860–862 (2011).
    <a href="https://doi.org/10.1093/bioinformatics/btr013" class="ref-link">DOI</a>
    <span class="ref-note">— DSRC</span>
  </li>
  <li>
    <span class="ref-number">[4]</span>
    <span class="ref-authors">Deorowicz, S. & Grabowski, S.</span>
    <span class="ref-title">"Robust relative compression of genomes with random access."</span>
    <span class="ref-journal">Bioinformatics</span>, 29(22), 2886–2892 (2013).
    <a href="https://doi.org/10.1093/bioinformatics/btt505" class="ref-link">DOI</a>
    <span class="ref-note">— DSRC 2</span>
  </li>
  <li>
    <span class="ref-number">[5]</span>
    <span class="ref-authors">Deorowicz, S., Grabowski, S., Robel, P. & Debudaj-Grabysz, A.</span>
    <span class="ref-title">"FaStore: a space-saving solution for raw sequencing data."</span>
    <span class="ref-journal">Bioinformatics</span>, 33(18), 2845–2852 (2017).
    <a href="https://doi.org/10.1093/bioinformatics/btx316" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[6]</span>
    <span class="ref-authors">Malysa, G. & Hernaez, M.</span>
    <span class="ref-title">"qvz: lossy compression of quality values."</span>
    <span class="ref-journal">Bioinformatics</span>, 31(19), 3122–3129 (2015).
    <a href="https://doi.org/10.1093/bioinformatics/btv338" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[7]</span>
    <span class="ref-authors">Patro, R. & Kingsford, C.</span>
    <span class="ref-title">"Spring: a next-generation compressor for FASTQ data."</span>
    <span class="ref-journal">Bioinformatics</span>, 35(14), i194–i202 (2019).
    <a href="https://doi.org/10.1093/bioinformatics/btz345" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[8]</span>
    <span class="ref-authors">Benoit, G., Lavenier, D., Drezen, E. & Rizk, G.</span>
    <span class="ref-title">"Leon: lossless and reference-free compression of FASTQ data."</span>
    <span class="ref-journal">BMC Bioinformatics</span>, 16, S3 (2015).
    <a href="https://doi.org/10.1186/1471-2105-16-S5-S3" class="ref-link">DOI</a>
  </li>
</ol>

### 通用压缩

<ol class="reference-list" start="9">
  <li>
    <span class="ref-number">[9]</span>
    <span class="ref-authors">Collet, Y. & Kucherawy, M.</span>
    <span class="ref-title">"Zstandard – Real-time data compression algorithm."</span>
    <span class="ref-journal">IETF RFC 8478</span> (2018).
    <a href="https://datatracker.ietf.org/doc/html/rfc8478" class="ref-link">RFC</a>
  </li>
  <li>
    <span class="ref-number">[10]</span>
    <span class="ref-authors">Ziv, J. & Lempel, A.</span>
    <span class="ref-title">"A universal algorithm for sequential data compression."</span>
    <span class="ref-journal">IEEE Trans. Information Theory</span>, 23(3), 337–343 (1977).
    <a href="https://doi.org/10.1109/TIT.1977.1055714" class="ref-link">DOI</a>
  </li>
</ol>

## 相关开源项目

| 项目 | 语言 | 描述 |
|------|------|------|
| [zstd-rs](https://github.com/gyscos/zstd-rs) | Rust | Rust 的 Zstandard 压缩绑定 |
| [criterion](https://github.com/bheisler/criterion.rs) | Rust | 统计驱动的基准测试库 |
| [seq_io](https://github.com/markschl/seq_io) | Rust | FASTA/FASTQ 解析库 |
| [Spring](https://github.com/shubhamchandak94/Spring) | C++ | 基于参考序列的 FASTQ 压缩器 |
| [DSRC](https://github.com/refresh-bio/DSRC) | C++ | FASTQ 压缩库 |

## fqc 的差异化定位

fqc 在 FASTQ 压缩领域占据独特位置：

1. **块索引随机访问**：不同于流式压缩器（DSRC、FaStore），fqc 的块索引支持 O(log N) 查找，无需解压整个归档。

2. **组件级编码**：每个 FASTQ 组件（ID、序列、质量值、辅助数据）使用独立调优的编解码器，不同于统一记录方式。

3. **三种执行模式**：Archive、Streaming 和 Pipeline 模式输出相同文件，用户可在压缩比和内存之间灵活权衡，无需格式碎片化。

4. **运维工具集成**：同一二进制文件提供 compress、decompress、info、verify 命令，无需额外脚本。

5. **ABC 算法**：专为短读段设计的领域特定压缩算法，通过 contig 构建和 delta 编码利用生物学读段相似性。
