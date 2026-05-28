# Dev container

This container is tuned for the current `fqc` workflow:

- Rust **1.75.0**
- Node **24**
- `taplo` via `taplo-cli 0.10.0`
- repository-local Git hooks
- named volumes for Cargo registry, Cargo git, `target/`, and npm cache

## Lifecycle

- `postCreateCommand`: marks the workspace as a safe Git directory, requires the repository hook setup script, fetches Rust crates, and runs `npm ci`
- `postStartCommand`: re-applies safe Git and hook configuration without re-running dependency installs

Bootstrap failures are surfaced to the caller instead of being ignored.

## Core commands

```bash
cargo test --lib --tests
cargo clippy --all-targets -- -D warnings
npm run docs:build
bash scripts/validate.sh full
```
