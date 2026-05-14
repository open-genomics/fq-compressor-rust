# References & Related Work

Academic references and related tools in FASTQ compression. This page provides context for the design decisions behind fqc.

## FASTQ Format

- Cock, P.J.A., Fields, C.J., Goto, N., Heuer, M.L. & Rice, P.M. (2010). "The Sanger FASTQ file format for sequences with quality scores, and the Solexa/Illumina FASTQ variant." *Nucleic Acids Research*, 38(6), 1767–1771. [DOI:10.1093/nar/gkp1137](https://doi.org/10.1093/nar/gkp1137)

## FASTQ Compression Tools

| Tool | Approach | Random Access | Lossy Quality | Paired-End | Year |
|------|----------|:---:|:---:|:---:|:---:|
| **fqc** | Block-indexed, ABC + Zstd | Yes | Yes | Yes | 2025 |
| **CRAM** | Reference-based, range coding | Limited | Yes | Yes | 2011 |
| **DSRC** | Stream-based, LZ77 variant | No | No | No | 2011 |
| **DSRC 2** | Stream-based, improved LZ77 | No | No | Yes | 2013 |
| **FaStore** | Hybrid, delta + LZ77 | No | No | No | 2017 |
| **qvz** | Quality-only, context model | N/A | Yes | N/A | 2015 |
| **Spring** | Reference-based, minimizers | No | No | Yes | 2019 |
| **Minicom** | Reference-based, BOSS | No | No | No | 2018 |
| **Leon** | Reference-free, de Bruijn graph | No | No | No | 2015 |

### Tool References

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

## General Compression

- Collet, Y. & Kucherawy, M. (2018). "Zstandard – Real-time data compression algorithm." *IETF RFC 8478*. [RFC 8478](https://datatracker.ietf.org/doc/html/rfc8478)
- Ziv, J. & Lempel, A. (1977). "A universal algorithm for sequential data compression." *IEEE Transactions on Information Theory*, 23(3), 337–343. [DOI:10.1109/TIT.1977.1055714](https://doi.org/10.1109/TIT.1977.1055714)
- Ziv, J. & Lempel, A. (1978). "Compression of individual sequences via variable-rate coding." *IEEE Transactions on Information Theory*, 24(5), 530–536. [DOI:10.1109/TIT.1978.1055934](https://doi.org/10.1109/TIT.1978.1055934)

## Bioinformatics Standards

- Li, H. et al. (2009). "The Sequence Alignment/Map format and SAMtools." *Bioinformatics*, 25(16), 2078–2079. [DOI:10.1093/bioinformatics/btp352](https://doi.org/10.1093/bioinformatics/btp352)
- GA4GH (2023). "Global Alliance for Genomics and Health – Data Standards." [ga4gh.org](https://www.ga4gh.org/our-products/)

## How fqc Differs

fqc occupies a distinct position in the FASTQ compression landscape:

1. **Block-indexed random access**: Unlike stream-based compressors (DSRC, FaStore), fqc's block index enables O(log N) lookup without decompressing the entire archive.
2. **Component-specific encoding**: Each FASTQ component (ID, sequence, quality, aux) uses an independently tuned codec, unlike unified-record approaches.
3. **Three execution modes**: Archive, Streaming, and Pipeline modes produce identical output, letting users trade memory for compression ratio without format fragmentation.
4. **Operational tooling**: The same binary provides compress, decompress, info, and verify commands — no sidecar scripts needed.
5. **ABC algorithm**: A domain-specific short-read compression algorithm that exploits biological read similarity through contig building and delta encoding.
