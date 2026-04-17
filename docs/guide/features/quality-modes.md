# Quality Score Modes

fqc supports multiple quality score formats and compression modes.

## Quality Score Formats

### Illumina 1.8+ (Default)

- **Encoding**: Sanger / Illumina 1.8+
- **Range**: ASCII 33-73 (Phred+33)
- **Score range**: 0-40
- **Most common**: Modern Illumina data

```
@read1
ATCG...
+
IIII...  (I = Phred 40)
```

### Illumina 1.3+

- **Encoding**: Illumina 1.3+
- **Range**: ASCII 64-104 (Phred+64)
- **Score range**: 0-40
- **Legacy**: Older Illumina data

```
@read1
ATCG...
+
hhhh...  (h = Phred 40)
```

### Solexa

- **Encoding**: Solexa
- **Range**: ASCII 59-104
- **Score range**: -5 to 40
- **Very old**: Early Solexia instruments

## Quality Compression Modes

fqc offers multiple quality compression strategies:

### Lossless (Default)

Preserves all quality scores exactly:

```bash
fqc compress input.fastq --quality-mode lossless -o output.fqc
```

- ✅ 100% fidelity
- ⚠️ Largest file size
- Best for: Archival, re-analysis

### Lossy

Applies quality score binning:

```bash
fqc compress input.fastq --quality-mode lossy -o output.fqc
```

Quality scores are grouped into bins:
- Scores 0-10 → bin 0
- Scores 11-20 → bin 1
- Scores 21-30 → bin 2
- Scores 31-40 → bin 3

- ✅ Smaller file size (10-20% reduction)
- ⚠️ Some precision loss
- Best for: Primary analysis, variant calling

### Discard

Removes quality scores entirely:

```bash
fqc compress input.fastq --quality-mode discard -o output.fqc
```

- ✅ Smallest file size (30-40% reduction)
- ⚠️ No quality information
- Best for: Sequence-only analysis

## Mode Comparison

| Mode | Size | Fidelity | Use Case |
|------|------|----------|----------|
| Lossless | 100% | 100% | Archival |
| Lossy | 80-90% | ~95% | Analysis |
| Discard | 60-70% | 0% | Sequence only |

## Auto-Detection

fqc can auto-detect quality format:

```bash
fqc compress input.fastq --quality-format auto -o output.fqc
```

Supported formats:
- `auto` (default)
- `illumina_1.8`
- `illumina_1.3`
- `solexa`

## Examples

### Lossless Compression

```bash
fqc compress data.fastq --quality-mode lossless -o lossless.fqc
```

### Lossy for Analysis

```bash
fqc compress data.fastq --quality-mode lossy -o analysis.fqc
```

### Discard Quality

```bash
fqc compress data.fastq --quality-mode discard -o sequence_only.fqc
```

## Impact on Downstream Tools

| Tool | Lossless | Lossy | Discard |
|------|----------|-------|---------|
| Variant calling | ✅ | ⚠️ | ❌ |
| Assembly | ✅ | ✅ | ✅ |
| QC metrics | ✅ | ⚠️ | ❌ |
| Mapping | ✅ | ✅ | ✅ |

## Related

- [Compress Command](../cli/compress.md)
- [ABC Algorithm](../../algorithms/abc.md)
- [SCM Compression](../../algorithms/scm.md)
