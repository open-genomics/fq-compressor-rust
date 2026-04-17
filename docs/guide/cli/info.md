# Info Command

The `info` command displays metadata about FQC archives without decompressing.

## Basic Usage

```bash
fqc info archive.fqc
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --input <FILE>` | Input FQC file | **Required** |
| `--detailed` | Show detailed block-level information | false |
| `--show-codecs` | Display compression algorithms used | false |
| `--json` | Output as JSON | false |

## Examples

### Basic Archive Info

```bash
fqc info sample.fqc
```

Output:
```
FQC Archive Information
========================
Version: 1.0
Total reads: 1,000,000
Total blocks: 50
Original size: 500 MB
Compressed size: 125 MB
Compression ratio: 4.0x
Read length mode: 150bp
Quality mode: Illumina 1.8+
```

### Detailed View

Show information for each block:

```bash
fqc info sample.fqc --detailed
```

Output:
```
FQC Archive Information (Detailed)
====================================
...

Block #0:
  Read count: 20,000
  Codec: ABC
  Original size: 10 MB
  Compressed size: 2.5 MB
  Ratio: 4.0x

Block #1:
  Read count: 20,000
  Codec: Zstd
  Original size: 10 MB
  Compressed size: 3.0 MB
  Ratio: 3.3x
...
```

### Show Codecs Used

List all compression algorithms in the archive:

```bash
fqc info sample.fqc --show-codecs
```

Output:
```
Codecs Used:
  ABC (Consensus + Delta): 40 blocks
  Zstd (Level 6): 8 blocks
  SCM (Quality): 40 blocks
```

### JSON Output

For programmatic access:

```bash
fqc info sample.fqc --json
```

Output:
```json
{
  "version": "1.0",
  "read_count": 1000000,
  "block_count": 50,
  "original_size": 524288000,
  "compressed_size": 131072000,
  "compression_ratio": 4.0,
  "read_length_mode": 150,
  "quality_mode": "illumina_1.8"
}
```

## Information Displayed

### Basic Mode

- Archive version
- Total read count
- Total block count
- Original and compressed sizes
- Overall compression ratio
- Read length mode
- Quality score mode

### Detailed Mode

All basic info plus:
- Per-block read counts
- Per-block codecs
- Per-block sizes and ratios
- Block boundaries and offsets
- Checksums

### Show Codecs

- List of unique codecs used
- Frequency of each codec
- Compression levels/settings

## Exit Codes

| Code | Meaning |
|------|----------|
| 0 | Success |
| 1 | File not found |
| 2 | Invalid FQC format |
| 3 | Read error |

## Use Cases

1. **Quick verification** - Check archive validity before processing
2. **Pipeline integration** - Get archive metadata for workflow decisions
3. **Quality control** - Verify compression ratios meet expectations
4. **Debugging** - Identify problematic blocks with `--detailed`

## Related

- [Compress Command](./compress.md)
- [Decompress Command](./decompress.md)
- [Verify Command](./verify.md)
