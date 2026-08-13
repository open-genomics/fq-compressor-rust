# Design: make-file-output-atomic

## Approach

Introduce one internal `OutputTransaction` type used by every ordinary-file
writer path:

1. If the final path exists and `--force` is false → fail before creating temps.
2. Create a `NamedTempFile` in the final path's parent directory (same filesystem
   for rename).
3. Hand the owned `File` to `FqcWriter::from_file` or a `BufWriter` for FASTQ.
4. On success: flush/close the writer, then `persist` (rename) onto the final path.
5. On failure or drop without commit: temp file is deleted; final path untouched.

Split-PE uses two transactions. Commit order is R1 then R2. If the second
persist fails after the first succeeded, restore best-effort by renaming any
pre-force backup is not guaranteed on POSIX; document the limitation and keep
both temps until both commits succeed when possible (commit R1, then R2; on R2
failure leave R1 already replaced — documented platform limit).

Stdout (`"-"`) bypasses the transaction entirely.

## Allowed surface

- `Cargo.toml` (move `tempfile` to runtime dependency)
- `src/io/output_transaction.rs`, `src/io/mod.rs`
- `src/archive/writer.rs` (`from_file`)
- `src/engine/compression_engine.rs`
- `src/pipeline/compression.rs`, `src/pipeline/decompression.rs`
- `src/commands/decompress.rs`, compress option plumbing if needed
- Tests under `tests/`
- `CHANGELOG.md`, brief README note if overwrite semantics are documented

## Non-goals

- Exact Rust allocator accounting
- Changing archive bytes or CLI flag names
