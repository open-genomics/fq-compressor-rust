# References & Related Work

Academic references and related tools in FASTQ compression. This page provides context for the design decisions behind fqc.

## Feature Comparison

<div class="comparison-matrix">

| Feature | fqc | gzip | zstd | CRAM | DSRC 2 | Spring |
|---------|:---:|:----:|:----:|:----:|:------:|:------:|
| Random Access | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-partial">△</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> |
| Domain-Aware | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> |
| Streaming Mode | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> |
| Rust Native | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> |
| Zero unsafe | <span class="feature-check">✓</span> | <span class="feature-na">—</span> | <span class="feature-na">—</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> |
| Lossy Quality | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> | <span class="feature-cross">✗</span> |
| Paired-End | <span class="feature-check">✓</span> | <span class="feature-na">—</span> | <span class="feature-na">—</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> |
| Single Binary | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-check">✓</span> | <span class="feature-cross">✗</span> |

</div>

::: tip Legend
- <span class="feature-check">✓</span> Fully supported
- <span class="feature-partial">△</span> Partial/Limited support
- <span class="feature-cross">✗</span> Not supported
- <span class="feature-na">—</span> Not applicable (general-purpose tool)
:::

## Academic References

### FASTQ Format

<ol class="reference-list">
  <li>
    <span class="ref-number">[1]</span>
    <span class="ref-authors">Cock, P.J.A., Fields, C.J., Goto, N., Heuer, M.L. & Rice, P.M.</span>
    <span class="ref-title">"The Sanger FASTQ file format for sequences with quality scores, and the Solexa/Illumina FASTQ variant."</span>
    <span class="ref-journal">Nucleic Acids Research</span>, 38(6), 1767–1771 (2010).
    <a href="https://doi.org/10.1093/nar/gkp1137" class="ref-link">DOI</a>
  </li>
</ol>

### FASTQ Compression Tools

<ol class="reference-list" start="2">
  <li>
    <span class="ref-number">[2]</span>
    <span class="ref-authors">Hsi-Yang Fritz, M., Leinonen, R., Cochrane, G. & Birney, E.</span>
    <span class="ref-title">"Efficient storage of high throughput DNA sequencing data using reference-based compression."</span>
    <span class="ref-journal">Genome Research</span>, 21(5), 734–740 (2011).
    <a href="https://doi.org/10.1101/gr.114819.110" class="ref-link">DOI</a>
    <span class="ref-note">— CRAM format</span>
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

### General Compression

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

## Related Open Source Projects

| Project | Language | Description |
|---------|----------|-------------|
| [zstd-rs](https://github.com/gyscos/zstd-rs) | Rust | Rust bindings for Zstandard compression |
| [criterion](https://github.com/bheisler/criterion.rs) | Rust | Statistics-driven benchmarking library |
| [seq_io](https://github.com/markschl/seq_io) | Rust | FASTA/FASTQ parsing library |
| [Spring](https://github.com/shubhamchandak94/Spring) | C++ | Reference-based FASTQ compressor |
| [DSRC](https://github.com/refresh-bio/DSRC) | C++ | FASTQ compression library |

## How fqc Differs

fqc occupies a distinct position in the FASTQ compression landscape:

1. **Block-indexed random access**: Unlike stream-based compressors (DSRC, FaStore), fqc's block index enables O(log N) lookup without decompressing the entire archive.

2. **Component-specific encoding**: Each FASTQ component (ID, sequence, quality, aux) uses an independently tuned codec, unlike unified-record approaches.

3. **Three execution modes**: Archive, Streaming, and Pipeline modes produce identical output, letting users trade memory for compression ratio without format fragmentation.

4. **Operational tooling**: The same binary provides compress, decompress, info, and verify commands — no sidecar scripts needed.

5. **ABC algorithm**: A domain-specific short-read compression algorithm that exploits biological read similarity through contig building and delta encoding.
