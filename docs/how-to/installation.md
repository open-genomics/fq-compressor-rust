# 安装

## 从源码构建

依赖要求：

- Rust **1.75.0+**（MSRV）
- Git

```bash
git clone https://github.com/open-genomics/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release
./target/release/fqc --help
```

## 本地安装

```bash
cargo install --path .
```

> **同名二进制 `PATH` 覆盖风险**：本实现与 C++ `fq-compressor` 都安装名为 `fqc` 的二进制。若两者同时在 `PATH`，后者（或更靠前的目录）覆盖前者——安装后用 `which fqc` 确认实际调用的实现。

## ARM64 / aarch64 Linux 服务器

`fqc` 没有 x86 专用代码路径；Linux aarch64 使用 Rust 标准 target 的 NEON/AdvSIMD
基线即可构建，不需要额外的架构 feature 或 C 交叉编译器。

本仓库在以下环境完成过 release 构建、完整测试和 smoke round-trip（2026-10-27）：

- CPU：HiSilicon Kunpeng-920，aarch64，192 个物理 CPU
- 容器可见 CPU affinity：21 个 CPU；`nproc` 返回 21
- 内存 cgroup：`memory.max = 200 GiB`
- 工具链：`rustc 1.75.0`、`cargo 1.75.0`

```bash
uname -m                         # 应为 aarch64
nproc                            # 当前进程可见 CPU 数
cargo build --release --locked
cargo test --lib --tests --locked

./target/release/fqc compress \
  -i tests/data/test_se.fastq -o /tmp/fqc-arm-smoke.fqc --force
./target/release/fqc verify -i /tmp/fqc-arm-smoke.fqc
```

### 部署注意

- `--threads 0` 按当前进程可见的 CPU 自动选择；本机可见 21 个 CPU。若容器
  CPU quota 与 affinity 不一致，显式传 `--threads N`，不要只依赖宿主机核数。
- `--memory-limit 0` 在 Linux 上会取宿主机 `MemAvailable` 与 cgroup 剩余额度的
  较小值；超大文件仍建议显式设置 `--memory-limit`。
- 已按基因组位置排序的 Illumina 短读通常应使用 `--reorder false`；重排适合
  打乱顺序的输入。最新热点证据见
  [hotspot 报告](../reference/hotspot-report.md)。
- 不建议把 `RUSTFLAGS="-C target-cpu=native"` 设为发布默认值。本机 Kunpeng 920
  上的 A/B 未显示稳定收益，而且 native 二进制不应直接搬到 CPU 不同的机器。
  跨机器分发时在目标 ARM 机器重新构建，或明确记录 CPU/glibc 兼容边界。
