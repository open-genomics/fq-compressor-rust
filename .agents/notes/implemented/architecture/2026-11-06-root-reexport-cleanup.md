# Agent Note: 收窄 crate 根部再导出

Status: implemented

## 问题

`lib.rs` 曾同时提供两套入口：

- `fqc::api` 的四个稳定操作；
- crate 根部的 `fqc::compress` / `decompress` / `info` / `verify` 别名；
- crate 根部再导出 `SequenceCompressor`、`QualityCompressor`、`IdCompressor`、
  `AuxCompressor` 四个内部 trait。

文档明确说稳定嵌入面是 `fqc::api`，但根部 trait 再导出让 `fqc::SequenceCompressor`
看起来像稳定 API；集成测试也因此绕过模块路径直接使用根级别名。入口越多，后续
重构越容易把内部实现误当成兼容承诺。

## 决定

删除 crate 根部的四个 compressor trait 再导出；需要 trait 的测试改用
`fqc::algo::compressor_traits::SequenceCompressor`。保留 `fqc::api` 的四个便捷别名，
因为它们直接指向稳定 facade，不会产生第二套实现。

在 `tests/test_architecture.rs` 增加源码守卫：禁止 `lib.rs` 再导出 `algo`、`archive`、
`block`、`pipeline`、`plan`、`stats`、`types` 等内部模块的类型。这样新增根级内部
再导出会在测试阶段失败。

## 备选方案

- **保留 trait 再导出并加 `#[doc(hidden)]`**：仍然是 public API，只是隐藏文档；
  Rust 用户仍可直接依赖，不能真正收窄。
- **同时删除 `fqc::compress` 等根部别名**：能进一步减少入口，但它们是 facade 的
  直接别名，不引入内部依赖；本轮不做额外 breaking change。
- **把 traits 移到新的公共扩展模块**：当前没有稳定的自定义 codec 需求，先不为
  尚未存在的扩展点设计公共 API。

## 后果

- `fqc::SequenceCompressor` 等根级路径不再可用；这属于 internal API 收窄，项目允许
  breaking change。
- `fqc::api` 与 `fqc::algo::compressor_traits` 的模块路径成为唯一明确入口。
- 根级 `fqc::compress` / `decompress` / `info` / `verify` 仍作为 facade 别名保留；
  它们不改变内部依赖方向。
- 新增根级内部再导出会被 `test_architecture` 拦截。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `npm run verify-notes`
