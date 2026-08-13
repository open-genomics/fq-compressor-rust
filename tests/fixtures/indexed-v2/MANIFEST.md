# Frozen Indexed v2 Archive Fixture

## Purpose

This fixture freezes a minimal `fqc-indexed/v2` archive so that future decoder
changes can be caught by a committed, verifiable byte stream. It is a decoder
compatibility contract, not a canonical-writer assertion: zstd and other
dependencies may change exact compressed bytes between versions.

## Generator

- **Source commit**: `1a2a2161bed88df5e21ce6e83ac3b668a93a44b0`
- **Binary**: `fqc compress --input input.fastq --output frozen.fqc`
- **Profile**: `--release` (default features: gz, bz2, xz)

## Files

| File | SHA-256 |
|------|---------|
| `input.fastq` | `cef5299ac4ef2d5130d8993f787e133deca568e1eaf565ddfa6a048749166ce1` |
| `frozen.fqc` | `32081764a533a20704bb614624eb4d0a4651bcac8ecc4e4e7cc0e54ac37e7a03` |

## Archive structure (from `fqc info`)

- File size: 448 bytes
- Total reads: 3
- Blocks: 1
- Reorder map: present
- Quality mode: lossless
- ID mode: exact
- Read length class: short

## Magic bytes (first 9 bytes)

```
89 46 51 43 0D 0A 1A 0A  20
```

The first 8 bytes are the indexed family magic; byte 9 is the version byte
`(2 << 4) | 0 = 0x20`.
