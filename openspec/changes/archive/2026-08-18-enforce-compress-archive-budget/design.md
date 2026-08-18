# Design: enforce-compress-archive-budget

Reuse the decode resolver shape, do not invent a second policy language.

- `resolve_compress_limit_mb(0)` = 75% of `get_available_memory_mb()`, clamped
  to `[MIN_COMPRESS_MEMORY_MB, HARD_MAX_COMPRESS_MEMORY_MB]` (16 MB .. 512 GB).
- Explicit `N > 0` uses `N`, then the same clamp.
- Per-record estimate: `id + seq + qual + 128` bytes. Archive peak multiplies
  by 2 to cover the extra sequence/block copies in `run_archive`.
- Check incrementally while reading; writer is opened only after ingest.
- Streaming does not use this ingest check.
- Fail with `FqcError::ResourceLimit` (`location` names archive ingest).
