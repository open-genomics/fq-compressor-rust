# Final Closeout Design

## Overview

This document defines the design for the final closeout of `fqc`, transitioning from active development to a stable, archived state.

## Current State Analysis

### Repository Health

| Aspect | Status | Evidence |
|--------|--------|----------|
| Code Quality | ✅ Good | `cargo fmt`, `clippy`, tests all pass |
| OpenSpec Compliance | ✅ Complete | All 3 changes fully implemented |
| Worktrees | ⚠️ 3 Active | `feature/real-data-validation`, `perf-bench-foundation`, `perf-foundation-docs` |
| Branch Hygiene | ⚠️ Needs Cleanup | 4 local branches, only `master` on remote |
| Documentation | ✅ Sufficient | VitePress site operational, README accurate |
| CI/CD | ✅ Minimal | 4 workflows, each with clear purpose |

### Worktree Status

| Branch | Commit | Tasks | Action |
|--------|--------|-------|--------|
| `perf-bench-foundation` | `4184e90` | ✅ All complete | Merge |
| `perf-foundation-docs` | `b248bb8` | ✅ All complete | Merge |
| `feature/real-data-validation` | `7f771e1` | ⚠️ 3/6 blocked by external data | Close |

### OpenSpec Changes

| Change | Status | Tasks |
|--------|--------|-------|
| `closure-hardening-foundation` | ✅ Complete | 10/10 |
| `benchmark-foundation` | ✅ Complete | 3/3 |
| `performance-foundation-phase-1` | ✅ Complete | 5/5 |

## Design

### Phase 1: Branch Consolidation

**Goal**: Merge completed worktrees, close blocked worktree, achieve single-branch state.

**Steps**:
1. Merge `perf-bench-foundation` into `master`
2. Merge `perf-foundation-docs` into `master`
3. Close `feature/real-data-validation` with documentation of blockers
4. Delete all local worktrees
5. Delete all local branches except `master`
6. Force push `master` to ensure remote is clean

**Rationale**:
- `perf-bench-foundation` and `perf-foundation-docs` have complete task lists
- `feature/real-data-validation` requires external data (`FQC_REAL_DATA_DIR`) which violates YOLO autonomous execution
- Single-branch state aligns with "极简分支流" requirement

### Phase 2: Repository Hygiene

**Goal**: Clean up any remaining drift, ensure all specs and docs are accurate.

**Steps**:
1. Run full validation suite
2. Verify all OpenSpec specs match current implementation
3. Ensure `CHANGELOG.md` reflects recent work
4. Verify VitePress site builds and deploys correctly

**Rationale**: Final sanity check before archival.

### Phase 3: GitHub Metadata Update

**Goal**: Update GitHub repository metadata to reflect final state.

**Steps**:
1. Update repository description via `gh`
2. Set repository topics
3. Ensure GitHub Pages URL is configured
4. Disable GitHub Projects/Wikis if not used

**Rationale**: Improves discoverability and reflects project status.

### Phase 4: Final Verification

**Goal**: Confirm project meets "工业级稳定标准".

**Steps**:
1. Run all validation commands
2. Build release binary locally
3. Test basic compression/decompression workflow
4. Verify docs site is accessible

**Rationale**: Evidence before assertions.

## Non-Goals

- Adding new features
- Refactoring working code
- Addressing `feature/real-data-validation` blockers (requires external data)

## Success Criteria

1. Only `master` branch exists locally and remotely
2. All validation commands pass
3. GitHub metadata is accurate
4. Documentation is current
5. No open worktrees

## Timeline

Single session execution - approximately 20-30 minutes.

---

**Self-Review**: ✅ Passed - No placeholders, consistent with exploration findings, focused scope, actionable steps.
