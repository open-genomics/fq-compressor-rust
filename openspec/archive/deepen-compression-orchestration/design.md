# Design

## Decision 1: keep `CompressCommand` as the CLI seam

`CompressCommand` should continue to own CLI-facing validation, logging, summary printing, and exit-code shaping. It should stop owning compression policy. Instead, it will normalize `CompressOptions` into a shared compression request and hand that request to one deeper orchestration module.

## Decision 2: normalize compression input before execution

The new orchestration seam should accept a normalized request that captures:

- explicit execution mode (`archive`, `streaming`, `pipeline`)
- input topology (single-end file, paired-end files, interleaved paired-end file, or stdin)
- compression policy (quality mode, ID mode, read-length mode, block sizing, memory limit, overwrite behavior)
- paired-end archive layout metadata

This keeps clap-shaped flags and command-specific branching out of the orchestration implementation while preserving current user-visible behavior.

## Decision 3: keep execution backends as internal adapters

Archive, streaming, and pipeline execution paths should remain separate internal adapters behind the deepened seam. They have different implementation constraints, but callers should not need to branch on those constraints directly. The deleted `compression_strategy` trait does not earn its keep because it provides no leverage beyond forwarding.

## Decision 4: return a richer compression outcome

The orchestration seam should return a compression outcome that includes:

- `ProcessingStats`
- selected execution mode
- detected `ReadLengthClass`
- whether a `Reorder Map` was written
- block counts and related archive decisions

This allows command code and tests to assert on compression decisions without reopening the archive just to recover them.

## Decision 5: preserve existing mode semantics and fix docs drift

This slice should not change compression mode behavior. In current code, `--streaming` wins over `--pipeline`, and pipeline mode still performs a full ingest. The refactor should preserve that behavior and update README and `docs/guide/cli.md` so the docs stop implying a streaming pipeline that does not exist.
