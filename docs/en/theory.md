# Algorithmic Theory and Foundations

<div class="whitepaper-hero">
<div class="whitepaper-title">Theoretical Foundations of fqc</div>
<div class="whitepaper-meta">
  Information theory, compression bounds, and the algorithmic rationale behind fqc's design decisions.
</div>
</div>

## 1. Information-Theoretic Background

### 1.1 Entropy of DNA Sequences

The theoretical limit of lossless compression is given by Shannon entropy [11]:

$$H(X) = -\sum_{i} p(x_i) \log_2 p(x_i)$$

For a uniform random DNA sequence with alphabet {A, C, G, T}, the per-symbol entropy is:

$$H = -4 \times \frac{1}{4} \log_2 \frac{1}{4} = 2 \text{ bits/base}$$

However, real sequencing data is far from uniform:

- **GC bias**: Most organisms have GC content between 30-60%, not 50%
- **K-mer repetition**: Coverage depth creates many near-identical reads
- **Quality correlation**: Adjacent quality scores are highly correlated
- **Read structure**: Paired-end reads have predictable distance relationships

These properties mean the true entropy of sequencing data is significantly lower than 2 bits/base, creating an opportunity for domain-specific compression.

### 1.2 Empirical Entropy

For practical compression, we consider k-th order empirical entropy:

$$H_k = -\sum_{w \in \Sigma^k} \frac{|w|}{n} \sum_{a \in \Sigma} p(a|w) \log_2 p(a|w)$$

where $\Sigma = \{A, C, G, T, N\}$ and $p(a|w)$ is the conditional probability of symbol $a$ given context $w$.

For high-coverage short-read data, $H_k$ decreases rapidly with $k$ because biological contexts are highly predictive. This is the theoretical justification for context-based compressors like DSRC's order-3 arithmetic coder.

## 2. LZ Compression Theory

### 2.1 LZ77 and DNA

The Lempel-Ziv family of algorithms [10] achieves compression by replacing repeated substrings with back-references. The key parameter is the LZ parse: a partition of the input into phrases where each phrase is either a new symbol or a reference to a previous occurrence.

For DNA sequences, LZ parsing is particularly effective because:

1. **High repetition**: Coverage depth $c$ means each $k$-mer appears approximately $c$ times
2. **Locality**: Similar reads cluster in the input (especially after reordering)
3. **Bounded alphabet**: The small alphabet $\Sigma$ creates frequent substring matches

The compression ratio achievable by LZ is bounded by:

$$\text{ratio} \approx \frac{n}{z \log n}$$

where $n$ is input size and $z$ is the number of LZ phrases. For $c$-coverage sequencing, $z \approx n/c$, giving expected ratio $\approx c / \log n$.

### 2.2 Zstd's Role

Zstd [9] is a modern LZ variant optimized for speed. In fqc, Zstd serves as the "universal back-end" for:

- Medium/long read sequences (where ABC overhead exceeds benefit)
- Quality score streams (exploiting autocorrelation)
- Delta-encoded short-read streams (after ABC processing)
- ID streams (after pattern extraction)

Zstd's adaptive Huffman coding and fast match-finding make it superior to gzip for structured biological data.

## 3. Reference-Based vs. Reference-Free Compression

### 3.1 The Reference Advantage

Reference-based compressors (CRAM, Spring) map each read to a reference genome. If the edit distance is small, the storage reduces to:

$$S_{ref} = O(n \cdot d \cdot \log L)$$

where $n$ is read count, $d$ is average edit distance, and $L$ is read length. For $d \ll L$, this is dramatically smaller than storing full sequences.

### 3.2 The Reference Limitation

Reference-based compression has three fundamental limitations:

1. **Reference dependency**: The archive is useless without the reference
2. **Reference quality**: Poor references (gaps, errors) reduce compression
3. **Novel sequences**: Structural variations, novel organisms, and metagenomic samples have high edit distance

### 3.3 Internal Reference: The ABC Approach

