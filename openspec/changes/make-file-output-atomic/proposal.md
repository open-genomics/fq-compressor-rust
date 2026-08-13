# Change Proposal: make-file-output-atomic

## Metadata

- Status: `Applying`
- Repository: `open-genomics/fq-compressor-rust`
- Capability: `file-output`
- Task IDs: `FQCR-IO-001`
- Decision IDs: (none; follows maintenance-design/02 §5)

## Why

Ordinary, streaming, and pipeline writers call `File::create` / `FqcWriter::create`
on the final destination path. With `--force`, an existing archive is truncated
before compression finishes; on mid-run failure the final path holds a partial
archive that looks complete. Decompress split-PE creates both targets eagerly
with the same risk.

## Changes

**Transactional ordinary-file output**

- From: writers truncate/create the final path immediately.
- To: writers write same-directory temporary files and rename only after a
  successful flush/close. Failure leaves a missing target unchanged, or keeps
  the previous file when `--force` was used.
- Reason: fail closed for user data; match the C++ sequential tool's atomic
  replace policy.
- Impact: no format/CLI/product changes; stdout remains non-transactional.

## Scope

- New `src/io/output_transaction.rs` (single abstraction);
- All compression engine/pipeline writer creation sites;
- Decompress ordinary file and split-PE outputs;
- Decompression pipeline ordinary file output;
- Unit/integration tests for success, mid-failure, force, and split-PE;
- Docs/CHANGELOG note about platform split-PE rename limits.

## Out of scope

- Format family recognition / CLI identity;
- Codec dispatch / memory budget;
- stdout rollback;
- Distributed/cross-filesystem atomic multi-file commit;
- Product rename or suffix changes.

## Compatibility and rollback

- Existing archives and CLI flags unchanged.
- Rollback: revert the change; writers again create final paths directly.
