# Change Proposal: enforce-decode-resource-budget

## Metadata

- Status: `Completed`
- Task IDs: `FQCR-LIMIT-001`
- Prerequisites: `FQCR-SPEC-001`, `FQCR-CODEC-001`

## Why

`--memory-limit` only constrained compression. Archive readers allocate from
attacker-controlled sizes (`num_blocks`, stream sizes, reorder maps) and use
unbounded `zstd::decode_all`, so damaged/malicious archives can OOM.

## Changes

Wire an operation-scoped `DecodeBudget` through open/read/reorder/decode and
CLI decompress/verify; reject oversize claims before allocation; bound zstd
output; keep automatic mode finite with hard structural caps.

## Out of scope

Exact allocator accounting, fuzz infrastructure, format family recognition.
