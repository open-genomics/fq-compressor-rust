# Agent Note: stdin + -2 拒绝，解压错误保留根因

Status: implemented

## Problem

1. `compress -i - -2 R2.fastq`：`to_request` 在 `input_path == "-"` 时优先
   选择 Stdin 拓扑并忽略 `input2_path`，命令成功退出但归档缺少 mate。
2. 解压流水线在 `reader_handle.join()?` 处提前返回：若 decoder/writer 先失败，
   reader 会因 channel 关闭报 `channel closed`，用户看到的是次生错误而不是
   真正根因。

## Decision

- `CompressCommand::validate_options` 在 stdin + `input2_path` 时返回
  `FqcError::InvalidArgument`，CLI 映射为用法错误（退出码 2）。
- 解压等待阶段先 join reader、decoder、writer 全部线程并收集错误，再选择
  第一条不含 `channel closed` 的错误返回；只有全部都是次生错误时才回退。

## Verification

- `cargo test --test test_stdin_stdout < /dev/null`：7 项通过，新增
  `stdin_with_second_input_is_rejected`。
- `cargo test --quiet < /dev/null`：34 个测试目标全部通过。
- `cargo clippy --all-targets -- -D warnings`、`cargo fmt --all -- --check` 通过。

## Alternatives considered

- **支持 stdin 与文件配对**：需要同时读两个来源并保持流式同步，语义与现有
  单 reader 模型冲突；直接拒绝更安全。
- **只 join 所有线程后返回第一条错误**：仍可能返回 `channel closed`；按
  关键字过滤保留了"谁是根因"的信息。

## Consequences

- 该参数组合从"成功但输出错误"变为明确的用法错误。
- 解压失败时的错误信息指向 decoder/writer 的真实失败原因。
