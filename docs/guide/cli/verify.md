# Verify Command

The `verify` command checks the integrity and validity of FQC archives.

## Basic Usage

```bash
fqc verify archive.fqc
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --input <FILE>` | Input FQC file | **Required** |
| `--verbose` | Show detailed verification progress | false |
| `--quick` | Skip checksum validation, check structure only | false |
| `--json` | Output results as JSON | false |

## Examples

### Basic Verification

```bash
fqc verify sample.fqc
```

Output:
```
Verifying archive integrity...
  ✓ Magic bytes valid
  ✓ Header checksum valid
  ✓ 50 blocks verified
  ✓ Footer checksum valid
  ✓ Global checksum valid
Archive is VALID
```

### Verbose Mode

Show detailed progress:

```bash
fqc verify sample.fqc --verbose
```

Output:
```
Verifying archive integrity...
  Block #0/50: ✓ Checksum valid (ABC codec)
  Block #1/50: ✓ Checksum valid (Zstd codec)
  Block #2/50: ✓ Checksum valid (ABC codec)
  ...
  Block #50/50: ✓ Checksum valid (Zstd codec)
  
  Footer: ✓ Valid
  Global checksum: ✓ Valid
  
Archive is VALID
Total time: 1.2s
```

### Quick Check

Skip checksums, validate structure only:

```bash
fqc verify sample.fqc --quick
```

This is faster but only validates the structure, not data integrity.

### JSON Output

For programmatic validation:

```bash
fqc verify sample.fqc --json
```

Output:
```json
{
  "valid": true,
  "checks": {
    "magic": true,
    "header_checksum": true,
    "blocks_valid": 50,
    "blocks_total": 50,
    "footer_valid": true,
    "global_checksum": true
  },
  "details": {
    "version": "1.0",
    "read_count": 1000000,
    "block_count": 50
  }
}
```

## Validation Checks

The verify command performs the following checks:

### 1. Magic Bytes

Validates the FQC magic number (`FQC\x01`) at the start of the file.

### 2. Header Checksum

Validates the CRC64 checksum of the global header.

### 3. Block Checksums

For each block:
- Validates block header checksum
- Validates data integrity using per-block checksums
- Verifies codec is valid

### 4. Footer Checksum

Validates the archive footer structure and checksums.

### 5. Global Checksum

Validates the overall archive integrity using a global checksum.

## Exit Codes

| Code | Meaning |
|------|----------|
| 0 | Archive is valid |
| 1 | File not found |
| 2 | Invalid FQC format |
| 3 | Checksum mismatch detected |
| 4 | Corrupted block found |
| 5 | Read error |

## Use Cases

1. **Post-transfer validation** - Verify archives after copying or downloading
2. **Pipeline quality gate** - Ensure input archives are valid before processing
3. **Storage integrity** - Periodically verify stored archives haven't degraded
4. **Debug assistance** - Identify specific corrupted blocks

## Related

- [Compress Command](./compress.md)
- [Info Command](./info.md)
- [Decompress Command](./decompress.md)
