# fqc Development Container
FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

# Install system dependencies for compression libraries
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libbz2-dev \
        liblzma-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install commonly used Rust development tools
RUN cargo install --locked \
        bacon \
        cargo-deny \
        cargo-release \
        git-cliff \
        taplo-cli \
    && rm -rf /usr/local/cargo/registry/cache

# Set default toolchain components
RUN rustup component add \
        rustfmt \
        clippy \
        rust-src
