# Testing Specifications

This directory contains BDD (Behavior-Driven Development) test specifications for fqc.

## Testing Strategy

fqc uses a multi-layer testing approach to ensure correctness:

1. **Unit Tests** — Individual function testing
2. **Integration Tests** — Component interaction testing
3. **End-to-End Tests** — Full compress/decompress round-trip
4. **Property Tests** — Invariant checking (future)

## Test Organization

### Source Tests (`src/`)

Located alongside implementation code using `#[cfg(test)]` modules.

| Module | Test Coverage |
|--------|---------------|
| `types.rs` | Type constants, validation |
| `format.rs` | Header serialization/deserialization |
| `error.rs` | Error conversion, exit codes |
| `reorder_map.rs` | ZigZag delta encoding, varint |

### Integration Tests (`tests/`)

External test files in `tests/` directory:

| File | Tests | Coverage |
|------|-------|----------|
| `test_algo.rs` | 19 | ID/quality compressor, PE optimizer |
| `test_dna.rs` | 15 | DNA encoding tables, reverse complement |
| `test_e2e.rs` | 15 | End-to-end compression workflows |
| `test_format.rs` | 15 | Binary format validation |
| `test_parser.rs` | 19 | FASTQ parser edge cases |
| `test_reorder_map.rs` | 23 | Reorder map operations |
| `test_roundtrip.rs` | 14 | Compress→decompress round-trip |
| `test_types.rs` | 11 | Type/constant validation |

**Total: 131 tests, 0 failures expected**

## Test Data

Test data files are located in `tests/data/`:

| File | Purpose |
|------|---------|
| `test_se.fastq` | Single-end test data (short reads) |
| `test_pe.fastq` | Paired-end test data |
| `test_long.fastq` | Long reads (>300bp) |
| `test_mixed.fastq` | Mixed read lengths |

### Generating Test Data

```bash
# Generate test FASTQ files
cargo run --bin generate_test_data
```

## Test Execution

### Run All Tests

```bash
cargo test --lib --tests
```

### Run Specific Test File

```bash
cargo test --test test_roundtrip
```

### Run Single Test

```bash
cargo test test_my_feature
```

### Performance Testing

```bash
# Benchmark compression
cargo bench --bench compression
```

## Test Patterns

### Round-trip Test Pattern

```rust
#[test]
fn test_roundtrip() -> Result<()> {
    let input = "tests/data/test_se.fastq";
    let output = TempFile::new(".fqc")?;

    // Compress
    compress_file(input, output.path(), Default::default())?;

    // Decompress
    let records = decompress_file(output.path(), Default::default())?;
    let original = read_fastq_records(input)?;

    // Verify
    assert_roundtrip_match(&original, &records)?;
    Ok(())
}
```

### Helper Functions (from `test_e2e.rs`)

| Function | Purpose |
|----------|---------|
| `compress_file()` | Compress FASTQ to FQC |
| `decompress_file()` | Decompress FQC to FASTQ |
| `read_fastq_records()` | Parse FASTQ file |
| `assert_roundtrip_match()` | Verify round-trip integrity |
| `TempFile::new()` | Create temporary file with cleanup |

## Acceptance Criteria

### Compress Command

**Given** a valid FASTQ file
**When** running `fqc compress`
**Then**:
- Output FQC file is valid
- All records are compressed
- Exit code is 0

### Decompress Command

**Given** a valid FQC file
**When** running `fqc decompress`
**Then**:
- Output FASTQ matches original
- All records are preserved
- Exit code is 0

### Info Command

**Given** a valid FQC file
**When** running `fqc info`
**Then**:
- Archive metadata is displayed
- Read count, sizes, and modes are correct

### Verify Command

**Given** a valid FQC file
**When** running `fqc verify`
**Then**:
- Checksums match
- All blocks are valid
- Exit code is 0

## CI Integration

Tests run automatically on:
- Every push/PR to `main` branch
- Every release tag (`v*`)

**Workflow**: `.github/workflows/ci.yml`

## Related Documents

- [Core Compression Spec](../product/core-compression.md)
- [CLI Commands Spec](../product/cli-commands.md)
- [File Format Spec](../product/file-format.md)
