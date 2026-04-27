# benchmark-foundation

## Why

Optimization work in `fqc` should be driven by measured evidence before any larger parser or pipeline changes. The repository currently has no benchmark surface, so contributors lack a stable way to compare parser and archive-path performance.

## What changes

- add a small stable-compatible benchmark harness for parser and archive hot paths
- generate benchmark inputs from existing test fixtures instead of adding large checked-in data
- record the benchmark foundation in OpenSpec so the surface stays intentionally small

## Non-goals

- redesigning parser, compression, or verification behavior
- expanding the public CLI or docs surface
- building a broad benchmarking platform or fixture corpus
