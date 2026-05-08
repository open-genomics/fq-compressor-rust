# deepen-compression-orchestration

## Why

Compression orchestration is currently spread across multiple shallow modules and duplicated paths:

- `src/commands/compression_strategy.rs` is a pass-through seam that only forwards to large methods on `CompressCommand`
- `src/commands/compress.rs` mixes CLI details, input topology handling, length profiling, mode selection, writer setup, and execution
- `src/pipeline/compression.rs` duplicates ingest and archive-writing decisions that also exist in archive and streaming paths
- the maintained CLI docs imply `--pipeline` can be paired with `--streaming`, but the current implementation treats them as distinct modes

## What changes

- introduce one deep compression orchestration module with a normalized compression request and a richer compression outcome
- keep `CompressCommand` as the CLI seam and move planning/execution policy behind the shared orchestration seam
- preserve current compression mode semantics: archive, streaming, and pipeline remain distinct execution modes
- centralize FASTQ input topology handling and Read ordering so paired/interleaved behavior does not leak across multiple callers
- add interface-level tests for the orchestration seam and align CLI/docs wording with the actual mode behavior

## Non-goals

- changing the `.fqc` archive format
- changing compression algorithms or codec choices
- introducing a true streaming pipeline in this slice
- expanding the CLI surface beyond clarifying documentation to match current behavior
