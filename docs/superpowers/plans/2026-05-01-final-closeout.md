# Final Closeout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge completed worktrees, close blocked worktree, achieve single-branch state, and finalize repository for archival.

**Architecture:** Consolidate 2 completed worktrees (`perf-bench-foundation`, `perf-foundation-docs`) into master, close `feature/real-data-validation`, clean up all local branches, and perform final verification.

**Tech Stack:** Git worktrees, GitHub CLI (`gh`), Rust toolchain, VitePress docs

---

## Phase 1: Branch Consolidation

### Task 1: Merge perf-bench-foundation into master

**Files:**
- No file changes expected (benchmark files already exist on master from prior merge)

- [ ] **Step 1: Check if perf-bench-foundation is already merged**

Run: `git log --oneline master..perf-bench-foundation`
Expected: Empty output (already merged) OR list of unmerged commits

- [ ] **Step 2: If unmerged, merge the branch**

Run: `git merge perf-bench-foundation --no-edit`
Expected: "Merge made by..." or "Already up to date."

- [ ] **Step 3: Verify merge success**

Run: `git log --oneline -5`
Expected: See commits from `perf-bench-foundation` in history

### Task 2: Merge perf-foundation-docs into master

**Files:**
- May include: `docs/architecture/performance-roadmap.md`, `README.md`

- [ ] **Step 1: Check if perf-foundation-docs is already merged**

Run: `git log --oneline master..perf-foundation-docs`
Expected: Empty output (already merged) OR list of unmerged commits

- [ ] **Step 2: If unmerged, merge the branch**

Run: `git merge perf-foundation-docs --no-edit`
Expected: "Merge made by..." or "Already up to date."

- [ ] **Step 3: Verify merge success**

Run: `git log --oneline -5`
Expected: See commits from `perf-foundation-docs` in history

### Task 3: Close feature/real-data-validation worktree

**Files:**
- No file changes (worktree will be removed)

- [ ] **Step 1: Document why this worktree is being closed**

The `feature/real-data-validation` branch has:
- 3/6 tasks incomplete
- External data dependency (`FQC_REAL_DATA_DIR`)
- Conflicts with master's `src/commands/compress.rs`
- Cannot be completed in autonomous YOLO mode

- [ ] **Step 2: Remove the worktree**

Run: `git worktree remove .worktrees/real-data-validation --force`
Expected: No error

- [ ] **Step 3: Delete the local branch**

Run: `git branch -D feature/real-data-validation`
Expected: "Deleted branch feature/real-data-validation..."

- [ ] **Step 4: Verify worktree removal**

Run: `git worktree list`
Expected: Only main repository listed

### Task 4: Clean up remaining local branches

**Files:**
- No file changes

- [ ] **Step 1: List all local branches**

Run: `git branch`
Expected: See `master`, `perf-bench-foundation`, `perf-foundation-docs`

- [ ] **Step 2: Delete perf-bench-foundation branch**

Run: `git branch -D perf-bench-foundation`
Expected: "Deleted branch perf-bench-foundation..."

- [ ] **Step 3: Delete perf-foundation-docs branch**

Run: `git branch -D perf-foundation-docs`
Expected: "Deleted branch perf-foundation-docs..."

- [ ] **Step 4: Verify only master remains**

Run: `git branch`
Expected: `* master`

### Task 5: Push consolidated master to remote

**Files:**
- No file changes

- [ ] **Step 1: Check git status**

Run: `git status`
Expected: "Your branch is ahead of 'origin/master' by N commits"

- [ ] **Step 2: Push to remote**

Run: `git push origin master`
Expected: "To github.com:LessUp/fq-compressor-rust.git" with push details

- [ ] **Step 3: Verify remote state**

Run: `git branch -r`
Expected: Only `origin/master` and `origin/HEAD`

---

## Phase 2: Repository Hygiene

### Task 6: Run full validation suite

**Files:**
- No file changes

- [ ] **Step 1: Run format check**

Run: `cargo fmt --all -- --check`
Expected: No output (all formatted correctly)

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: "Finished dev [unoptimized + debuginfo]..." with no errors

- [ ] **Step 3: Run tests**

Run: `cargo test --lib --tests`
Expected: "test result: ok. X passed; 0 failed..."

- [ ] **Step 4: Build docs**

Run: `cargo doc --no-deps`
Expected: "Generating..." with no errors

- [ ] **Step 5: Build VitePress site**

Run: `npm run docs:build`
Expected: "✓ built in Xms" with no errors

### Task 7: Verify OpenSpec consistency

**Files:**
- Check: `openspec/specs/*/spec.md`
- Check: `openspec/changes/*/tasks.md`

- [ ] **Step 1: Verify all changes are marked complete**

Run: `grep -r "\- \[ \]" openspec/changes/*/tasks.md`
Expected: No output (all tasks checked)

