# Tasks: correct-indexed-v2-spec

## 1. Baseline

- [x] 1.1 阅读根 `AGENTS.md`，记录 HEAD、`git status --short` 和审计 base 差异
  - HEAD: `1a2a2161bed88df5e21ce6e83ac3b668a93a44b0` (matches audit base)
  - Working tree: clean
- [x] 1.2 运行 `cargo fmt --all -- --check` 与最小 archive tests，记录现有结果
  - fmt: passed, all existing tests: passed
- [x] 1.3 从 `src/archive/format.rs`、writer、reader 建立实际字段/identifier 对照表
  - Codec: `(family << 4) | version`, families: Raw=0x0, AbcV1=0x1, ScmV1=0x2, DeltaLzma=0x3, DeltaZstd=0x4, DeltaVarint=0x5, OverlapV1=0x6, ZstdPlain=0x7, ScmOrder1=0x8, External=0xE, Reserved=0xF
  - Checksum: XxHash64 = 0 (not "none")
  - Version: only major==2 accepted, no v1 fallback

## 2. Characterization and fixture

- [x] 2.1 添加 codec/checksum/version 精确 bytes 的结构测试
  - `tests/test_format_contract.rs`: 16 tests covering magic, version, codec encoding, checksum ID, block header codec bytes
- [x] 2.2 用固定最小 FASTQ 生成 indexed v2 archive，提交 input/archive/manifest
  - `tests/fixtures/indexed-v2/input.fastq` (3 reads, 50bp each)
  - `tests/fixtures/indexed-v2/frozen.fqc` (448 bytes)
  - `tests/fixtures/indexed-v2/MANIFEST.md` (SHA-256 hashes, generator commit, command)
- [x] 2.3 添加 frozen decoder round-trip、hash、未知 identifier 和 unsupported major 测试
  - frozen archive magic/version/size checks, reader open + info match, block codec check
  - unsupported major version rejection, bad magic rejection, nonzero reserved rejection

## 3. Documentation

- [x] 3.1 修正 format spec 的 codec/checksum 表、版本兼容和 v1 声明
  - `docs/reference/format-spec.md`: codec table now shows (family << 4) | version with full family table
  - checksum table: ID 0 = XxHash64 (not "none")
  - version compatibility: no v1 fallback, only major==2
  - flags table: removed nonexistent LEGACY_LONG_READ_MODE at bit 2
  - stream types: replaced nonexistent QVZ with actual codec families
  - added format family identification section
  - added frozen fixture section
- [x] 3.2 建立/更新 `openspec/project.md`，登记 `fqc-indexed/v2` 与 `FQC-DEC-001`
  - `openspec/project.md` created with identity, contracts, boundaries, decision index
- [x] 3.3 将根 `AGENTS.md` 的 blanket prohibition 收窄到 lightweight high-risk change policy
  - "no spec-driven change management" replaced with lightweight OpenSpec policy for high-risk changes only
  - no Node.js, CLI, dashboard, or tool-specific configs
- [x] 3.4 更新直接引用错误编号的文档和 CHANGELOG；保留旧系统曾被移除的历史
  - `CHANGELOG.md`: Added entries for lightweight OpenSpec, frozen fixture, format contract tests, project.md
  - Fixed entries for codec table, checksum table, version compatibility, flags table, stream types
  - Existing "Removed: OpenSpec spec system" entry preserved as historical fact

## 4. Verification

- [x] 4.1 `cargo fmt --all -- --check` — exit 0
- [x] 4.2 `cargo clippy --all-targets -- -D warnings` — exit 0
- [x] 4.3 `cargo test --lib --tests` — exit 0, 192 tests passed
- [x] 4.4 `cargo doc --no-deps` — exit 0
- [x] 4.5 `git diff --check`、scope 审计并填写 `verification.md` — exit 0, scope verified
