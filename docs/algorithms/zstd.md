# Zstd Integration for Long Reads

This document describes how fqc integrates the Zstandard (Zstd) compression library for handling medium and long reads, and as a secondary compression layer for other algorithms.

## Overview

Zstd is used in fqc in two roles:

1. **Primary compressor** for medium and long reads (>= 300bp), where the ABC algorithm is less effective
2. **Secondary compressor** that post-processes the output of other algorithms (ABC, SCM, ID compression) to capture residual patterns

Zstd was chosen for its excellent speed-to-ratio trade-off, streaming support, and permissive BSD license.

## Zstd in the Compression Pipeline

### As Primary Compressor (Sequence Data)

For medium and long reads, sequences are compressed directly with Zstd:

```rust
fn compress_sequences_zstd(reads: &[ReadRecord], zstd_level: i32) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();

    // Length-prefixed encoding for variable-length reads
    for read in reads {
        let seq = read.sequence.as_bytes();
        buf.write_u32::<LittleEndian>(seq.len() as u32)?;
        buf.extend_from_slice(seq);
    }

    let compressed = zstd::bulk::compress(&buf, zstd_level)
        .map_err(|e| FqcError::Compression(format!("Zstd compress: {e}")))?;
    Ok(compressed)
}
```

The **length-prefix** format enables decompression to reconstruct individual reads:

```
+------------------+
| Read 0 Length    |  u32 (little-endian)
+------------------+
| Read 0 Sequence  |  length bytes
+------------------+
| Read 1 Length    |  u32
+------------------+
| Read 1 Sequence  |  length bytes
+------------------+
| ...              |
+------------------+
```

### As Secondary Compressor

Zstd compresses the output of other encoders:

| Primary Encoder | Zstd Role |
|-----------------|-----------|
| ABC (contigs) | Compresses serialized contig binary format |
| SCM (quality) | Compresses arithmetic-coded quality bitstream |
| ID compression | Compresses tokenized/varint-encoded IDs |
| Aux stream | Compresses delta-varint encoded lengths |

```
ABC contigs ──→ Serialize ──→ Zstd ──→ Block payload
SCM quality  ──→ Arithmetic encode ──→ Zstd ──→ Block payload
IDs          ──→ Tokenize/delta ──→ Zstd ──→ Block payload
```

## Compression Level Mapping

User-facing compression levels (1-9) are mapped to Zstd levels:

```rust
pub fn zstd_level_for_compression_level(level: CompressionLevel) -> i32 {
    match level {
        1..=2 => 1,   // Fast
        3..=4 => 3,   // Balanced
        5..=6 => 5,   // Default
        7..=8 => 9,   // High
        _     => 15,  // Maximum (level 9 user → zstd 15)
    }
}
```

| User Level | Zstd Level | Use Case |
|------------|------------|----------|
| 1 | 1 | Maximum speed, lowest ratio |
| 2 | 1 | Fast |
| 3 | 3 | Good speed/ratio |
| 4 | 3 | Good |
| 5 | 5 | Default |
| 6 | 5 | Default |
| 7 | 9 | High compression |
| 8 | 9 | High compression |
| 9 | 15 | Maximum compression |

Note: Zstd supports levels up to 22, but fqc caps at 15 for reasonable compression speed.

### Secondary Compression Levels

For secondary compression (ABC serialization, SCM output, ID compression), a **fixed level of 3** is used:

```rust
// ABC final compression
zstd::bulk::compress(&buf, zstd_level)  // Uses user-configured level

// ABC contig serialization (inside compress_sequences_abc)
zstd::bulk::compress(&buf, zstd_level)  // Same level

// Quality SCM output
zstd::bulk::compress(&encoded, 3)  // Fixed level 3

// Reorder map compression
zstd::bulk::compress(&forward_encoded, 3)  // Fixed level 3
zstd::bulk::compress(&reverse_encoded, 3)  // Fixed level 3
```

The fixed level of 3 for secondary compression provides a good balance without dominating the overall compression time.

## Decompression

Decompression uses Zstd's streaming decoder for memory efficiency:

```rust
// Block-based decompression
let buf = zstd::stream::decode_all(data)
    .map_err(|e| FqcError::Decompression(format!("Zstd decompress: {e}")))?;
```

For sequence decompression with Zstd:

```rust
fn decompress_sequences_zstd(
    data: &[u8],
    read_count: u32,
    uniform_read_length: u32,
    lengths: &[u32],
) -> Result<Vec<String>> {
    let buf = zstd::stream::decode_all(data)?;

    let mut sequences = Vec::with_capacity(read_count as usize);
    let mut offset = 0;

    for i in 0..read_count as usize {
        let len = if uniform_read_length > 0 {
            uniform_read_length as usize
        } else {
            lengths.get(i).copied().unwrap_or(0) as usize
        };

        if offset + len > buf.len() {
            return Err(FqcError::Format("Truncated sequence data".to_string()));
        }

        sequences.push(String::from_utf8_lossy(&buf[offset..offset + len]).into_owned());
        offset += len;
    }
    Ok(sequences)
}
```

