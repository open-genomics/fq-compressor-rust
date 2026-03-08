# fqc DevContainer

本目录包含 DevContainer 配置，提供一致的 Rust 开发环境。

## 快速开始

1. 安装 [Docker Desktop](https://www.docker.com/products/docker-desktop/) 和 VS Code [Dev Containers 扩展](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
2. 在 VS Code 中打开项目
3. 按 `F1` → `Dev Containers: Reopen in Container`
4. 等待容器构建完成（首次约 5-10 分钟）

容器启动后会自动执行 `cargo build` 和 `cargo test`。

## 支持的开发环境

| 环境 | 推荐度 | 说明 |
|------|--------|------|
| **WSL2** | ⭐⭐⭐ 推荐 | Windows 上的最佳路径，原生文件系统性能 |
| **远程 Linux** | ⭐⭐⭐ 推荐 | 通过 VS Code Remote-SSH |
| **Windows 原生** | ⚠️ 不推荐 | volume 性能较差 |

## 文件说明

```
.devcontainer/
├── devcontainer.json        # 主配置（VS Code / Windsurf / Cursor）
├── Dockerfile               # 开发环境镜像（Rust + 系统依赖 + 工具）
├── scripts/
│   └── container-setup.sh   # 容器内设置脚本（postXxxCommand）
└── README.md                # 本文件
```

## 容器内预装工具

| 类别 | 工具 |
|------|------|
| **编译器** | Rust stable (MSRV 1.75) |
| **组件** | rustfmt, clippy, rust-src |
| **Cargo 工具** | bacon, cargo-deny, cargo-release, git-cliff, taplo-cli |
| **系统依赖** | libbz2-dev, liblzma-dev, pkg-config |
| **实用工具** | jq, ripgrep, git, GitHub CLI |

## 持久化缓存

容器使用 Docker named volumes 持久化以下目录，重建容器后无需重新下载/编译：

- **`fqc-cargo-cache`** → `/usr/local/cargo/registry`（crate 下载缓存）
- **`fqc-target-cache`** → `target/`（编译产物缓存）

清理缓存：

```bash
docker volume rm fqc-cargo-cache fqc-target-cache
```

## 常用命令

```bash
# 构建
cargo build

# 测试
cargo test --lib --tests

# Clippy 检查
cargo clippy --all-targets

# 格式检查
cargo fmt --all -- --check

# TOML 格式检查
taplo check

# 持续编译（bacon）
bacon
```

## SSH 密钥

如需在容器内访问私有仓库，取消 `devcontainer.json` 中 SSH mount 的注释：

```jsonc
// "type=bind,source=${localEnv:HOME}/.ssh,target=/home/vscode/.ssh,readonly"
```

## 故障排除

### 容器构建失败

```bash
# 清理并重建
docker volume rm fqc-cargo-cache fqc-target-cache
# 然后 F1 → "Dev Containers: Rebuild Container"
```

### cargo build 报 permission denied

target 目录由 named volume 管理，权限可能不一致：

```bash
sudo chown -R vscode:vscode target/
```

### WSL2 性能差

确保项目在 **WSL 原生文件系统** 内（`/home/...`），**不要**放在 `/mnt/c/...`。
