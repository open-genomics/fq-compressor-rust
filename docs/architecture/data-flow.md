# Data Flow

Detailed data flow through the fqc compression and decompression pipeline.

## Compression Data Flow

```
┌─────────────┐
│ FASTQ Input │
└──────┬──────┘
       │
       ▼
┌──────────────────┐
│ 1. FASTQ Parser  │
│ - Read records   │
│ - Validate       │
│ - Extract fields │
└──────┬───────────┘
       │ ReadRecord[]
       ▼
┌──────────────────┐
│ 2. Global        │
│    Analyzer      │
│ - Minimizer hash │
│ - Reorder reads  │
│ - Build map      │
└──────┬───────────┘
       │ Reordered ReadRecord[]
       ▼
┌──────────────────┐
│ 3. Block Builder │
│ - Split blocks   │
│ - Classify reads │
└──────┬───────────┘
       │ Blocks
       ▼
┌──────────────────┐
│ 4. Compressor    │
│ - ABC (short)    │
│ - Zstd (long)    │
│ - SCM (quality)  │
│ - ID compressor  │
└──────┬───────────┘
       │ Compressed blocks
       ▼
┌──────────────────┐
│ 5. FQC Writer    │
│ - Add headers    │
│ - Write blocks   │
│ - Add footer     │
└──────┬───────────┘
       │
       ▼
┌──────────────┐
│ FQC Archive  │
└──────────────┘
```

## Decompression Data Flow

```
┌──────────────┐
│ FQC Archive  │
└──────┬───────┘
       │
       ▼
┌──────────────────┐
│ 1. FQC Reader    │
│ - Read header    │
│ - Load index     │
└──────┬───────────┘
       │ Block headers
       ▼
┌──────────────────┐
│ 2. Block Reader  │
│ - Read blocks    │
│ - Verify checksum│
└──────┬───────────┘
       │ Compressed blocks
       ▼
┌──────────────────┐
│ 3. Decompressor  │
│ - ABC inverse    │
│ - Zstd decode    │
│ - SCM inverse    │
└──────┬───────────┘
       │ Decompressed blocks
       ▼
┌──────────────────┐
│ 4. Reorder Map   │
│ - Restore order  │
│ - Apply inverse  │
└──────┬───────────┘
       │ Reordered ReadRecord[]
       ▼
┌──────────────────┐
│ 5. FASTQ Writer  │
│ - Format output  │
│ - Write file     │
└──────┬───────────┘
       │
       ▼
┌──────────────┐
│ FASTQ Output │
└──────────────┘
```

## Parallel Pipeline (Compression)

```
Thread 1          Thread 2           Thread 3
┌────────┐       ┌────────┐        ┌────────┐
│ Read   │  ch1  │Compress│  ch2   │ Write  │
│ Stage  │──────▶│ Stage  │───────▶│ Stage  │
└────────┘       └────────┘        └────────┘
   │                 │                  │
   ▼                 ▼                  ▼
I/O               CPU                I/O
```

**Channel Types:**
- `ch1`: Bounded crossbeam channel (buffer_size)
- `ch2`: Bounded crossbeam channel (buffer_size)

## Memory Flow

### During Compression

1. **Parse Phase**: Load chunk into memory
2. **Compress Phase**: Allocate compression buffers
3. **Write Phase**: Flush to disk, free buffers

### Peak Memory Usage

```
peak_memory = chunk_size + compression_buffers + output_buffer
```

- **Default mode**: `chunk_size = entire_file`
- **Streaming mode**: `chunk_size = buffer_size * record_size`

## Error Flow

Errors are propagated using `Result<T, FqcError>`:

```rust
pub fn compress() -> Result<()> {
    let records = parse_fastq(input)?;    // May return ParseError
    let blocks = build_blocks(records)?;  // May return BlockError
    let archive = compress(blocks)?;      // May return CompressError
    write_archive(archive)?;              // May return IoError
    Ok(())
}
```

## Related

- [Architecture Overview](./index.md)
- [Pipeline Components](./components/pipeline.md)
- [Parser Component](./components/parser.md)
