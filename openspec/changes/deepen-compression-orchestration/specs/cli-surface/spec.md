# cli-surface change

## ADDED Requirements

### Requirement: compression execution mode docs must match implementation semantics

The maintained docs MUST describe archive, streaming, and pipeline compression as distinct modes and MUST not imply that `--pipeline` provides strict low-memory streaming unless the implementation actually supports that behavior.

#### Scenario: compression mode docs are updated

- **WHEN** README or `docs/guide/cli.md` describe `--streaming` and `--pipeline`
- **THEN** the wording matches `src/main.rs` and `src/commands/compress.rs`
- **AND** low-memory guidance points users to `--streaming` without claiming that pipeline mode changes the ingest model