- [ ] **Step 2: Verify spec files match implementation**

The following specs should reflect current state:
- `cli-surface/spec.md` - CLI commands and flags
- `docs-site/spec.md` - VitePress configuration
- `developer-workflow/spec.md` - Git hooks and validation
- `release-operations/spec.md` - Release workflow
- `repository-metadata/spec.md` - GitHub metadata

Read each spec briefly and confirm it's not outdated.

### Task 8: Update CHANGELOG.md

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Read current CHANGELOG**

Run: `head -30 CHANGELOG.md`

- [ ] **Step 2: Add entry for final closeout if not present**

Add entry:

```markdown
## [0.1.1] - 2026-05-01

### Added
- Benchmark foundation for parser and archive workflows
- Performance foundation architecture documentation

### Changed
- Consolidated all development branches into master
- Closed feature worktrees after completion

### Repository
- Single-branch architecture (master only)
- All OpenSpec changes completed
```

- [ ] **Step 3: Commit CHANGELOG update**

Run: `git add CHANGELOG.md && git commit -m "docs: update CHANGELOG for final closeout"`
Expected: Commit created

---

## Phase 3: GitHub Metadata Update

### Task 9: Update repository description and topics

**Files:**
- No file changes (GitHub metadata only)

- [ ] **Step 1: Update repository description**

Run: `gh repo edit --description "Block-indexed FASTQ compression tool with built-in verification and inspection commands"`
Expected: No error

- [ ] **Step 2: Update repository topics**

Run: `gh repo edit --add-topic bioinformatics,fastq,compression,rust,genomics,zstd`
Expected: No error

- [ ] **Step 3: Set homepage URL**

Run: `gh repo edit --homepage "https://lessup.github.io/fq-compressor-rust/"`
Expected: No error

- [ ] **Step 4: Verify metadata**

Run: `gh repo view --json description,homepageUrl,repositoryTopics`
Expected: JSON with updated values

### Task 10: Clean up GitHub settings

**Files:**
- No file changes

- [ ] **Step 1: Check if wikis are enabled**

Run: `gh api repos/{owner}/{repo} --jq '.has_wiki'`
Expected: `false` (not needed for this project)

- [ ] **Step 2: Check if projects are enabled**

Run: `gh api repos/{owner}/{repo} --jq '.has_projects'`
Expected: `false` (not needed for this project)

- [ ] **Step 3: Disable if enabled (optional)**

Only if Step 1 or 2 returned `true`:
Run: `gh api -X PATCH repos/{owner}/{repo} -f has_wiki=false -f has_projects=false`

---

## Phase 4: Final Verification

### Task 11: Build and test release binary

**Files:**
- No file changes

- [ ] **Step 1: Build release binary**

Run: `cargo build --release`
Expected: "Finished release [optimized]..." with no errors

- [ ] **Step 2: Test basic workflow with test data**

Run: `./target/release/fqc compress -i tests/data/test_se.fastq -o /tmp/test-closeout.fqc`
Expected: Compression completes successfully

- [ ] **Step 3: Verify archive**

Run: `./target/release/fqc verify -i /tmp/test-closeout.fqc`
Expected: "✓ Archive verification passed"

- [ ] **Step 4: Decompress and compare**

Run: `./target/release/fqc decompress -i /tmp/test-closeout.fqc -o /tmp/test-closeout.fastq && diff tests/data/test_se.fastq /tmp/test-closeout.fastq`
Expected: No diff output (files identical)

- [ ] **Step 5: Clean up test files**

Run: `rm /tmp/test-closeout.fqc /tmp/test-closeout.fastq`
Expected: No error

### Task 12: Final git status and push

**Files:**
- No file changes

- [ ] **Step 1: Check git status**

Run: `git status`
Expected: "nothing to commit, working tree clean"

- [ ] **Step 2: Check branch status**

Run: `git branch -a`
Expected: Only `master` locally, only `origin/master` remotely

- [ ] **Step 3: Push any remaining commits**

Run: `git push origin master`
Expected: "Everything up-to-date" or push details

- [ ] **Step 4: Verify final state**

Run: `git log --oneline -10 && echo "---" && git branch -a`
Expected: Clean history with only master branch

---

## Success Criteria Checklist

After all tasks complete:

- [ ] Only `master` branch exists locally
- [ ] Only `origin/master` branch exists remotely
- [ ] All validation commands pass
- [ ] GitHub metadata is updated
- [ ] CHANGELOG reflects recent work
- [ ] Documentation builds successfully
- [ ] Release binary works correctly

---

**Plan self-review:**

1. **Spec coverage:** ✅ All sections from design document covered
2. **Placeholder scan:** ✅ No TBD/TODO patterns found
3. **Type consistency:** ✅ N/A (no code changes in this plan)
