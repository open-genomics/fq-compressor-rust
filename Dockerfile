# ============================================================================
# fqc — Multi-stage Docker Build
# ============================================================================
# Build:  docker build -t fqc .
# Run:    docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc
# ============================================================================

# ---- Stage 1: Builder -------------------------------------------------------
FROM rust:1.75-bookworm AS builder

WORKDIR /build

# Cache dependencies: copy manifests first, build a dummy to cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo 'fn main() { println!("dummy"); }' > src/main.rs && \
    echo '' > src/lib.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy full source and build
COPY . .
RUN cargo build --release --locked && \
    strip target/release/fqc

# ---- Stage 2: Runtime --------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libbz2-1.0 \
        liblzma5 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/fqc /usr/local/bin/fqc

# Create non-root user for security
RUN groupadd -r fqc && useradd -r -g fqc -d /data fqc
USER fqc

# Default working directory for data
WORKDIR /data

# Health check
HEALTHCHECK --interval=60s --timeout=3s CMD ["fqc", "--version"]

ENTRYPOINT ["fqc"]
CMD ["--help"]

# Labels (OCI standard)
LABEL org.opencontainers.image.title="fqc"
LABEL org.opencontainers.image.description="High-performance FASTQ compressor"
LABEL org.opencontainers.image.source="https://github.com/lessup/fq-compressor-rust"
LABEL org.opencontainers.image.licenses="GPL-3.0"
