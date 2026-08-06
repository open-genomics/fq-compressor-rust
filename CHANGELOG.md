# Changelog

## [Unreleased]

### Added

- Benchmark foundation for parser and archive workflows (criterion-based)
- Performance foundation architecture documentation

### Changed

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
