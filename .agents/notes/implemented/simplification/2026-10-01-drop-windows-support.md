# Agent Note: 移除 Windows 内存探测 FFI，聚焦 Linux/macOS

Status: implemented

## 问题

全仓库唯一一处 `unsafe` 是 Windows `GlobalMemoryStatusEx` FFI（原
`src/memory_budget.rs` 的 `get_available_memory_windows`），而 CI 是
Linux-only——这段唯一的 unsafe 从未被任何门禁编译或执行过，README 的
"除 Windows 内存探测外零 unsafe" 安全声明恰好落在无人验证的代码上。若为它
补 Windows CI job，则与 `AGENTS.md` 的 "lightweight, low-maintenance" 定位
冲突（Windows runner 最重，且三平台矩阵删除后新增的大量测试从未在 Windows
验证过，首跑即红的风险真实存在）。维护者确认不做 Windows 支持。

## 决定

- 删除 `get_available_memory_windows` 与 `GlobalMemoryStatusEx` FFI；
  `get_available_memory_mb` 的 fallback cfg 收窄为
  `not(any(target_os = "linux", target_os = "macos"))`——Windows 及其他平台
  落入 8192 MB 默认预算，可由 `--memory-limit` 显式覆盖。
- CI 不设 Windows job；`deny.toml` 的 `targets` 移除
  `x86_64-pc-windows-msvc`。
- README / SECURITY.md 的 unsafe 声明更新为字面零 unsafe；`unsafe_code = "deny"`
  从此没有任何豁免对象。macOS 路径保留（`sysctl` 子进程，safe 实现）。

## 备选方案

- **Windows 仅编译检查 job（`cargo check`）** — 最强论据：几行配置就能防止
  FFI 代码腐烂，成本远低于完整测试；否决理由：为一个明确不支持的平台保留
  专属代码与 CI 资源本末倒置，删码比护码更符合轻量定位。
- **保留 FFI 并恢复完整 Windows 测试** — 最强论据：覆盖最全、历史配置现成；
  否决理由：重量与维护成本最高，且"历史能过"不成立——矩阵删除后测试集
  大幅扩充，从未被 Windows 验证。

## 后果

- **收益**：unsafe 字面归零，README/SECURITY 的安全声明与代码、CI 验证范围
  完全一致；CI 维持 Linux 单平台轻量（MSRV + cargo-deny 仍是廉价 Linux job）。
- **代价**：若确有 Windows 用户，其失去真实可用内存探测，自动预算固定为
  8192 MB——对大内存机器上的超大文件可能提前触发 `ResourceLimit`，需显式
  `--memory-limit`；此退化是静默的，仅在文档层面说明。

## 验证

`cargo fmt/clippy/test/doc` 全绿；`grep -rn "unsafe" src/` 除 lints 配置引用外
零命中；`cargo +1.75.0 check --all-targets --locked` 通过（MSRV 维持 1.75）。
