#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="${WORKSPACE:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

setup_git() {
    if ! git config --global --get-all safe.directory | grep -Fx -- "$WORKSPACE" >/dev/null; then
        git config --global --add safe.directory "$WORKSPACE"
    fi
}

setup_hooks() {
    local hook_script="$WORKSPACE/scripts/setup-hooks.sh"

    if [ ! -x "$hook_script" ]; then
        echo "Missing or non-executable hook setup script: $hook_script" >&2
        exit 1
    fi

    "$hook_script"
}

install_js_deps() {
    if [ -f "$WORKSPACE/package.json" ]; then
        (
            cd "$WORKSPACE"
            npm ci
        )
    fi
}

prefetch_rust() {
    if [ -f "$WORKSPACE/Cargo.toml" ]; then
        (
            cd "$WORKSPACE"
            cargo fetch
        )
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
        setup_hooks
        ;;
    *)
        echo "Usage: $0 {create|start}" >&2
        exit 1
        ;;
esac
