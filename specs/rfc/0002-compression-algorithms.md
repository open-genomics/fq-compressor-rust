# RFC-0002: Compression Algorithms

**Status**: ✅ Accepted  
**Proposed**: 2024-01-15  
**Accepted**: 2024-02-01

## Summary

This RFC specifies the compression algorithms used in fqc: ABC for short reads, Zstd for medium/long reads, and SCM for quality scores.

## Motivation

FASTQ data has unique characteristics that domain-specific algorithms can exploit:
- DNA sequences have limited alphabet (A, C, G, T, N)
- Short reads from same sample have high similarity
- Quality scores have contextual patterns
- Read IDs follow repetitive patterns

## Design Details

### ABC Algorithm (Short Reads < 300bp)

**Consensus-Based Compression**:

1. **Consensus Computation**:
   - For each position, find most common base across all reads
   - Build consensus sequence representing population

2. **Delta Encoding**:
   - Encode each read as differences from consensus
   - Delta values are typically small (0-2 bits per position)
   - Run-length encode deltas for further compression

3. **Block Processing**:
   - Group reads into blocks (configurable size)
   - Each block has its own consensus
   - Block boundaries aligned for random access

**Implementation**: `src/algo/block_compressor.rs`

### Zstd Compression (Medium/Long Reads ≥ 300bp)

**Rationale**:
- ABC becomes less effective for longer reads
- Zstd excels at general-purpose compression
- Length-prefixed encoding handles variable-length reads

**Implementation**:
- Use `zstd` crate
- Configurable compression level (1-9)
- Length prefix before compressed data

### SCM Quality Score Compression

**Statistical Context Model**:

1. **Order-2 Model** (Short Reads):
   - Context: previous 2 quality values
   - Arithmetic coding based on conditional probability
   - Exploits local correlation in quality scores

2. **Order-1 Model** (Long Reads):
   - Context: previous 1 quality value
   - Lower memory overhead for large reads
   - Still exploits sequential correlation

3. **Illumina 8-Bin Mode**:
   - Quantize quality scores to 8 bins
   - Reduces entropy before SCM encoding
   - Follows Illumina binning convention

4. **Discard Mode**:
   - Omit quality scores entirely
   - Smallest output for applications that don't need quality

**Implementation**: `src/algo/quality_compressor.rs`

### ID Compression

**Tokenization + Delta Encoding**:

1. **Pattern Detection**:
   - Identify repetitive ID components (e.g., `@HWI-ST1234:1:1101:`)
   - Tokenize common prefixes/suffixes

2. **Delta Encoding**:
   - Numeric components delta-encoded
   - Variable-length integers for compact storage

3. **Modes**:
   - **Exact**: Preserve full ID
   - **StripComment**: Remove comments after space
   - **Discard**: Omit IDs entirely (for ID-less applications)

**Implementation**: `src/algo/id_compressor.rs`

### Paired-End Optimization

**Complementarity Exploitation**:

1. **Detection**:
   - Identify paired-end data from flags
   - Match R1/R2 pairs by ID

2. **Optimization**:
   - Store R1 fully
   - Store R2 as complement of R1 where applicable
   - Handle overlapping regions specially

3. **Layout Support**:
   - **Interleaved**: R1, R2, R1, R2 in single file
   - **Consecutive**: All R1s, then all R2s

**Implementation**: `src/algo/pe_optimizer.rs`

### Global Read Reordering

**Minimizer-Based Reordering**:

1. **Minimizer Computation**:
   - For each read, compute minimizers (canonical k-mers)
   - Minimizer represents read's "signature"

2. **Reordering**:
   - Group reads with similar minimizers together
   - Increases locality → better compression
   - Applied only to short reads (ABC algorithm)

3. **Bidirectional Map**:
   - Forward map: original → archive order
   - Reverse map: archive → original order
   - ZigZag delta + varint encoding for compactness

**Implementation**: `src/algo/global_analyzer.rs`, `src/reorder_map.rs`

## Performance Targets

| Algorithm | Target Ratio | Target Speed |
|-----------|--------------|--------------|
| ABC (short) | ≥ 4x | ≥ 10 MB/s |
| Zstd (long) | ≥ 3x | ≥ 50 MB/s |
| SCM quality | ≥ 2x | ≥ 20 MB/s |
| ID compression | ≥ 3x | ≥ 100 MB/s |

## Alternatives Considered

### Use Only Zstd
- **Rejected**: Domain-specific ABC achieves 20-30% better ratios for short reads
- Zstd doesn't exploit DNA-specific patterns

### Reference-Based Compression
- **Rejected**: Requires reference genome; FQC is reference-free
- Reference-based is better suited for alignment workflows (CRAM)

## References

- [Spring algorithm](https://github.com/shubhamchandak94/Spring)
- [ABC paper: "FASTQ Compression" (2014)](https://web.archive.org/web/20140302093354/http://www.ebi.ac.uk/~swah/FastqCompression.pdf)
- [SCM arithmetic coding](https://en.wikipedia.org/wiki/Arithmetic_coding)