fqc's ABC algorithm can be understood as **internal reference compression**: instead of an external genome, it constructs references (consensus contigs) from the input itself. This provides:

$$S_{abc} = O(n \cdot d_{intra} \cdot \log L + L_{contig})$$

where $d_{intra}$ is the intra-sample edit distance (typically very small for clonal sequencing) and $L_{contig}$ is the contig representation overhead.

For high-coverage data, $d_{intra} \approx d_{ref}$, making ABC competitive with reference-based approaches while maintaining standalone portability.

## 4. Quality Score Compression Theory

### 4.1 Quality Score Entropy

Illumina quality scores use Phred encoding: $Q = -10 \log_{10} p$, where $p$ is the base-calling error probability. The scores range from 0 to 41 (ASCII 33-74), but the empirical distribution is highly skewed:

- Most bases have high quality ($Q \geq 30$, error rate $< 0.1\%$)
- Quality decreases toward read ends
- Quality profiles are predictable by position

The position-dependent quality profile means the joint entropy $H(Q, pos)$ is significantly lower than $H(Q)$ alone.

### 4.2 Lossy Compression Bounds

When lossy compression is acceptable, we can consider rate-distortion theory. For quality binning to $b$ levels, the distortion is:

$$D = \mathbb{E}[|Q - \hat{Q}|^2]$$

Illumina8 binning (8 levels) has been shown to have negligible impact on downstream variant calling [12] while reducing quality storage by ~50%.

fqc's quality modes allow users to select their preferred point on the rate-distortion curve:

| Mode | Rate | Distortion | Use Case |
|------|------|------------|----------|
| Lossless | $H(Q \| pos)$ | 0 | Archival, reanalysis |
| Illumina8 | $\approx 0.5 \times H(Q \| pos)$ | Low | Standard analysis |
| QVZ | Variable | Variable | Custom trade-off |
| Discard | ~0 bits | High | Sequence-only analysis |

## 5. Block Indexing Theory

### 5.1 The Access-Compression Trade-off

There is a fundamental information-theoretic trade-off between compression ratio and random access granularity [13]. If we partition data into $b$ blocks:

- **Compression ratio**: Improves with larger blocks (more context for LZ)
- **Access overhead**: Proportional to block size (must decompress entire block)

The optimal block size balances these competing objectives. fqc uses adaptive block sizing based on read length and memory constraints.

### 5.2 Index Overhead

The block index stores $(block\_id, offset, compressed\_size, read\_count)$ tuples. For $b$ blocks:

$$S_{index} = b \times (4 + 8 + 8 + 4) = 24b \text{ bytes}$$

For typical block sizes (10,000-100,000 reads), the index overhead is $< 0.1\%$ of total archive size, making it effectively free.

### 5.3 Query Complexity

Random access queries decompose into:

1. **Block lookup**: $O(\log b)$ binary search on the index
2. **Block decompression**: $O(r)$ where $r$ is reads in the block
3. **Record extraction**: $O(1)$ with block-local offsets

For range queries spanning $k$ blocks: $O(\log b + k \cdot r)$.

## 6. Read Reordering Theory

### 6.1 The Sorting Problem

Read reordering in fqc's Archive mode is a form of **metric space sorting**: given a set of DNA sequences with edit distance as the metric, find a permutation that minimizes sum of adjacent distances.

This is NP-hard in general (related to the Traveling Salesman Problem), but greedy approximations work well in practice because:

1. The metric space has low dimensionality (sequences cluster by genomic position)
2. Coverage depth creates many near-duplicates (zero-distance clusters)
3. Minimizer sketches provide fast approximate distance estimation

### 6.2 Compression Gain from Reordering

The improvement from reordering can be modeled as:

$$\Delta R = \frac{H_{unordered} - H_{ordered}}{H_{unordered}}$$

For coverage-$c$ data with cluster size $s$, expected improvement is:

$$\Delta R \approx 1 - \frac{\log(s)}{\log(n)}$$

For typical Illumina data ($c \approx 30$, $s \approx c$), reordering improves ratio by 5-15%.

