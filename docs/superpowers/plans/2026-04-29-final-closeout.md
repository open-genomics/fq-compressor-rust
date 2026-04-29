# Final Closeout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring `fqc` to a clean stabilization closeout state without discarding existing worktree or working-tree changes.

**Architecture:** Treat OpenSpec as the source of truth, finish the already-started CLI/memory/blocking behavior fixes, then prune repository process surface area only where the current specs support it. Keep the Rust implementation changes small and covered by focused tests; keep docs and AI guidance project-specific instead of generic templates.

**Tech Stack:** Rust 1.75, Cargo, Clap, Rayon, VitePress, GitHub Actions, GitHub CLI, OpenSpec.

---

## File Structure

- `src/commands/compress.rs`: compression mode orchestration, read-length sampling, effective block sizing, and memory-limit enforcement.
- `src/algo/block_compressor.rs`: sequence codec selection for short-read ABC versus Zstd fallback.
- `src/algo/global_analyzer.rs`: archive block boundary calculation using requested block size.
- `src/main.rs`, `src/types.rs`, `src/pipeline/compression.rs`: CLI/default semantics for automatic memory selection and block size.
- `tests/test_algo.rs`, `tests/test_roundtrip.rs`, `tests/test_e2e.rs`: regression coverage for block sizing, codec fallback, compressed paired-end input, length scanning, and block-base caps.
- `README.md`, `docs/guide/cli.md`, `docs/algorithms/index.md`: public documentation synchronized with CLI behavior.
- `openspec/specs/*`, `openspec/changes/closure-hardening-foundation/*`: stabilization requirements and closeout task state.
- `.github/workflows/*`, `.github/copilot-instructions.md`, `.github/instructions/openspec.instructions.md`, `.github/lsp.json`, `AGENTS.md`, `CLAUDE.md`: workflow and AI-tooling surface.
- `.gitignore`, `docs/.vitepress/dist/`, `.omc/`, `.worktrees/*`: local/build artifacts and worktree cleanup surface.

## Task 1: Stabilize Existing Behavior Changes

**Files:**
- Modify: `src/commands/compress.rs`
- Modify: `src/algo/block_compressor.rs`
- Modify: `src/algo/global_analyzer.rs`
- Modify: `src/main.rs`
- Modify: `src/types.rs`
- Modify: `src/pipeline/compression.rs`
- Test: `tests/test_algo.rs`
- Test: `tests/test_roundtrip.rs`
- Test: `tests/test_e2e.rs`

- [ ] Run the targeted tests that correspond to the current uncommitted behavior changes.

Run:

```bash
cargo test --test test_algo test_global_analyzer_respects_requested_block_size
cargo test --test test_roundtrip test_block_compress_decompress_large_short_block_falls_back_to_zstd
cargo test --test test_e2e test_e2e_archive_mode_accepts_small_input_with_low_memory_limit
cargo test --test test_e2e test_e2e_compress_decompress_gzip_paired_end_roundtrip
cargo test --test test_e2e test_e2e_scan_all_lengths_changes_length_classification
cargo test --test test_e2e test_e2e_max_block_bases_limits_long_read_block_count
```

Expected: each command passes. If a test fails, fix the implementation or the test helper that directly owns that behavior before moving on.

- [ ] Inspect `CompressCommand` defaults and ensure `memory_limit_mb: 0` means automatic selection, not no limit or a fixed 8192 MB budget.

Required code shape in `src/commands/compress.rs` and `src/types.rs`:

```rust
memory_limit_mb: 0,
block_size: 0,
```

- [ ] Ensure archive mode rejects explicit memory limits that require chunking.

Required behavior in `src/commands/compress.rs`:

```rust
if self.opts.memory_limit_mb > 0 && strategy.requires_chunking() {
    return Err(FqcError::InvalidArgument(format!(
        "--memory-limit {} MB is too small for archive mode with global analysis ({}) ; use --streaming or increase the limit",
        self.opts.memory_limit_mb,
        strategy.summary()
    )));
}
```

