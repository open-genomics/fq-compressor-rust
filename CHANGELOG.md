# Changelog

## [Unreleased]

### Added

- 阶段计时：compress / decompress 的 summary 打印 `Stage timings`（parse / reorder /
  process / write，单位 ms；process 为并行 worker 聚合 CPU 时间）。覆盖 archive、pipeline、
  streaming 三种模式，单 block 与 `--original-order` 路径同样计时。
- 真实语料压缩/吞吐证据：`scripts/fetch_real_corpus.sh` + `docs/reference/real-corpus.md`
  （ENA 两份公开切片，与 C++ 侧同语料同 sha256，round-trip 逐字节一致）；
  `docs/reference/hotspot-report.md` 记录阶段热点与优化前后对比。
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
- Frozen indexed v2 decoder fixture (`tests/fixtures/indexed-v2/`) with
  MANIFEST.md documenting generator commit, command, and SHA-256 hashes.
- Format contract tests (`tests/test_format_contract.rs`) covering magic,
  version, codec/checksum identifier encoding, frozen fixture round-trip,
  and unknown-identifier rejection.
- Benchmark foundation for parser and archive workflows (criterion-based),
  plus performance hotspot documentation.
- 轻量 CI（`.github/workflows/ci.yml`，GitHub Actions）：fmt / clippy / test /
  doc 四门禁（`--locked`），push 与 PR 触发。
- 治理文档：`SECURITY.md`（私有漏洞上报）、`CONTRIBUTING.md`、`VERSIONING.md`、
  `.github/ISSUE_TEMPLATE/`（bug / feature 表单）。
- `Cargo.toml` 补 `homepage` 字段；`openspec/project.md` 记录 `fqc-indexed/v2`
  身份、决策 `FQC-DEC-001` 与外部边界。
- 面防御测试：归档解码器对 frozen fixture 的全部截断点、逐字节单比特翻转与
  确定性伪随机垃圾输入均不 panic（fail closed 的手工用例原由
  `test_decode_budget` 覆盖，此处补穷举广度）；`FastqParser` 对随机与变异
  FASTQ 输入不 panic。
- `ZstdSequenceCompressor` 直接测试（roundtrip、空流、计数/长度不一致、长度
  分歧、截断、尾随字节）；`AsyncWriter` 直接测试（字节顺序、flush 应答、
  后台写失败传播）；`FqcReader::info()` 字段断言钉住 frozen fixture 的
  MANIFEST 数字。

### Changed

- docs 按 Diátaxis 重组为 `tutorials/`、`how-to/`、`reference/`、`explanation/` 四象限，
  `docs/README.md` 改为按用户目标索引；`cli.md` 归入 `reference/`、`quick-start.md` 归入
  `tutorials/`、基准与测量报告归入 `reference/`；README 与 guide 重复的安装/常用命令/
  `--memory-limit` 段落收敛为指向 canonical 文档的薄链接。
- 决策记录收敛到 `.agents/notes/`（Agent Note，npm scripts `verify-notes` 校验，
  不进 CI）：`openspec/` 变更流程停用（`openspec/specs/` 保留为能力规范参照），
  `docs/architecture/decisions/` 的三篇 ADR 折叠为 `implemented/architecture/` 下的
  Note 后删除。
- README 首屏格式族说明重写为两实现对照表（仓库、实现语言、格式族 ID、完整 magic、
  访问模型、对方链接），并新增同名二进制 `fqc` 的 `PATH` 覆盖风险提醒。对应 openspec
  变更 `document-fqc-format-family`（2026-10-01 归档，规格落为
  `openspec/specs/format-governance/`）。
- README：新增 CI 徽章；"零 unsafe"表述修正为"除 Windows 内存探测外零 unsafe"
  （`GlobalMemoryStatusEx` FFI 见 `src/memory_budget.rs`）。
- docs 卫生：功能对比矩阵 / 学术文献 / 执行模式说明单源化（分别收敛到
  `docs/explanation/comparison.md`、`docs/explanation/whitepaper.md`、
  `docs/how-to/modes.md`）；白皮书修正 src 树、测试数（215，2026-08 实测）与重排说明；
  微型与真实语料压缩比互引；内部收尾计划移出 docs/ 文档树（2026-10-01 随收尾完成删除）。
- 重排/建块从逐条克隆改为移动（`mem::take` / `split_off`）：块内容与顺序不变、压缩输出
  逐字节不变（reorder on 14,036,446 B / off 12,508,029 B），减少建块期整份读集克隆的分配。
- 热点测量（`docs/reference/hotspot-report.md`）：真实 Illumina WXS 上全局重排为纯负收益
  （慢 5.7× 且输出大 12.2%，输入原序已空间有序）；长读压缩为单 block 串行
  （`--max-block-bases` 可切块并行）。
- Default `compress` ID mode is `tokenize` and is recorded as such in archive
  flags (no longer stored as Exact while the encoder auto-tokenizes).
- `--lossy-quality qvz` is no longer a lossless SCM alias.
- Archived five completed OpenSpec changes under
  `openspec/changes/archive/2026-08-18-*` and merged their requirements into
  `openspec/specs/` (`archive-format`, `file-output`, `decode-budget`).
