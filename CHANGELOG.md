# Changelog

## [Unreleased]

### Added

- Archive compress ingest budget: `--memory-limit 0` resolves to a finite
  automatic cap (same 75% / hard-ceiling policy as decode); running peak
  estimate fails with `ResourceLimit` before the `.fqc` is created, and
  `--streaming` skips this full-ingest check.
- Cross-family magic dispatch: reject C++ `fqc-sequential/v2` archives with an
  explicit unsupported-format-family error pointing at
  `open-genomics/fq-compressor` (unknown/truncated magics stay distinct).
- Operation-scoped decode/verify memory budget (`DecodeBudget`): `--memory-limit`
  applies to decompress and full verify; archive-declared sizes and zstd output
  are capped before allocation; automatic (`0`) remains finite with hard
  structural ceilings (not unlimited).
- Header-driven per-stream codec dispatch on decompress: IDs/seq/qual/aux each
  validate their block-header codec byte against an allow-list (unknown family,
  wrong stream, and non-v0 versions fail closed); global quality/id mode must
  not contradict the declared families.
- Transactional ordinary-file output (`OutputTransaction`): compress and
  decompress write same-directory temps and rename only after a successful
  flush/close. Mid-run failure leaves a missing target absent, or keeps the
  previous file when `-f/--force` was used. Stdout remains non-transactional;
  `--split-pe` commits R1 then R2 (POSIX cannot atomically rename two paths).
- Lightweight OpenSpec change workflow for high-risk changes (pure Markdown
  `openspec/` artifacts; no Node.js, CLI, or tool-specific configs)
- Frozen indexed v2 decoder fixture (`tests/fixtures/indexed-v2/`) with
  MANIFEST.md documenting generator commit, command, and SHA-256 hashes
- Format contract tests (`tests/test_format_contract.rs`) covering magic,
  version, codec/checksum identifier encoding, frozen fixture round-trip,
  and unknown-identifier rejection
- `openspec/project.md` recording `fqc-indexed/v2` identity, decision
  `FQC-DEC-001`, and external boundaries

### Fixed

- Format spec (`docs/reference/format-spec.md`) codec table: corrected from
  flat enumeration to `(family << 4) | version` encoding with full family table
- Format spec checksum table: ID 0 means XxHash64, not "none"
- Format spec version compatibility: removed fictional v1 fallback; reader
  accepts only major == 2
- Format spec flags table: removed nonexistent `LEGACY_LONG_READ_MODE` at bit 2
- Format spec stream types: replaced nonexistent QVZ with actual codec families

- Benchmark foundation for parser and archive workflows (criterion-based)
- Performance foundation architecture documentation

### Changed

- Archived five completed OpenSpec changes under
  `openspec/changes/archive/2026-08-18-*` and merged their requirements into
  `openspec/specs/` (`archive-format`, `file-output`, `decode-budget`)
- Corrected stale module paths in the whitepaper and architecture diagram
- Relabeled unverified ratio/throughput claims in `docs/comparison.md`;
  documented that compress archive `--memory-limit 0` still allows full ingest
- Regrouped `src/`: archive format files under `src/archive/`, core engine under
  `src/engine/`, `common/memory_budget` flattened to the top level
- Docs converted to plain Markdown with a `docs/README.md` index
- Consolidated all development branches into master

### Removed

- OpenSpec spec system (`openspec/`)
- VitePress docs site, GitHub Pages deployment, and all Node.js dependencies
- CI workflows, release automation, and cargo-deny
- Governance boilerplate: CODE_OF_CONDUCT, CONTRIBUTING, SECURITY
- Dockerfile, devcontainer, git hooks, helper scripts, and peripheral tooling configs

### Fixed

- All stale org links (LessUp -> open-genomics)
- README CLI example: `--memory-limit` is a global flag and must precede the subcommand

### Repository

- Repository moved to the open-genomics organization
- Single-branch architecture (master only)

## [0.1.1] - 2026-04-16

- Documentation and CI refresh for the 0.1.x line.
- Security policy added.
- Release automation and Pages deployment tightened.

## [0.1.0] - 2026-03-07

- Initial stable release of `fqc`.
- Core `compress`, `decompress`, `info`, and `verify` commands shipped.
- `.fqc` block-indexed archive format released with paired-end support.

[Unreleased]: https://github.com/open-genomics/fq-compressor-rust/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/open-genomics/fq-compressor-rust/releases/tag/v0.1.1
[0.1.0]: https://github.com/open-genomics/fq-compressor-rust/releases/tag/v0.1.0
