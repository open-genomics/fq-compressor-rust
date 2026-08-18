# Change Proposal: recognize-sequential-fqc-family

## Metadata

- Status: `Archived`
- Task IDs: `FQC-FAMILY-001` (Rust)
- Prerequisites: `FQCR-SPEC-001` / format family docs

## Why

Rust reader treated the C++ sequential magic as a generic bad magic. Users with
the wrong `fqc` on `PATH` need a clear “other known family” error pointing at
`open-genomics/fq-compressor`.

## Changes

Classify the first 8 bytes before version/header parsing; reject
`fqc-sequential/v2` with a locked unsupported-family message; keep unknown and
truncated magics distinct. Cover info/verify/decompress entry points.

## Out of scope

Decoding sequential archives, CLI identity fields, product rename.
