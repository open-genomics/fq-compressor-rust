# Capability: benchmark-foundation

## Requirement: benchmark coverage must stay intentionally small

The repository MUST keep performance measurement focused on a small set of hot paths instead of growing a broad benchmarking platform.

### Scenario: a contributor adds or updates benchmarks

- **WHEN** benchmark targets are introduced or revised
- **THEN** they cover parser throughput and archive lifecycle hot paths with no more than three benchmark targets total
- **AND** they reuse repository-local fixtures or generated data instead of large checked-in benchmark inputs
- **AND** they do not change the public CLI surface just to support benchmarking

## Requirement: benchmark tooling must stay stable-compatible

Benchmark execution MUST work on the project's stable Rust toolchain.

### Scenario: local benchmark tooling is reviewed

- **WHEN** the repository depends on a benchmark framework
- **THEN** the dependency is limited to the smallest reasonable stable-compatible option for this project
- **AND** benchmark targets compile through `cargo bench --no-run`
