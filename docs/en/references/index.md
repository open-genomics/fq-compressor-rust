# References & Related Work

Academic references and related tools in FASTQ compression. This page provides comprehensive context for the design decisions behind fqc.

## Feature Comparison

<div class="comparison-matrix">

| Feature | fqc | gzip | zstd | CRAM | DSRC 2 | Spring |
|---------|:---:|:----:|:----:|:----:|:------:|:------:|
| Random Access | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-partial">&#9651;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| Domain-Aware | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> |
| Streaming Mode | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> |
| Rust Native | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| Zero unsafe | <span class="feature-check">&#10003;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| Lossy Quality | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> | <span class="feature-cross">&#10007;</span> |
| Paired-End | <span class="feature-check">&#10003;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-na">&mdash;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> |
| Single Binary | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-check">&#10003;</span> | <span class="feature-cross">&#10007;</span> |

</div>

::: tip Legend
- <span class="feature-check">&#10003;</span> Fully supported
- <span class="feature-partial">&#9651;</span> Partial/Limited support
- <span class="feature-cross">&#10007;</span> Not supported
- <span class="feature-na">&mdash;</span> Not applicable (general-purpose tool)
:::

## Academic References

### FASTQ Format

<ol class="reference-list">
  <li>
    <span class="ref-number">[1]</span>
    <span class="ref-authors">Cock, P.J.A., Fields, C.J., Goto, N., Heuer, M.L. &amp; Rice, P.M.</span>
    <span class="ref-title">"The Sanger FASTQ file format for sequences with quality scores, and the Solexa/Illumina FASTQ variant."</span>
    <span class="ref-journal">Nucleic Acids Research</span>, 38(6), 1767&ndash;1771 (2010).
    <a href="https://doi.org/10.1093/nar/gkp1137" class="ref-link">DOI</a>
  </li>
</ol>

### FASTQ Compression Tools

<ol class="reference-list" start="2">
  <li>
    <span class="ref-number">[2]</span>
    <span class="ref-authors">Hsi-Yang Fritz, M., Leinonen, R., Cochrane, G. &amp; Birney, E.</span>
    <span class="ref-title">"Efficient storage of high throughput DNA sequencing data using reference-based compression."</span>
    <span class="ref-journal">Genome Research</span>, 21(5), 734&ndash;740 (2011).
    <a href="https://doi.org/10.1101/gr.114819.110" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; CRAM format</span>
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
    <span class="ref-authors">Deorowicz, S., Grabowski, S., Robel, P. &amp; Debudaj-Grabysz, A.</span>
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
    <span class="ref-authors">Benoit, G., Lavenier, D., Drezen, E. &amp; Rizk, G.</span>
    <span class="ref-title">"Leon: lossless and reference-free compression of FASTQ data."</span>
    <span class="ref-journal">BMC Bioinformatics</span>, 16, S3 (2015).
    <a href="https://doi.org/10.1186/1471-2105-16-S5-S3" class="ref-link">DOI</a>
  </li>
</ol>

### General Compression Theory

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

### Information Theory & Foundations

<ol class="reference-list" start="11">
  <li>
    <span class="ref-number">[11]</span>
    <span class="ref-authors">Shannon, C.E.</span>
    <span class="ref-title">"A mathematical theory of communication."</span>
    <span class="ref-journal">Bell System Technical Journal</span>, 27(3), 379&ndash;423 (1948).
    <a href="https://doi.org/10.1002/j.1538-7305.1948.tb01338.x" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; Shannon entropy, fundamental compression limits</span>
  </li>
  <li>
    <span class="ref-number">[12]</span>
    <span class="ref-authors">Yu, Z., Novak, A.M., Yee, M.C. &amp; Schatz, M.C.</span>
    <span class="ref-title">"Quality score compression improves genotyping accuracy."</span>
    <span class="ref-journal">Nature Biotechnology</span>, 38, 1184&ndash;1188 (2020).
    <a href="https://doi.org/10.1038/s41587-020-0552-1" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; Lossy quality compression and downstream analysis impact</span>
  </li>
  <li>
    <span class="ref-number">[13]</span>
    <span class="ref-authors">Ferragina, P. &amp; Venturini, R.</span>
    <span class="ref-title">"Compressed cache-oblivious string B-tree."</span>
    <span class="ref-journal">Theoretical Computer Science</span>, 412(29), 3555&ndash;3568 (2011).
    <a href="https://doi.org/10.1016/j.tcs.2011.02.023" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; Compression-access trade-off theory</span>
  </li>
  <li>
    <span class="ref-number">[14]</span>
    <span class="ref-authors">Manzini, G.</span>
    <span class="ref-title">"An analysis of the Burrows-Wheeler transform."</span>
    <span class="ref-journal">Journal of the ACM</span>, 48(3), 407&ndash;430 (2001).
    <a href="https://doi.org/10.1145/382780.382782" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; BWT theory and its compression properties</span>
  </li>
  <li>
    <span class="ref-number">[15]</span>
    <span class="ref-authors">Koslicki, D. &amp; Falush, D.</span>
    <span class="ref-title">"Introduction to compression strategies in Bioinformatics."</span>
    <span class="ref-journal">Briefings in Bioinformatics</span>, 13(3), 305&ndash;313 (2012).
    <a href="https://doi.org/10.1093/bib/bbr073" class="ref-link">DOI</a>
    <span class="ref-note">&mdash; Survey of DNA compression techniques</span>
  </li>
</ol>

## Related Open Source Projects

| Project | Language | Description | Relevance to fqc |
|---------|----------|-------------|-----------------|
| [zstd-rs](https://github.com/gyscos/zstd-rs) | Rust | Rust bindings for Zstandard | fqc's compression backend |
| [criterion](https://github.com/bheisler/criterion.rs) | Rust | Statistics-driven benchmarking | fqc's benchmark framework |
| [seq_io](https://github.com/markschl/seq_io) | Rust | FASTA/FASTQ parsing library | Alternative parser design |
| [Spring](https://github.com/shubhamchandak94/Spring) | C++ | Reference-based FASTQ compressor | Primary competitor |
| [DSRC](https://github.com/refresh-bio/DSRC) | C++ | FASTQ compression library | Reference-free competitor |
| [seqtk](https://github.com/lh3/seqtk) | C | FASTQ toolkit | Lightweight utility comparison |

## How fqc Differs

fqc occupies a distinct position in the FASTQ compression landscape:

1. **Block-indexed random access**: Unlike stream-based compressors (DSRC, FaStore), fqc's self-contained block index enables O(log N) lookup without sidecar files.

2. **Component-specific encoding**: Each FASTQ component uses an independently tuned codec, unlike unified-record approaches.

3. **Three execution modes**: Archive, Streaming, and Pipeline modes produce identical output, letting users trade memory for compression ratio without format fragmentation.

4. **Operational tooling**: The same binary provides compress, decompress, info, and verify commands &mdash; no sidecar scripts needed.

5. **ABC algorithm**: A domain-specific short-read compression algorithm that exploits biological read similarity through contig building and delta encoding.

6. **Memory safety**: Rust's compile-time guarantees eliminate the memory vulnerability class common in bioinformatics C/C++ tools.

For a detailed competitive analysis, see [Competitive Analysis](../comparison).
