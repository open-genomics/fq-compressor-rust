# Design

## Decision 1: support one explicit developer-environment contract

The repository should treat the devcontainer as a defined interface, not just a convenience image. The supported environments for that contract are:

- local VS Code Dev Containers
- GitHub Codespaces

Other container runtimes may work incidentally, but they are not part of the supported contract.

## Decision 2: pin the repo-local toolchain surface

The devcontainer contract should guarantee the tools used for day-to-day work at repository-pinned versions:

- Rust toolchain `1.75.0`
- `rustfmt`, `clippy`, `rust-analyzer`, and `rust-src`
- Node.js `24.x`
- `git`, `gh`, `jq`, `less`, `ripgrep`, `pkg-config`, `libbz2-dev`, and `liblzma-dev`
- the minimum repo-local helper tool surface the workflow invokes directly: `taplo`

Every guaranteed tool should come from the repository-controlled devcontainer image, feature, or repository-local install step rather than from the host.

`taplo` should be provided by a repository-pinned `taplo-cli` install because `.github/lsp.json` invokes the `taplo` binary directly.

This keeps the container aligned with the repository's current workflow policy instead of depending on host drift.

## Decision 3: make lifecycle behavior explicit

The container interface should define a two-phase bootstrap:

- `create` may install dependencies, prime caches, and configure repository-local state
- `start` must be idempotent and limited to safe environment repair

Bootstrap failures must surface instead of being silently suppressed.

## Decision 4: own cache policy inside the contract

The devcontainer contract should explicitly own the caches needed for repeatable local work:

- Cargo registry and git caches
- Rust build artifacts in `target/`
- npm dependency cache used by `npm ci`

Disposable state such as `node_modules/` remains reinstallable and must not be treated as durable developer data.

The contract should also own the mounts that back those caches, with Cargo and npm caches mounted at user-owned locations so durable state survives rebuilds as part of repository policy rather than as an accident of the host.

## Decision 5: keep the implementation seam small

The future implementation should converge on one repo-local bootstrap seam instead of spreading setup policy across multiple devcontainer entry points. Both `postCreateCommand` and `postStartCommand` should call the same bootstrap script, and that script should own setup, cache priming, and safe repair behavior.
