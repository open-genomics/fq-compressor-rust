# Tasks

- [ ] Create the `deepen-compression-orchestration` change with proposal, design, tasks, and scoped spec deltas.
- [ ] Add a shared compression orchestration module that owns normalized request handling and returns a compression outcome.
- [ ] Route archive, streaming, and pipeline execution through the shared orchestration seam while keeping `CompressCommand` as the CLI seam.
- [ ] Remove the shallow `compression_strategy` module and keep mode selection as explicit data on the normalized request.
- [ ] Add interface-level tests for request normalization, execution-mode semantics, and compression outcomes; keep CLI end-to-end coverage.
- [ ] Update README, `docs/guide/cli.md`, and architecture docs so streaming and pipeline guidance matches the implementation.
- [ ] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib --tests`, `cargo doc --no-deps`, and `npm run docs:build`.
