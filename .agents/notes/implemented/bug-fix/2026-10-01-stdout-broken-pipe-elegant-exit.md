# Agent Note: stdout BrokenPipe 优雅退出，替代 println! 宏

Status: implemented

## 问题

`fqc info | head` 会把进程打崩：Rust 默认忽略 SIGPIPE，`println!` 写入已关闭的管道（下游 `head` 提前退出）返回 `BrokenPipe`，std 在 `println!` 内部 panic（退出码 101 + 强制打印 RUST_BACKTRACE 提示）。影响面是所有向 stdout 打印的命令（`info` / `verify` / `compress` / `decompress` 的 summary），在管道/`head`/`grep` 组合下是崩溃而非正常结束。项目坚持零 unsafe，不能走 `signal(SIGPIPE, SIG_IGN)`。

## 决定

- 在 `src/io/mod.rs` 新增 `println_stdout!` 与 `print_stdout!` 两个 `#[macro_export]` 宏：写 stdout 时遇到 `io::ErrorKind::BrokenPipe` 静默忽略（视为正常截断，退出码保持 0），其他 IO 错误保持原 `println!` 的 panic 行为。
- 四个命令文件（`commands/info.rs` / `verify.rs` / `compress.rs` / `decompress.rs`）的所有 stdout `println!` / `print!` 改用对应宏；`eprintln!`（stderr）不动——stderr 关闭时崩溃语义无变化。
- 语义取舍：`fqc info | head` 是"下游主动截断"的正常场景，应视为成功（exit 0）；只有磁盘满、stdout 是坏 fd 等非管道错误才保留报错/panic。

## 备选方案

- **`libc::signal(SIGPIPE, SIG_IGN)`** — 最强论据：一行代码、全局生效、进程级彻底解决；否决理由：需要 `unsafe`（`signal` 是 FFI），直接违反 `unsafe_code = "deny"` 与 README 的"零 unsafe"承诺。
- **全局 `catch_unwind` 捕获 println! panic** — 最强论据：不改任何调用点；否决理由：panic 已经丢弃部分缓冲输出、语义混杂，且把 panic 当控制流是本末倒置。
- **仅修 info 命令** — 最强论据：报告场景就是 `fqc info`；否决理由：verify/compress/decompress 的 summary 同样受影响，修一半留一半。

## 后果

- **收益**：`fqc info | head`、`verify | grep`、summary 管道组合全部优雅退出（exit 0），与 ripgrep 等 CLI 惯例一致；无 unsafe 引入。
- **代价**：两个宏取代标准 `println!`，调用点需经 `use crate::println_stdout;` 引入；宏对非 BrokenPipe 的写错误仍 panic，行为与旧 `println!` 相同——若未来想统一优雅处理所有 stdout 写错误（如 `/dev/full`），需另行设计错误传播，不在本次范围。

## 验证

`fqc info | head -2`、`fqc info --json | head -1`、`fqc verify | head -1` 管道截断后退出码均为 0 且无 panic 输出；`fqc info > /dev/full` 仍报错（exit 134，保持原 panic 语义）。四门禁 + MSRV 1.75 + 31 个测试二进制全绿。
