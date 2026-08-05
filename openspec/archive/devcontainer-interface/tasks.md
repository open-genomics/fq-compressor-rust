# Tasks

- [x] Update `.devcontainer/Dockerfile` and `.devcontainer/devcontainer.json` to pin Rust `1.75.0`, Node.js `24`, and the minimum guaranteed helper tool surface (`taplo-cli 0.10.0`).
- [x] Keep `.devcontainer/scripts/container-setup.sh` as the shared `postCreateCommand`/`postStartCommand` bootstrap path, with visible bootstrap failures and idempotent start-safe repair.
- [x] Declare persistent named volumes for Cargo registry, Cargo git, `target/`, and npm cache in `.devcontainer/devcontainer.json`.
- [x] Reword the devcontainer proposal, design, and spec files so the scope stays devcontainer-only and the cache and helper-tool contract are precise.
