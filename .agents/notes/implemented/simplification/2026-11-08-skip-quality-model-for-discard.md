# Agent Note: discard 质量流不再构造 order-2 模型

Status: implemented

## Problem

`decompress_quality_stream` 对 `CodecFamily::Raw` 也调用
`ScmQualityCompressor::new`，只为随后走 `QualityMode::Discard` 分支返回
占位质量串。order-2 质量模型约 13 MiB，每个 discard block（含 verify）
都会白建一次。

## Decision

在 `decompress_quality_stream` 里提前处理 `CodecFamily::Raw`：

- 拒绝非空 payload（与 `QualityCompressor::decompress` 的 discard 语义一致）；
- 按 `uniform_read_length` 或逐条 `lengths` 直接生成 `!` 占位串；
- 非 Raw family 仍走原 compressor 路径。

## Verification

`cargo test --quiet < /dev/null`：34 个测试目标全部通过（含 decode/verify
的 discard 与 lossy 用例）。

## Alternatives considered

- **把 `QualityCompressor::new` 改为惰性分配模型**：需要给所有压缩/解压方法
  加 `Option` 分支，改动面大于一个提前返回。
- **在 `QualityContextModel::new` 里按 mode 返回空模型**：仍要实例化
  compressor，且模型结构会变成"可能为空"的隐式契约。

## Consequences

discard block 解码少一次约 13 MiB 分配；Raw 流的错误文本与原实现保持一致。
