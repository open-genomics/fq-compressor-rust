# =============================================================================
# fqc (fq-compressor-rust) Development Container
# =============================================================================
# Base: Microsoft Rust devcontainer (Debian Bookworm)
# MSRV: 1.75
# =============================================================================
FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

# ---- Layer 1: System dependencies (rarely changes) ----
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libbz2-dev \
        liblzma-dev \
        pkg-config \
        # Useful dev utilities
        jq \
        less \
        ripgrep \
    && rm -rf /var/lib/apt/lists/*

# ---- Layer 2: Rust toolchain components (changes with rustup updates) ----
RUN rustup component add \
        rustfmt \
        clippy \
        rust-src

# ---- Layer 3: Cargo tools (most expensive, changes rarely) ----
# Separate into two layers: frequently used vs. release-only tools
RUN cargo install --locked \
        bacon \
        cargo-deny \
        taplo-cli \
    && rm -rf /usr/local/cargo/registry/cache

RUN cargo install --locked \
        cargo-release \
        git-cliff \
    && rm -rf /usr/local/cargo/registry/cache
