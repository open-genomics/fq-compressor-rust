# CONTEXT.md

Domain language for the fqc (FASTQ compressor) project.

## Core Domain Concepts

### FASTQ

A text-based format for storing nucleotide sequences and quality scores. Each read has four lines:
1. ID line (starts with `@`)
2. Sequence line (DNA bases: A, C, G, T, N)
3. Plus line (starts with `+`)
4. Quality line (ASCII-encoded quality scores)

### Read

A single sequencing read. In fqc, represented as `ReadRecord` with:
- `id`: The read identifier
- `comment`: Optional comment after the ID
- `sequence`: DNA sequence string
- `quality`: Quality score string

### Block

A group of reads compressed together. Blocks enable:
- Random access via block index
- Parallel compression/decompression
- Memory-bounded processing

### Contig

In ABC compression, a group of similar reads that share a consensus sequence. Each read in a contig is delta-encoded against the consensus.

## Compression Concepts

### ABC (Anchor-Based Compression)

Algorithm for short reads (≤511 bp) that:
1. Groups similar reads into contigs
2. Builds consensus sequences
3. Delta-encodes reads against consensus
4. Compresses deltas with Zstd

Best for: Short reads with high similarity (typical Illumina data).

### Zstd Compression

General-purpose compression for sequences that don't fit ABC:
- Medium reads (>511 bp, <10 KB)
- Long reads (≥10 KB)

### SCM (Statistical Compression Model)

Quality score compression using:
- Adaptive arithmetic coding
- Context modeling (order 1 or 2)
- Position binning

### Read Length Class

Categorization of reads affecting compression strategy:
- **Short**: ≤511 bp (ABC eligible)
- **Medium**: 512 bp to 10 KB (Zstd)
- **Long**: >10 KB (Zstd with different settings)

## Codec Concepts

### Codec Family

Identifier for compression algorithm used on each stream:
- `AbcV1`: ABC algorithm
- `ZstdPlain`: Raw Zstd compression
- `ScmV1`/`ScmOrder1`: Quality compression
- `DeltaZstd`: Delta-encoded + Zstd (for IDs)
- `DeltaVarint`: Varint-encoded deltas (for lengths)

### Stream

Compressed data for one aspect of a block:
- `id_stream`: Compressed read IDs
- `seq_stream`: Compressed sequences
- `qual_stream`: Compressed quality scores
- `aux_stream`: Compressed auxiliary data (lengths)

## Archive Concepts

### Global Header

Archive metadata including:
- Magic number
- Version
- Flags (paired-end, read length class, quality mode, etc.)
- Block index offset

### Block Header

Per-block metadata including:
- Block ID
- Read count
- Codec identifiers
- Stream offsets and sizes
- Checksums

### Block Index

Table mapping block IDs to file offsets, enabling random access.

### Reorder Map

Bidirectional mapping between original read order and compressed order. Used when reads are reordered for better compression (minimizer-based bucketing).

## Processing Concepts

### Minimizer

Short k-mer extracted from each read for similarity-based grouping. Reads sharing minimizers are placed nearby for better compression.

### Delta Encoding

Representing a value as the difference from a reference. Used in:
- ABC: reads vs consensus
- IDs: sequential ID deltas
- Lengths: consecutive length deltas

### Noise Character

In ABC, a compact encoding of a mismatch:
- '0'-'3' represent specific substitutions
- Decoded using reference base + noise character

## Module Architecture

The codebase is organized into layers:

1. **Commands** (`src/commands/`): CLI orchestration
2. **Pipeline** (`src/pipeline/`): Parallel processing stages
3. **Algorithms** (`src/algo/`): Compression algorithms
   - `abc`: Anchor-Based Compression
   - `block_compressor`: Block-level orchestration
   - `quality_compressor`: SCM for quality scores
   - `id_compressor`: ID tokenization/encoding
   - `global_analyzer`: Minimizer extraction, reordering
4. **Format** (`src/format.rs`): Binary format definitions
5. **I/O** (`src/io/`): Async I/O, compressed stream detection
6. **FASTQ** (`src/fastq/`): Parsing and validation

## Compressor Traits

Each stream type (sequence, quality, ID, aux) has a compressor trait defining its interface:

- **SequenceCompressor** — compresses/decompresses DNA sequences
- **QualityCompressor** — compresses/decompresses quality scores
- **IdCompressor** — compresses/decompresses read IDs
- **AuxCompressor** — compresses/decompresses auxiliary data (lengths)

Each trait provides `compress()`, `decompress()`, and `codec_id()` methods. The `BlockCompressor` coordinates these traits, selecting the appropriate implementation based on `ReadLengthClass` and configuration.