- [ ] Ensure short-read ABC is bounded by read count and large short-read blocks fall back to Zstd.

Required behavior in `src/algo/block_compressor.rs`:

```rust
const SHORT_READ_ABC_MAX_READS: usize = 4_096;

pub fn use_short_read_abc(&self, read_count: usize) -> bool {
    self.read_length_class == ReadLengthClass::Short && read_count <= SHORT_READ_ABC_MAX_READS
}
```

- [ ] Ensure `GlobalAnalyzer` respects caller-provided `reads_per_block`.

Required behavior in `src/algo/global_analyzer.rs`:

```rust
let effective_block_size = self.config.reads_per_block.max(1);
result.block_boundaries = self.compute_block_boundaries(total_reads, effective_block_size);
```

## Task 2: Repository Documentation And OpenSpec Cleanup

**Files:**
- Modify: `README.md`
- Modify: `docs/guide/cli.md`
- Modify: `docs/algorithms/index.md`
- Modify: `docs/index.md`
- Modify: `docs/.vitepress/config.mts`
- Modify: `openspec/changes/closure-hardening-foundation/tasks.md`
- Modify or delete: `CODE_OF_CONDUCT.md`
- Delete if tracked: `docs/.vitepress/dist/**`

- [ ] Verify root docs and docs site describe `--memory-limit 0` as automatic memory selection.

Run:

```bash
rg "memory-limit|automatic memory|auto-detect|no limit|8192" README.md docs openspec src/main.rs src/commands/compress.rs src/types.rs src/pipeline/compression.rs
```

Expected: no public docs describe `--memory-limit 0` as `no limit`; no defaults advertise `8192` MB.

- [ ] Replace any generic or incomplete root policy content with project-specific concise text.

If `CODE_OF_CONDUCT.md` contains placeholders such as `[INSERT CONTACT METHOD]`, rewrite it as a short repository conduct note that names this project and points private reports to `SECURITY.md`. Do not keep generic boilerplate.

- [ ] Remove committed VitePress build artifacts if they are tracked.

Run:

```bash
git ls-files docs/.vitepress/dist
```

Expected: no output after cleanup. If output exists, remove those tracked files from the index and working tree because `.gitignore` already excludes `docs/.vitepress/dist/`.

- [ ] Keep the docs site concise and portal-oriented.

Required docs navigation remains limited to quick start, CLI, architecture, algorithms, release notes, and GitHub unless OpenSpec adds another explicit requirement.