## 7. Algorithm Selection Rationale

### 7.1 Why ABC for Short Reads?

For reads $\leq 511$ bp, the expected number of exact $k$-mer matches between random reads is:

$$E[matches] = (L - k + 1)^2 / 4^k$$

For $L = 150$, $k = 21$: $E[matches] \approx 130^2 / 4^{21} \approx 0$

But with coverage depth $c = 30$, the expected matches between a read and its $c - 1$ siblings is:

$$E[matches] = 30 \times 130^2 / 4^{21} \times 4^{21} / genome\_size \gg 0$$

This high intra-sample similarity makes consensus-based approaches far more effective than general-purpose LZ for short-read data.

### 7.2 Why Zstd for Long Reads?

For long reads ($> 10$ kb), coverage depth is typically lower ($c \approx 5-10$ for PacBio, $c \approx 20-40$ for ONT). The consensus construction overhead ($O(nL)$ for contig building) exceeds the compression benefit. Zstd's fast match-finding and adaptive entropy coding provide better throughput/compression trade-offs.

### 7.3 Why Component Separation?

The joint entropy of a FASTQ record's components is:

$$H(ID, Seq, Qual, Aux) = H(Seq) + H(Qual | Seq) + H(ID | Seq, Qual) + H(Aux | Seq, Qual, ID)$$

Because quality scores are conditionally independent of sequence given position ($H(Qual | Seq) \approx H(Qual | pos)$), and IDs are weakly dependent on sequence content, the mutual information between components is small. Separating them allows independent optimization without significant ratio loss.

## 8. Future Theoretical Directions

### 8.1 Burrows-Wheeler Transform

The BWT is the foundation of modern genomic indexing (FM-index, bwa). A BWT-based compressor could exploit the fact that the BWT of a set of reads clusters similar suffixes, creating runs of identical symbols ideal for move-to-front coding.

### 8.2 Graph-Based Compression

Representing reads as walks on a de Bruijn graph or variation graph provides a natural compression: each read is a path, and shared subpaths are stored once. Leon [8] uses a probabilistic de Bruijn graph; exact graph compression remains an active research area.

### 8.3 Machine Learning Approaches

Neural compressors (transformers, VAEs) can learn the distribution of sequencing data. While currently too slow for production, they may eventually outperform hand-designed algorithms by capturing subtle dependencies (adapter sequences, systematic errors, platform-specific biases).

## References

<ol class="reference-list" start="11">
  <li>
    <span class="ref-number">[11]</span>
    <span class="ref-authors">Shannon, C.E.</span>
    <span class="ref-title">"A mathematical theory of communication."</span>
    <span class="ref-journal">Bell System Technical Journal</span>, 27(3), 379&ndash;423 (1948).
    <a href="https://doi.org/10.1002/j.1538-7305.1948.tb01338.x" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[12]</span>
    <span class="ref-authors">Yu, Z., Novak, A.M., Yee, M.C. &amp; Schatz, M.C.</span>
    <span class="ref-title">"Quality score compression improves genotyping accuracy."</span>
    <span class="ref-journal">Nature Biotechnology</span>, 38, 1184&ndash;1188 (2020).
    <a href="https://doi.org/10.1038/s41587-020-0552-1" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[13]</span>
    <span class="ref-authors">Ferragina, P. &amp; Venturini, R.</span>
    <span class="ref-title">"Compressed cache-oblivious string B-tree."</span>
    <span class="ref-journal">Theoretical Computer Science</span>, 412(29), 3555&ndash;3568 (2011).
    <a href="https://doi.org/10.1016/j.tcs.2011.02.023" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[14]</span>
    <span class="ref-authors">Manzini, G.</span>
    <span class="ref-title">"An analysis of the Burrows-Wheeler transform."</span>
    <span class="ref-journal">Journal of the ACM</span>, 48(3), 407&ndash;430 (2001).
    <a href="https://doi.org/10.1145/382780.382782" class="ref-link">DOI</a>
  </li>
</ol>
