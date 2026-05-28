# developer-workflow change

## ADDED Requirements

### Requirement: the repository devcontainer interface must be explicit

The repository MUST define a single supported developer-environment contract for the devcontainer workflow and MUST treat that contract as part of repository policy.

The supported environments MUST be:

- local VS Code Dev Containers
- GitHub Codespaces

Other container runtimes MAY work, but they are not part of the supported interface.

#### Scenario: a contributor chooses a development environment

- **WHEN** a contributor opens `fqc` in a supported devcontainer environment
- **THEN** they get the same repository policy surface regardless of whether the container runs locally or in Codespaces
- **AND** unsupported container setups are not documented as first-class environments

### Requirement: the devcontainer toolchain must be pinned

The devcontainer interface MUST guarantee the repository toolchain used for local development at repository-pinned versions:

- Rust toolchain `1.75.0`
- `rustfmt`, `clippy`, `rust-analyzer`, and `rust-src`
- Node.js `24.x`
- `git`, `gh`, `jq`, `less`, `ripgrep`, `pkg-config`, `libbz2-dev`, and `liblzma-dev`
- the minimum repo-local helper tool surface required by the development workflow: `taplo`

Every guaranteed tool MUST be sourced from the repository-controlled devcontainer image, feature, or repository-local install step rather than from the host environment.

Because `.github/lsp.json` invokes the `taplo` binary directly, the repository MUST pin `taplo-cli` in the devcontainer image or another repository-controlled install definition; the host environment MUST NOT decide that version.

#### Scenario: a developer rebuilds the container

- **WHEN** the devcontainer image is rebuilt
- **THEN** the same pinned toolchain versions are available without depending on host-installed tools
- **AND** workflow helpers remain inside the repository-defined environment

### Requirement: the devcontainer bootstrap must use one shared seam

The devcontainer interface MUST route all bootstrap behavior through one repository-local seam.

That seam MUST be invoked by both create and start lifecycle entry points, and all dependency install, cache priming, and safe repair behavior MUST flow through that shared path.

#### Scenario: multiple entry points initialize the workspace

- **WHEN** the container runs create or start
- **THEN** both entry points use the same repository-local bootstrap logic
- **AND** no second bootstrap path introduces drift in setup behavior

### Requirement: the devcontainer lifecycle must be idempotent and fail fast

The devcontainer interface MUST define a two-phase lifecycle:

- `create` MAY install dependencies, configure repository-local state, and prime caches
- `start` MUST be idempotent and limited to safe environment repair

Bootstrap failures MUST be visible to the caller and MUST NOT be suppressed.

#### Scenario: the container starts more than once

- **WHEN** the container is started repeatedly
- **THEN** repeated starts do not rerun slow bootstrap work
- **AND** a failed bootstrap step is reported instead of being ignored

### Requirement: the devcontainer cache policy must be owned

The devcontainer interface MUST explicitly own the caches required for repeatable local work.

At minimum, the owned caches MUST include:

- Cargo registry state
- Cargo git state
- Rust build artifacts in `target/`
- npm dependency cache used by `npm ci`

Disposable install output such as `node_modules/` MUST be treated as rebuildable state rather than durable developer data.

The devcontainer interface MUST define the owned cache mounts as persistent named volumes in `.devcontainer/devcontainer.json`.
Cargo and npm caches SHOULD be mounted at locations owned by the devcontainer user so cache reuse does not depend on root-owned paths.

Those mounts MUST survive devcontainer rebuilds and container restarts for the same workspace. If the workspace or its named volumes are deleted, the caches MAY be recreated from the repository lockfiles and bootstrap steps.

#### Scenario: a contributor refreshes the workspace

- **WHEN** the container is rebuilt or restarted for the same workspace
- **THEN** the owned caches remain available through the named volume mounts
- **AND** disposable package-install output can be regenerated from lockfiles after volume loss