- Corrected stale module paths in the whitepaper and architecture diagram;
  relabeled unverified ratio/throughput claims in `docs/explanation/comparison.md`;
  documented that compress archive `--memory-limit 0` still allows full ingest.
- Regrouped `src/`: archive format files under `src/archive/`, core engine under
  `src/engine/`, `common/memory_budget` flattened to the top level.
- Docs converted to plain Markdown with a `docs/README.md` index.
- Consolidated all development branches into master; repository moved to the
  open-genomics organization.
- 内部重构（行为不变，T4 架构收敛）：解压编排下沉至
  `pipeline/decompression_classic.rs`（`commands/decompress.rs` 退化为薄 CLI
  层）；压缩 7 条执行路径收敛为"骨架 + 拓扑参数"（`pipeline/compression.rs`
  共享重排与三阶段写核心，engine streaming 三兄弟收敛为一个核心 + 薄适配
  器）；`run_paired` 由串行改为并行；清零全部 `too_many_lines` /
  `needless_pass_by_value` / `dead_code` 豁免（`main.rs` 改为复用 lib
  target）。决策见 `.agents/notes/implemented/architecture/2026-10-01-converge-execution-paths.md`。
- CI 门禁扩展：在 fmt / clippy / test / doc 四门禁基础上恢复 MSRV（1.75.0 编译
  检查，已本地实测通过）与 cargo-deny job，全部 job 接入 `Swatinem/rust-cache`
  缓存与 `concurrency` 并发取消。

### Fixed

- 加固 indexed v2 解码边界：恢复历史无损块校验和语义，有损/丢弃块不再携带
  不可能匹配的逻辑校验和；ABC V3 使用 codec `0x11`，并严格校验载荷版本、计数、
  映射、偏移、UTF-8 与尾随字节；Zstd、辅助流、ID 流和算术质量流拒绝截断或不一致数据。
- 归档读取器现在拒绝未知全局标志、错误 checksum/保留字段、非连续块/流布局及无效
  重排映射；`--skip-corrupted` 仍可在解码阶段把损坏块替换为占位读段。
- FASTQ 文件/标准输入路径启用 Phred+33 质量校验；`+` 行同时接受完整 header 或
  identifier-only 的规范写法；倒置解压范围在串行与 pipeline 路径统一返回 usage error。
- GlobalAnalyzer 对 `N`/歧义碱基和零窗口参数不再发生下溢或伪造 minimizer。
- e2e 测试在并行全量运行时偶发失败（每个 `TempFile` 使用唯一临时目录，消除跨测试共享
  `/tmp/fqc_e2e_tests/` 的并发竞态）。
- `--pipeline` 压缩长读时产出空归档（0 blocks，exit 0）：`GlobalAnalyzer` 对非 Short
  类跳过重排后 `reverse_map` 为空，pipeline 只凭空 map 建块把全部读丢弃。三处
  （run / run_paired / run_interleaved）现在回退到原序直通并正确设置 original-order flag，
  与 engine 的 `run_archive` 一致；新增回归测试
  `test_e2e_pipeline_long_reads_no_reorder_nonempty_archive`。
- Format spec (`docs/reference/format-spec.md`) corrections: codec table
  switched from flat enumeration to `(family << 4) | version` encoding with the
  full family table; checksum ID 0 means XxHash64 (not "none"); removed
  fictional v1 fallback (reader accepts only major == 2); removed nonexistent
  `LEGACY_LONG_READ_MODE` at bit 2; replaced nonexistent QVZ stream type with
  actual codec families.
- All stale org links (LessUp -> open-genomics).
- README CLI example: `--memory-limit` is a global flag and must precede the subcommand.
- `test_pipeline_concurrency` 移除"秒级时间戳必须变化"的偶发假红断言（三次快速
  压缩同秒完成属正常行为；测试本意是内容稳定性，予以保留）。
- `AsyncWriter` 死锁：后台线程因写错误提前退出时，在途的 `flush()` 会永久阻塞
  （Flush 应答滞留队列，而主线程持有的 Sender 使队列永不销毁）。后台线程改为
  锁存首个错误并排空队列，保证每个 Flush 都有应答；错误经 flush 返回值或
  Drop 日志浮出。由新增的直接测试发现。

### Security

- `crossbeam-epoch` 0.9.18 -> 0.9.21（RUSTSEC 公告：受影响版本的 `fmt::Display`
  实现解引用底层指针；由恢复的 cargo-deny 门禁首次运行即捕获）。

### Removed

- Windows 平台支持：移除 `GlobalMemoryStatusEx` FFI 内存探测（全仓库唯一
  `unsafe`，从未被 Linux-only CI 编译过），Windows 及其他未适配平台落入
  8192 MB 默认内存预算（可由 `--memory-limit` 覆盖）；README / SECURITY 的
  unsafe 声明更新为字面零 unsafe。CI 不含 Windows job。决策见
  `.agents/notes/implemented/simplification/2026-10-01-drop-windows-support.md`。

- Node.js-based OpenSpec spec tooling（现由 `.agents/notes/` 决策笔记体系取代）.
- VitePress docs site, GitHub Pages deployment, and all Node.js dependencies.
- Release automation（CI 四门禁与 cargo-deny 已恢复；自动发布保持移除）.
- Dockerfile, devcontainer, git hooks, helper scripts, and peripheral tooling configs.

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
