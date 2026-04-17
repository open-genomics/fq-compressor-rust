# Test Data

This directory contains test FASTQ files for the test suite.

## Files

| File | Description | Reads | Length |
|------|-------------|-------|--------|
| `test_se.fastq` | Single-end test data | 10 | 150bp |
| `test_R1.fastq` | Paired-end read 1 | 10 | 150bp |
| `test_R2.fastq` | Paired-end read 2 | 10 | 150bp |
| `test_interleaved.fastq` | Interleaved paired-end | 10 | 150bp |

## Format

All files use standard FASTQ format:
```
@SEQ_ID
GATTTGGGGTTCAAAGCAGTATCGATCAAATAGTACATCCCTTTAG
+
!''*((((***+))%%%++)(%%%%).1***-+*''))**55CCF>>>>
```

## Usage

Tests reference these files using relative paths:
```rust
let input = "tests/data/test_se.fastq";
```

## Generating New Test Data

To generate additional test data:
```bash
# Use wgsim or similar tools
wgsim -N 10 -1 150 -2 150 reference.fa test_R1.fastq test_R2.fastq
```

## Notes

- Files are small for fast test execution
- Reads are synthetic (not real genomic data)
- Quality scores are realistic but simulated
