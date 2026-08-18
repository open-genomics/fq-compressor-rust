# Change Proposal: enforce-compress-archive-budget

## Metadata

- Status: `Archived`
- Task IDs: `FQCR-LIMIT-002`
- Prerequisites: `FQCR-LIMIT-001` (decode budget)

## Why

`--memory-limit 0` is finite for decompress/verify (`DecodeBudget`) but archive
compress still skips the ingest check. A default run can load the entire FASTQ
before any budget applies. Docs now call this out; the implementation must match.

## Changes

Resolve compress `--memory-limit` (`0` = 75% of available, hard-capped) into a
finite budget. During archive ingest, estimate held records and fail with
`ResourceLimit` before creating the output file; hint `--streaming`. Tiny
fixtures must still succeed under automatic and `--memory-limit 16`.

## Out of scope

Exact allocator accounting, making pipeline a strict low-memory mode, new
codecs, changing default execution mode.
