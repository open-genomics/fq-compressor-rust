# Changelog

## [Unreleased]

### Fixed

- e2e 测试在并行全量运行时偶发失败（每个 `TempFile` 使用唯一临时目录，消除跨测试共享
  `/tmp/fqc_e2e_tests/` 的并发竞态）。
- `--pipeline` 压缩长读时产出空归档（0 blocks，exit 0）：`GlobalAnalyzer` 对非 Short
  类跳过重排后 `reverse_map` 为空，pipeline 只凭空 map 建块把全部读丢弃。三处
  （run / run_paired / run_interleaved）现在回退到原序直通并正确设置 original-order flag，
  与 engine 的 `run_archive` 一致；新增回归测试
  `test_e2e_pipeline_long_reads_no_reorder_nonempty_archive`。

### Changed

- README 首屏格式族说明重写为两实现对照表（仓库、实现语言、格式族 ID、完整 magic、
  访问模型、对方链接），并新增同名二进制 `fqc` 的 `PATH` 覆盖风险提醒。对应 openspec
  变更 `document-fqc-format-family`（verification 标注 ready-to-archive=no，待独立
  审查后再归档）。

### Added

- 阶段计时：compress / decompress 的 summary 打印 `Stage timings`（parse / reorder /
  process / write，单位 ms；process 为并行 worker 聚合 CPU 时间）。覆盖 archive、pipeline、
  streaming 三种模式，单 block 与 `--original-order` 路径同样计时。
- 真实语料压缩/吞吐证据：`scripts/fetch_real_corpus.sh` + `docs/real-corpus.md`（ENA 两份公开切片，
  与 C++ 侧同语料同 sha256，round-trip 逐字节一致）；`docs/hotspot-report.md` 记录阶段热点与
  优化前后对比。
- `--id-mode exact|tokenize|discard` on `compress` (default `tokenize`);
  Exact never tokenizes, Tokenize may fall back per block, Discard writes
  placeholder IDs. The chosen mode is stored in archive flags.
- `--lossy-quality qvz` is a distinct 8-level nearest-neighbor quality
  quantizer (`[7, 15, 20, 25, 30, 35, 40, 41]`) encoded with existing SCM,
  not a lossless alias and not trained rate-distortion QVZ.
- Archive compress ingest budget: `--memory-limit 0` resolves to a finite
  automatic cap (same 75% / hard-ceiling policy as decode); running peak
  estimate fails with `ResourceLimit` before the `.fqc` is created, and
  `--streaming` skips this full-ingest check. `--pipeline` uses the same
  ingest check (it is still not a bounded-memory streaming path).
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

### Performance

- 重排/建块从逐条克隆改为移动（`mem::take` / `split_off`）：块内容与顺序不变、压缩输出
  逐字节不变（reorder on 14,036,446 B / off 12,508,029 B），减少建块期整份读集克隆的分配。
- 热点测量（`docs/hotspot-report.md`）：真实 Illumina WXS 上全局重排为纯负收益（慢 5.7× 且
  输出大 12.2%，输入原序已空间有序）；长读压缩为单 block 串行（`--max-block-bases` 可切块并行）。

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

- Default `compress` ID mode is `tokenize` and is recorded as such in archive
  flags (no longer stored as Exact while the encoder auto-tokenizes).
- `--lossy-quality qvz` is no longer a lossless SCM alias.
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
