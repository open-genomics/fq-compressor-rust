FROM mcr.microsoft.com/devcontainers/rust:1.75-bookworm

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        jq \
        less \
        libbz2-dev \
        liblzma-dev \
        pkg-config \
        ripgrep \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add \
        clippy \
        rust-analyzer \
        rust-src \
        rustfmt

RUN cargo install --locked \
        bacon \
        cargo-deny \
        taplo-cli \
    && rm -rf /usr/local/cargo/registry/cache
