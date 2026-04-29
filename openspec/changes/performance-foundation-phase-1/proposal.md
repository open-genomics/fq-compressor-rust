# performance-foundation-phase-1

## Why

The repository needs one durable place to explain the current performance story without turning the public docs into a long research dump.

Today the codebase already shows the main pressure points:

- short-read compression still depends on a global analysis and optional reorder pass before archive writing
- lower-memory and pipeline modes exist, but the preferred direction is not summarized anywhere public
- contributor guidance for performance work needs smaller, worktree-scoped slices instead of broad autonomous exploration
- `--memory-limit 0` needs an explicit target meaning so a later implementation slice can align code and docs

## What changes

- add a bounded OpenSpec change for the first performance-foundation docs slice
- publish a concise architecture page that summarizes bottlenecks, direction, phase-1 scope, and deferred follow-up
- wire the page into architecture navigation and point the README at the maintained summary
- define bounded worktree-based workflow expectations for future performance slices
- record that `--memory-limit 0` means automatic memory selection

## Non-goals

- shipping runtime performance changes in this slice
- copying the full research report into the repository
- expanding the docs site beyond a small architecture summary
