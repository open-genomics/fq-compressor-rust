# Design

## Decision 1: keep the public performance write-up short and architectural

The docs site should publish one architecture page that explains the current bottlenecks and recommended direction in durable terms. It should summarize the code shape and near-term plan, not embed benchmark logs, speculative tuning ideas, or raw research notes.

## Decision 2: treat phase 1 as a documentation-and-contract slice

This slice does not change runtime behavior. Instead, it establishes the maintained summary that future work can point to, defines the contributor workflow for bounded performance slices, and records the intended `--memory-limit 0` semantics for a later implementation change.

## Decision 3: keep performance work reviewable with isolated worktrees

Performance work can sprawl quickly. The contributor workflow should prefer one OpenSpec-bounded slice per dedicated worktree so docs, code, and follow-up implementation remain easy to review and verify.

## Decision 4: use navigation, not duplication

The new page belongs under architecture because it explains where the current hot paths are and how the next phases should proceed. README should point readers to that maintained summary instead of repeating the full roadmap inline.
