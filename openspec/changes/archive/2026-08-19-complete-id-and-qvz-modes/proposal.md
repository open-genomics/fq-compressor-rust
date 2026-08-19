# Change Proposal: complete-id-and-qvz-modes

## Metadata

- Status: `Archived`
- Task IDs: `FQCR-CLI-001`
- Prerequisites: none

## Why

`--lossy-quality qvz` is a lossless alias, and `IdMode` is not a CLI flag.
Callers cannot choose exact / tokenize / discard IDs, and `qvz` does not
quantize. Half-exposed switches are worse than missing ones.

## Changes

- Add `--id-mode exact|tokenize|discard` (default `tokenize`, matching today's
  auto-tokenize compressor). Exact forces the exact stream; discard is
  unchanged.
- Make `qvz` a distinct 8-level nearest-neighbor quality quantizer (not
  Illumina8 bins, not lossless). Flags and `fqc info` report the requested mode.

## Out of scope

Full QVZ rate-distortion codebook training; new codec family bytes (still SCM
after quantization); changing the frozen indexed-v2 fixture.
