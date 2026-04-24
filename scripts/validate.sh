#!/usr/bin/env bash
set -euo pipefail

mode="${1:-full}"

run_docs() {
    if [ -f package.json ]; then
        npm run docs:build
    fi
}

case "$mode" in
    fast)
        cargo fmt --all -- --check
        cargo clippy --all-targets -- -D warnings
        ;;
    full)
        cargo fmt --all -- --check
        cargo clippy --all-targets -- -D warnings
        cargo test --lib --tests
        cargo doc --no-deps
        run_docs
        if command -v cargo-deny >/dev/null 2>&1; then
            cargo deny check bans licenses sources
        fi
        ;;
    *)
        echo "Usage: $0 {fast|full}" >&2
        exit 1
        ;;
esac
