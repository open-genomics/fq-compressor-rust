# Design: enforce-decode-resource-budget

## Approach

Introduce an operation-scoped `DecodeBudget` (no global mutable state):

1. Resolve `--memory-limit` MB (`0` → ~75% of available RAM, clamped to
   `[MIN_DECODE_MEMORY_MB, HARD_MAX_DECODE_MEMORY_MB]`).
2. Pass the budget into `FqcReader::open_with_budget` and use it for:
   - `total_read_count` / index `num_blocks` / file-region capacity
   - stream allocations (`check_alloc` + checked `u64 → usize`)
   - reorder map compressed and decompressed ceilings
   - per-block decode peak (`compressed_size * 3`)
3. Replace unbounded `zstd::stream::decode_all` with
   `zstd_decompress_bounded(max_out)` in stream codecs and reorder maps.
4. Original-order: `check_original_order_peak` before creating outputs.
5. Parallel / pipeline batch size = `min(threads*2, budget_batches)`, never 0;
   insufficient budget for one block → `ResourceLimit`.
6. Verify and decompress share the same resolve path; `--quick` skips block
   decode but still opens under the budget.

## Allowed surface

- `src/memory_budget.rs`, `src/error.rs` (`ResourceLimit`)
- `src/archive/{format,reader}.rs`
- `src/algo/*` zstd decompress call sites
- `src/commands/{decompress,verify}.rs`, `src/main.rs`
- `src/pipeline/decompression.rs`
- Tests, CHANGELOG, README / CLI guide, this OpenSpec change

## Non-goals

- Exact allocator accounting
- Fuzz harness as the primary gate
- Format-family recognition / CLI identity changes
