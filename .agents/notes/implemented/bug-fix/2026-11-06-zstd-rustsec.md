# Agent Note: 升级 zstd 修复 RustSec 告警

Status: implemented

## 问题

`cargo-deny` 在默认分支上报出四条和 zstd 相关的 RustSec 告警：

- `RUSTSEC-2026-0391`：复用 context 时，被放弃的 stream 可能把残留输入带入下一个
  frame；
- `RUSTSEC-2026-0398` / `RUSTSEC-2026-0403`：`zstd-safe` 的 seekable 与 legacy
  stream 解码路径可能越界设置 `Vec` 长度或暴露未初始化内存；
- `RUSTSEC-2026-0410`：`zstd-sys` 的 wasm32 allocator shim 在分配失败时可能破坏
  线性内存。

当前依赖链是 `zstd 0.13.3 -> zstd-safe 7.2.4 -> zstd-sys 2.0.16`。即使本项目主要
使用 bulk/stream API，也不能把“当前调用没有命中漏洞路径”当作依赖审计通过。

## 决定

将直接依赖固定到 `zstd = "=0.14.3"`，并用锁文件解析出：

- `zstd 0.14.3`
- `zstd-safe 8.2.0`
- `zstd-sys 2.1.2+zstd.1.5.7`

这三者都满足 RustSec 给出的修复范围，且声明的 MSRV 不高于仓库的 1.75。
不添加 `cargo-deny` ignore，也不关闭 advisory gate。

## 备选方案

- **只升级到 `zstd 0.13.4`**：同样能修复 `zstd` 自身告警，但 `zstd-safe` 仍可能停留
  在旧范围；`0.14.3` 的锁文件已经解析到 `zstd-safe 8.2.0` 和 `zstd-sys 2.1.2`，
  一次覆盖全部四条告警，故选新版本。
- **在 `deny.toml` 中忽略告警**：会永久掩盖真实漏洞，且和仓库“安全门禁 fail closed”
  的定位冲突；否决。
- **只升级 transitive dependency**：`zstd` 的直接版本约束仍固定旧版本，后续
  `cargo update` 可能回退到不安全组合；直接升级并锁定直接依赖。

## 后果

- `.fqc` 编解码 API 未变；现有 frozen fixture、round-trip 和损坏输入测试全部通过。
- 锁文件中的 zstd 家族版本整体前移，构建缓存会失效一次。
- `cargo-deny` 重新成为有效门禁；后续若再出现 advisory，需要继续升级或明确记录
  不可达证明，而不是直接 ignore。

## 验证

- `cargo tree -i zstd`：`0.14.3`
- `cargo tree -i zstd-safe`：`8.2.0`
- `cargo tree -i zstd-sys`：`2.1.2+zstd.1.5.7`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `cargo build --release --locked`
- `npm run verify-notes`

本地未安装 `cargo-deny`，因此本机没有伪造 `cargo deny check` 结果；由 GitHub
Actions 的 `cargo-deny` job 做最终 advisory 验收。