## Codec Identification

When Zstd is used as the primary sequence compressor, the block header codec byte is set to `ZstdPlain` (0x70):

```rust
pub fn get_sequence_codec(&self) -> u8 {
    match self.read_length_class {
        ReadLengthClass::Short => encode_codec(CodecFamily::AbcV1, 0),   // 0x10
        _ => encode_codec(CodecFamily::ZstdPlain, 0),                     // 0x70
    }
}
```

The decompressor checks this codec to decide whether to use ABC or Zstd decompression:

```rust
let seq_codec_family = decode_codec_family(codec_seq);
let sequences = if seq_codec_family == CodecFamily::AbcV1 {
    decompress_sequences_abc(seq_stream, read_count)?
} else {
    decompress_sequences_zstd(seq_stream, read_count, uniform_read_length, &lengths)?
};
```

## Why Zstd for Long Reads?

Zstd is preferred for medium and long reads over ABC for several reasons:

| Factor | ABC | Zstd |
|--------|-----|------|
| Effectiveness at 300bp+ | Decreases (fewer reads align well) | Maintains good ratio |
| Speed | Moderate (alignment overhead) | Fast |
| Memory | O(block_size × read_length) | Efficient streaming |
| Implementation complexity | High | Simple (library call) |
| Multi-megabase reads | Not supported (max ~65KB with u16, ~4GB with u32) | No practical limit |

ABC's consensus approach becomes less effective as read length increases because:
1. Longer reads have more opportunity for mutations/structural variants
2. Fewer reads align to the same consensus, creating many small contigs
3. The alignment search (O(max_shift × L)) becomes expensive
4. Delta encoding captures less of the read's information

Zstd, being a general-purpose dictionary-based compressor, handles long sequences well and can find repeated patterns across reads within the compression window.

## Zstd Dependency

Zstd is a **mandatory** dependency (not feature-gated):

```toml
[dependencies]
zstd = "0.13"
```

The `zstd` crate provides:

| API | Usage in fqc |
|-----|--------------|
| `zstd::bulk::compress()` | Block-based compression |
| `zstd::stream::decode_all()` | Block-based decompression |
| `zstd::bulk::compress_to_buffer()` | Not used |
| `zstd::stream::Decoder` | Not used (streaming not needed for blocks) |

### Other Compression Formats

Zstd is also the native format for `.zst` input files. Other formats (gzip, bzip2, xz) are feature-gated:

```rust
// io/compressed_stream.rs
#[cfg(feature = "gz")]   // flate2
#[cfg(feature = "bz2")]  // bzip2
#[cfg(feature = "xz")]   // xz2
```

## Performance Characteristics

### Compression Speed

| Zstd Level | Throughput (typical) | Notes |
|------------|---------------------|-------|
| 1 | 500+ MB/s | Fastest, dictionary-less |
| 3 | 200-400 MB/s | Good balance |
| 5 | 100-200 MB/s | Default |
| 9 | 50-100 MB/s | High compression |
| 15 | 10-30 MB/s | Maximum |

Throughput depends heavily on data compressibility. DNA sequences typically achieve 2-5x compression with Zstd alone.

### Compression Ratio for FASTQ Data

| Data Type | Zstd Ratio (level 3) | ABC Ratio | Combined (ABC + Zstd) |
|-----------|----------------------|-----------|------------------------|
| Illumina 150bp sequences | ~3x | ~5x | ~8x |
| Illumina 250bp sequences | ~3.5x | ~4x | ~7x |
| PacBio HiFi 15KB | ~3x | N/A | ~3x (Zstd only) |
| Nanopore 100KB | ~2.5x | N/A | ~2.5x (Zstd only) |

The "combined" column reflects ABC encoding followed by Zstd post-compression.

### Memory Usage

Zstd's memory usage depends on the compression level:

| Level | Compression Memory | Decompression Memory |
|-------|-------------------|---------------------|
| 1 | ~1 MB | ~64 KB |
| 3 | ~2 MB | ~64 KB |
| 5 | ~4 MB | ~64 KB |
| 9 | ~16 MB | ~64 KB |
| 15 | ~128 MB | ~64 KB |

Decompression memory is constant and small regardless of compression level.

## Error Handling

Zstd errors are wrapped into `FqcError::Compression` or `FqcError::Decompression`:

```rust
let compressed = zstd::bulk::compress(&buf, level)
    .map_err(|e| FqcError::Compression(format!("Zstd compress failed: {e}")))?;

let decompressed = zstd::stream::decode_all(data)
    .map_err(|e| FqcError::Decompression(format!("Zstd decompress failed: {e}")))?;
```

These map to exit codes:
- `Compression` → Exit code 2 (I/O Error)
- `Decompression` → Exit code 3 (Format Error)

## Related Documents

- [Strategy Selection](./strategy-selection.md)
- [ABC Algorithm](./abc.md)
- [Source Module Overview](../architecture/modules.md)
- [Block Format](../architecture/block-format.md)
- [Compression Algorithms RFC](../../specs/rfc/0002-compression-algorithms.md)
