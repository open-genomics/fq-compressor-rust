#!/usr/bin/env bash
# =============================================================================
# fqc (fq-compressor-rust) DevContainer - 容器内设置脚本
# =============================================================================
# 在容器内运行，配置 Rust 开发环境
# 用于 postCreateCommand / postStartCommand
# =============================================================================
set -euo pipefail

# 颜色输出
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $*" >&2; }

WORKSPACE="${WORKSPACE:-$(pwd)}"

# =============================================================================
# 设置函数
# =============================================================================

# 配置 Git safe directory
setup_git() {
    if ! command -v git >/dev/null 2>&1; then
        return
    fi

    if ! git config --global --get-all safe.directory 2>/dev/null | grep -Fxq "$WORKSPACE"; then
        git config --global --add safe.directory "$WORKSPACE"
    fi
}

# 首次构建 + 测试
initial_build() {
    cd "$WORKSPACE" || return

    log_info "Running cargo build..."
    cargo build 2>&1

    log_info "Running cargo test..."
    cargo test --lib --tests 2>&1

    log_info "Build & test completed."
}

# 验证工具链
check_toolchain() {
    log_info "Rust toolchain:"
    rustc --version
    cargo --version
    clippy-driver --version 2>/dev/null || true
}

# =============================================================================
# 主入口
# =============================================================================

usage() {
    cat <<EOF
用法: $0 <command>

命令:
  create    postCreateCommand - 首次创建容器时运行
  start     postStartCommand  - 每次启动容器时运行
EOF
}

cmd_create() {
    log_info "执行 postCreateCommand..."

    setup_git
    check_toolchain
    initial_build

    log_info "postCreateCommand 完成"
}

cmd_start() {
    log_info "执行 postStartCommand..."

    setup_git

    log_info "postStartCommand 完成"
}

main() {
    local cmd="${1:-}"

    case "$cmd" in
        create)  cmd_create ;;
        start)   cmd_start ;;
        -h|--help|help) usage ;;
        "")
            usage
            exit 1
            ;;
        *)
            log_warn "未知命令: $cmd"
            usage
            exit 1
            ;;
    esac
}

main "$@"
