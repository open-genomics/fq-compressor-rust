# Agent Note: Path-typed API 与非 UTF-8 路径

Status: implemented

## 问题

稳定 facade 已接受 `impl AsRef<Path>`，但内部仍把路径转成 `String`：

- `api::info` / `api::decompress` 调用 `to_string_lossy()`；
- `FqcReader::open*`、`ArchiveInfo.file_path`、解压 pipeline 和 `VerifyOptions.input_path`
  使用 `String`；
- 非 UTF-8 路径在打开归档、事务输出和 split-PE 路径推导前就可能被替换成 `�`。

这会让“通用嵌入 API”在 Unix 文件名边界上失效，也会把显示层需求泄漏到 I/O 层。

## 决定

- `FqcReader::open` / `open_with_budget` 接受 `impl AsRef<Path>`；reader 保存 `PathBuf`，
  `ArchiveInfo.file_path` 改为 `PathBuf`。
- `DecompressionPipeline::run`、`SequentialDecompressor::run` 接受 `impl AsRef<Path>`，
  writer 线程和 `OutputTransaction` 使用 `PathBuf`；split-PE 文件名用 `OsString` 拼接，
  不再解析成字符串。
- `VerifyOptions.input_path` 改为 `PathBuf`，verify 直接使用 `Path` API。
- CLI 的显示层仍可用 `Path::display()`；只有 JSON/日志需要人类可读文本时才做展示转换。
- 新增 Unix 非 UTF-8 文件名回归测试，覆盖 `api::compress`、`api::info`、pipeline
  `api::decompress` 和输出字节统计。

## 备选方案

- **只在 facade 边界做 lossy 转换**：改动最小，但非 UTF-8 路径会在实际打开前损坏；
  否决。
- **保留 `String` 并额外存一份 `PathBuf`**：可以兼容旧字段，但会产生两个真源，且
  调用方仍可能拿到错误的显示字符串；项目允许 breaking change，直接改成 `PathBuf`。
- **把整个 CLI 参数改为 `PathBuf`**：长期更彻底，但 clap 参数解析与 help 文本需要同步
  大改；本轮先把库/reader/解压边界做对，CLI 字符串在构造 request 时转换。

## 后果

- 嵌入方可安全处理包含任意 OS 字节的文件名；`ArchiveInfo.file_path` 不再有 lossy
  字符串。
- `VerifyOptions` 和 `ArchiveInfo` 的 public 字段类型发生 breaking change；这符合项目
  “允许 breaking、内部 API 不承诺兼容”的边界，但需要在下一次发布说明中保留记录。
- split-PE 的 `_R1` / `_R2` 推导现在保留原 `OsString` 扩展名，不再依赖 UTF-8 字符串
  的最后一个 `.`。
- 显示层仍可能对不可打印字节使用 `display()` 的替换表现；这是展示限制，不影响读写。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `npm run verify-notes`
- `tests/test_api.rs::facade_preserves_non_utf8_paths`：输入、归档、输出都使用含
  `0xFF` 的文件名，compress → info → pipeline decompress → 字节 `cmp` 全部通过。
