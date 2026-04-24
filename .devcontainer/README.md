# Dev container

This container is tuned for the current `fqc` workflow:

- Rust **1.75.0**
- Node **20** for VitePress
- `bacon`, `cargo-deny`, and `taplo-cli`
- repository-local Git hooks

## Lifecycle

- `postCreateCommand`: marks the workspace as a safe Git directory, installs npm dependencies, fetches Rust crates, and enables `.githooks/`
- `postStartCommand`: refreshes the Git safe-directory setting

## Core commands

```bash
cargo test --lib --tests
cargo clippy --all-targets -- -D warnings
npm run docs:build
bash scripts/validate.sh full
```
