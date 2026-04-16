# Glossary

## A

### ABC
**Alignment-Based Compression** — A compression algorithm for short DNA reads that exploits high sequence similarity via consensus building and delta encoding.

### Archive
The `.fqc` file produced by fqc containing compressed FASTQ data in a block-indexed binary format.

## B

### Block
A fixed-size unit of compressed data within an FQC archive. Each block can be independently compressed and decompressed, enabling random access.

### Block Index
Metadata stored at the end of an FQC archive that maps block IDs to file offsets, enabling random access to specific blocks.

## C

### Consensus
In ABC compression, a representative sequence derived from a cluster of aligned reads. Used as the reference for delta encoding.

### Context Model
In SCM quality compression, a statistical model that predicts the next quality value based on previous values (order-1 or order-2).

## D

### Delta Encoding
A compression technique that stores only the differences between data and a reference (e.g., consensus sequence).

## F

### FASTQ
A text-based file format for storing nucleotide sequences and their corresponding quality scores. Standard format for sequencing data.

### FQC
The binary archive format used by fqc. Block-indexed with magic header, global header, compressed blocks, and footer.

## I

### Interleaved
A paired-end FASTQ format where R1 and R2 reads alternate in a single file.

## L

### Lossless
Compression that preserves all original data exactly. Default mode for fqc.

### Lossy
Compression that may alter data to achieve better ratios. In fqc, applies to quality scores via `--lossy-quality` option.

## M

### Minimizer
The smallest k-mer (canonical form) in a read. Used for global read reordering to improve compression ratio.

### MSRV
**Minimum Supported Rust Version** — The oldest Rust version that can compile fqc. Currently 1.75.

## P

### Paired-End (PE)
Sequencing data where reads come in pairs (R1 and R2) from opposite ends of DNA fragments.

### Pipeline Mode
A 3-stage processing mode (Reader → Compressor → Writer) that overlaps I/O with computation for higher throughput.

## Q

### Quality Score
A Phred-scale value indicating the confidence of each base call in sequencing data. Typically encoded as ASCII characters.

### Quantization
Reducing the precision of quality scores to a smaller set of values. Illumina8Bin reduces to 8 representative values.

## R

### Random Access
The ability to decompress specific portions of an archive without processing the entire file. Enabled by block indexing.

### Reorder Map
A bidirectional mapping stored in FQC archives that records how reads were reordered during compression, allowing restoration of original order.

## S

### SCM
**Statistical Context Model** — A compression algorithm for quality scores using context-based probability modeling and arithmetic coding.

### Streaming Mode
Processing mode that reads and compresses data sequentially without global reordering. Suitable for stdin/pipe input.

## Z

### Zstd
**Zstandard** — A fast compression algorithm used by fqc for medium and long reads, as well as final compression of ABC-encoded blocks.