## Task 3: Workflow And AI Tooling Reduction

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`
- Modify: `.github/workflows/pages-vitepress.yml`
- Keep or justify: `.github/workflows/copilot-setup-steps.yml`
- Modify: `.github/copilot-instructions.md`
- Modify: `.github/instructions/openspec.instructions.md`
- Modify: `AGENTS.md`
- Modify: `CLAUDE.md`

- [ ] Reconcile workflow policy with OpenSpec.

Check `openspec/specs/release-operations/spec.md` before deleting `.github/workflows/copilot-setup-steps.yml`. Current living spec requires Copilot cloud setup, so either keep the workflow and ensure it has a clear purpose, or update OpenSpec first.

- [ ] Ensure no workflow auto-creates version branches.

Run:

```bash
rg "create.*branch|version.*branch|git checkout -b|pull_request_target|peter-evans/create-pull-request|release-please" .github/workflows
```

Expected: no matches for automatic version branch creation.

- [ ] Keep AI guidance layered instead of duplicated.

Required policy:

```text
AGENTS.md is canonical.
CLAUDE.md adds Claude-specific behavior only.
.github/copilot-instructions.md adds Copilot-specific behavior only and points back to AGENTS.md.
.github/instructions/openspec.instructions.md stays a short OpenSpec reminder.
```

- [ ] Keep LSP configuration minimal and Rust-focused.

Required `.github/lsp.json` scope: Rust analyzer and TOML support only unless the docs stack requires another language server.

## Task 4: Worktree And Branch Closeout

**Files:**
- Read-only first: `.worktrees/*`
- Git refs: local branches and worktrees

- [ ] Inspect every worktree status before merge or deletion.

Run:

```bash
git worktree list --porcelain
```

Then run `git status --short --branch` inside each listed worktree.

Expected: classify each worktree as clean and merged, clean and ahead, dirty, or duplicate of `master`.

- [ ] For clean worktrees whose branch is fully merged into `master`, remove the worktree and delete the local branch.

Safe commands only after verification:

```bash
git worktree remove .worktrees/<name>
git branch -d <branch>
```

- [ ] For dirty or unmerged worktrees, do not delete. Summarize the branch, commits ahead of `master`, and conflicting files.

Run:

```bash
git log --oneline master..<branch>
git diff --stat master...<branch>
```

Expected: only merge branches whose changes are understood and validated.

## Task 5: Full Verification

**Files:**
- Entire repository

- [ ] Run formatting check.

Run:

```bash
cargo fmt --all -- --check
```

Expected: pass. If it fails, run `cargo fmt --all`, then rerun the check.

- [ ] Run Clippy.

Run:

```bash
cargo clippy --all-targets -- -D warnings
```

Expected: pass without warnings.

- [ ] Run tests.

Run:

```bash
cargo test --lib --tests
```

Expected: all library and integration tests pass.

- [ ] Build Rust documentation.

Run:

```bash
cargo doc --no-deps
```

Expected: documentation builds successfully.

- [ ] Build VitePress documentation.

Run:

```bash
npm run docs:build
```

Expected: VitePress build succeeds.

## Task 6: Commit, Push, And Metadata

**Files:**
- Git index and GitHub repository metadata

- [ ] Review all diffs before staging.

Run:

```bash
git status --short --branch
git diff --stat
git diff --cached --stat
```

Expected: no secrets, no accidental build artifacts, no unknown generated files except intentional cleanup.

- [ ] Commit cohesive closeout changes with the existing Conventional Commits style.

Suggested commit message:

```text
chore: complete repository stabilization closeout
```

- [ ] Push only after local validation passes and the commit is reviewed.

Run:

```bash
git push origin master
```

Expected: remote `master` receives the local closeout commit(s).

- [ ] Update GitHub repository metadata if `gh` is authenticated and repository permissions allow it.

Run:

```bash
gh repo edit LessUp/fq-compressor-rust \
  --description "Rust FASTQ compressor with a block-indexed .fqc archive format" \
  --homepage "https://lessup.github.io/fq-compressor-rust/" \
  --add-topic rust \
  --add-topic fastq \
  --add-topic bioinformatics \
  --add-topic compression \
  --add-topic genomics
```

Expected: GitHub About metadata matches `openspec/specs/repository-metadata/spec.md`.

## Self-Review

- Spec coverage: project governance, developer workflow, docs-site, release operations, repository metadata, and CLI surface all map to tasks above.
- Placeholder scan: this plan contains no `TBD`, no unconstrained `TODO`, and no empty implementation slots.
- Type consistency: Rust names used here match the current code paths and test names observed in the repository.

## Worktree Audit Notes

- `perf-memory-limit-auto`: clean worktree; its `--memory-limit 0` documentation and test intent is covered by the current main worktree changes.
- `perf-bench-foundation`: clean worktree with complete benchmark OpenSpec and passing reported validation, but it adds Criterion and benchmark surface area. Defer unless the maintainer explicitly wants performance infrastructure in this closeout slice.
- `perf-bench-foundation-tdd`: clean older benchmark slice superseded by `perf-bench-foundation`; do not merge both.
- `perf-foundation-docs`: clean documentation slice that is small and coherent, but it adds a performance roadmap page. Defer to keep the current docs site focused on shipped behavior.
- `feature/real-data-validation`: dirty worktree with unfinished OpenSpec tasks, hard-coded local data path assumptions, and likely conflicts in `src/commands/compress.rs`; keep separate until main stabilizes.
- `final-closure-pass`: dirty worktree with an untracked closure summary; keep separate unless its summary is explicitly requested for migration.
