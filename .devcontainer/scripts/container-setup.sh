#!/usr/bin/env bash
set -euo pipefail

WORKSPACE="${WORKSPACE:-$(pwd)}"

setup_git() {
    git config --global --add safe.directory "$WORKSPACE" 2>/dev/null || true
}

setup_hooks() {
    if [ -x "$WORKSPACE/scripts/setup-hooks.sh" ]; then
        "$WORKSPACE/scripts/setup-hooks.sh" >/dev/null 2>&1 || true
    fi
}

install_js_deps() {
    if [ -f "$WORKSPACE/package.json" ] && command -v npm >/dev/null 2>&1; then
        cd "$WORKSPACE"
        npm ci
    fi
}

prefetch_rust() {
    if command -v cargo >/dev/null 2>&1; then
        cd "$WORKSPACE"
        cargo fetch
    fi
}

case "${1:-}" in
    create)
        setup_git
        setup_hooks
        prefetch_rust
        install_js_deps
        ;;
    start)
        setup_git
        ;;
    *)
        echo "Usage: $0 {create|start}" >&2
        exit 1
        ;;
esac
