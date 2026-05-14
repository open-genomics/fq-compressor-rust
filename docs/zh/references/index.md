# 参考文献与相关工作

FASTQ 压缩领域的学术参考文献和相关工具。本页为 fqc 的设计决策提供背景信息。

## FASTQ 格式

- Cock, P.J.A., Fields, C.J., Goto, N., Heuer, M.L. & Rice, P.M. (2010). "The Sanger FASTQ file format for sequences with quality scores, and the Solexa/Illumina FASTQ variant." *Nucleic Acids Research*, 38(6), 1767–1771. [DOI:10.1093/nar/gkp1137](https://doi.org/10.1093/nar/gkp1137)

## FASTQ 压缩工具

| 工具 | 方法 | 随机访问 | 有损质量 | 双端支持 | 年份 |
|------|------|:---:|:---:|:---:|:---:|
| **fqc** | 块索引，ABC + Zstd | 是 | 是 | 是 | 2025 |
| **CRAM** | 参考序列，区间编码 | 有限 | 是 | 是 | 2011 |
| **DSRC** | 流式，LZ77 变体 | 否 | 否 | 否 | 2011 |
| **DSRC 2** | 流式，改进 LZ77 | 否 | 否 | 是 | 2013 |
| **FaStore** | 混合，delta + LZ77 | 否 | 否 | 否 | 2017 |
| **qvz** | 质量值专用，上下文模型 | N/A | 是 | N/A | 2015 |
| **Spring** | 参考序列，minimizer | 否 | 否 | 是 | 2019 |
| **Minicom** | 参考序列，BOSS | 否 | 否 | 否 | 2018 |
| **Leon** | 无参考，de Bruijn 图 | 否 | 否 | 否 | 2015 |

### 工具参考文献

**CRAM**
- Hsi-Yang Fritz, M., Leinonen, R., Cochrane, G. & Birney, E. (2011). "Efficient storage of high throughput DNA sequencing data using reference-based compression." *Genome Research*, 21(5), 734–740. [DOI:10.1101/gr.114819.110](https://doi.org/10.1101/gr.114819.110)

**DSRC / DSRC 2**
- Deorowicz, S. & Grabowski, S. (2011). "Compression of DNA sequence reads in FASTQ format." *Bioinformatics*, 27(6), 860–862. [DOI:10.1093/bioinformatics/btr013](https://doi.org/10.1093/bioinformatics/btr013)
- Deorowicz, S. & Grabowski, S. (2013). "Robust relative compression of genomes with random access." *Bioinformatics*, 29(22), 2886–2892. [DOI:10.1093/bioinformatics/btt505](https://doi.org/10.1093/bioinformatics/btt505)

**FaStore**
- Deorowicz, S., Grabowski, S., Robel, P. & Debudaj-Grabysz, A. (2017). "FaStore: a space-saving solution for raw sequencing data." *Bioinformatics*, 33(18), 2845–2852. [DOI:10.1093/bioinformatics/btx316](https://doi.org/10.1093/bioinformatics/btx316)

**qvz**
- Malysa, G. & Hernaez, M. (2015). "qvz: lossy compression of quality values." *Bioinformatics*, 31(19), 3122–3129. [DOI:10.1093/bioinformatics/btv338](https://doi.org/10.1093/bioinformatics/btv338)

**Spring**
- Patro, R. & Kingsford, C. (2019). "Spring: a next-generation compressor for FASTQ data." *Bioinformatics*, 35(14), i194–i202. [DOI:10.1093/bioinformatics/btz345](https://doi.org/10.1093/bioinformatics/btz345)

**Minicom**
- Patro, R. & Kingsford, C. (2018). "Minicom: lossless compression of FASTQ data." *bioRxiv*. [DOI:10.1101/281410](https://doi.org/10.1101/281410)

**Leon**
- Benoit, G., Lavenier, D., Drezen, E. & Rizk, G. (2015). "Leon: lossless and reference-free compression of FASTQ data." *BMC Bioinformatics*, 16, S3. [DOI:10.1186/1471-2105-16-S5-S3](https://doi.org/10.1186/1471-2105-16-S5-S3)

## 通用压缩

- Collet, Y. & Kucherawy, M. (2018). "Zstandard – Real-time data compression algorithm." *IETF RFC 8478*. [RFC 8478](https://datatracker.ietf.org/doc/html/rfc8478)
- Ziv, J. & Lempel, A. (1977). "A universal algorithm for sequential data compression." *IEEE Transactions on Information Theory*, 23(3), 337–343. [DOI:10.1109/TIT.1977.1055714](https://doi.org/10.1109/TIT.1977.1055714)
- Ziv, J. & Lempel, A. (1978). "Compression of individual sequences via variable-rate coding." *IEEE Transactions on Information Theory*, 24(5), 530–536. [DOI:10.1109/TIT.1978.1055934](https://doi.org/10.1109/TIT.1978.1055934)

## 生物信息学标准

- Li, H. et al. (2009). "The Sequence Alignment/Map format and SAMtools." *Bioinformatics*, 25(16), 2078–2079. [DOI:10.1093/bioinformatics/btp352](https://doi.org/10.1093/bioinformatics/btp352)
- GA4GH (2023). "Global Alliance for Genomics and Health – Data Standards." [ga4gh.org](https://www.ga4gh.org/our-products/)

## fqc 的差异化定位

fqc 在 FASTQ 压缩领域占据独特位置：

1. **块索引随机访问**：不同于流式压缩器（DSRC、FaStore），fqc 的块索引支持 O(log N) 查找，无需解压整个归档。
2. **组件级编码**：每个 FASTQ 组件（ID、序列、质量值、辅助数据）使用独立调优的编解码器，不同于统一记录方式。
3. **三种执行模式**：Archive、Streaming 和 Pipeline 模式输出相同文件，用户可在压缩比和内存之间灵活权衡，无需格式碎片化。
4. **运维工具集成**：同一二进制文件提供 compress、decompress、info、verify 命令，无需额外脚本。
5. **ABC 算法**：专为短读段设计的领域特定压缩算法，通过 contig 构建和 delta 编码利用生物学读段相似性。
