# Verification: correct-indexed-v2-spec

- Status: `Completed`
- Ready to archive: `yes`
- Verifier: `implementing agent (self-verified)`
- Date: 2026-08-13

## Environment

- HEAD: `1a2a2161bed88df5e21ce6e83ac3b668a93a44b0` (matches audit base)
- Working tree: clean before apply; changes only in scope after apply
- CARGO_HOME: `/tmp/fqc-cargo-home` (read-only default registry workaround)
- Toolchain: rustc 1.x (stable)

## Requirement -> Evidence Matrix

| Requirement | Scenario | Evidence | Result |
|---|---|---|---|
| Indexed archive identity | Reader accepts indexed family | `test_indexed_magic_exact_bytes`: asserts MAGIC_BYTES == [0x89, F, Q, C, 0D, 0A, 1A, 0A] | passed |
| Indexed archive identity | Reader accepts indexed family | `test_frozen_archive_magic_and_version`: reads frozen.fqc, checks first 8 bytes == MAGIC_BYTES, byte 9 == 0x20 | passed |
| Encoded identifiers match implementation | Codec identifier is documented and tested | `test_codec_encoding_is_high_nibble_family_low_nibble_version`: verifies encode_codec(family, 0) == family << 4 for all 11 families | passed |
| Encoded identifiers match implementation | Codec identifier is documented and tested | `test_specific_codec_bytes_match_implementation`: asserts exact bytes (0x00, 0x10, 0x20, 0x40, 0x50, 0x70, 0x80) | passed |
| Encoded identifiers match implementation | Checksum identifier zero | `test_checksum_id_zero_means_xxhash64`: asserts ChecksumType::XxHash64 as u8 == 0, GlobalHeader round-trip preserves checksum_type == 0 | passed |
| Encoded identifiers match implementation | Codec identifier is documented and tested | `test_block_header_stores_encoded_codec_bytes`: round-trips BlockHeader, asserts codec_ids == 0x40, codec_seq == 0x10, codec_qual == 0x20, codec_aux == 0x50 | passed |
| Version compatibility is explicit | Unsupported major | `test_version_compatible_only_major_2`: asserts is_version_compatible(0x20) == true, (0x2F) == true, (0x10) == false, (0x30) == false, (0x00) == false | passed |
| Version compatibility is explicit | Unsupported major | `test_reader_rejects_unsupported_major_version`: corrupts frozen.fqc version byte to 0x30, asserts FqcReader::open returns UnsupportedVersion { major: 3 } | passed |
| Frozen decoder fixture | Future reader decodes frozen archive | `test_frozen_archive_reader_opens_and_info_matches`: opens frozen.fqc, checks total_reads=3, num_blocks=1, has_reorder_map=true, quality_mode=Lossless, id_mode=Exact, read_length_class=Short | passed |
| Frozen decoder fixture | Future reader decodes frozen archive | `test_frozen_archive_block_uses_abc_codec`: reads block 0, asserts checksum_type==0, decode_codec_family(codec_seq)==AbcV1 | passed |
| Frozen decoder fixture | Future reader decodes frozen archive | `test_frozen_archive_size_matches_manifest`: asserts file size == 448 bytes | passed |
| Unknown identifiers fail closed | Unknown codec identifier | `test_reader_rejects_bad_magic`: corrupts magic byte, asserts FqcReader::open fails | passed |
| Unknown identifiers fail closed | Unknown codec identifier | `test_block_header_rejects_nonzero_reserved`: asserts nonzero reserved1 is rejected | passed |
| Unknown identifiers fail closed | Unknown codec identifier | `test_global_header_rejects_nonzero_reserved`: asserts nonzero reserved is rejected | passed |

## Command Results

| Command | Exit status | Summary |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | No formatting issues |
| `cargo clippy --all-targets -- -D warnings` | 0 | No warnings |
| `cargo test --lib --tests` | 0 | 192 tests passed across 13 test suites (lib 10, main 10, test_algo 10, test_benchmark_support 2, test_compression_engine 11, test_compression_orchestration 6, test_dna 19, test_e2e 29, test_format 22, test_format_contract 16, test_parser 21, test_roundtrip 15, test_types 11) |
| `cargo doc --no-deps` | 0 | Documentation generated successfully |
| `git diff --check` | 0 | No whitespace errors |
| `git status --short` | clean scope | Modified: AGENTS.md, CHANGELOG.md, docs/reference/format-spec.md. New: openspec/, tests/fixtures/, tests/test_format_contract.rs |

## Diff scope audit

All changes are within the allowed surface defined in proposal.md:

- `openspec/` - new directory with project.md, AGENTS.md, and change package
- `AGENTS.md` - only the spec-management prohibition line narrowed
- `docs/reference/format-spec.md` - codec table, checksum table, version compatibility, flags table corrected
- `tests/test_format_contract.rs` - new characterization tests
- `tests/fixtures/indexed-v2/` - frozen fixture (input.fastq, frozen.fqc, MANIFEST.md)
- `CHANGELOG.md` - entries added under [Unreleased]

No source code (`src/`), no Cargo.toml, no CI, no build configuration modified.
No product name, extension, magic, or algorithm changed.

## Remaining risks

1. **Frozen archive byte stability**: The frozen archive was generated with the
   current zstd version. If zstd changes, future writer output may differ, but
   the frozen archive remains valid for decoder compatibility testing.
2. **SHA-256 verification is manual**: The test suite verifies structural
   properties (magic, version, size). Full SHA-256 verification is documented
   in MANIFEST.md for human review. A future change could add a SHA-256
   dev-dependency for automated hash checking.
3. **No v1 reader exists**: This is by design (no v1 fallback was ever
   implemented). The documentation now correctly states this.
