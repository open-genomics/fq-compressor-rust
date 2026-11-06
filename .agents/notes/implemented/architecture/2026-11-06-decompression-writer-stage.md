# Agent Note: 解压 writer 阶段独立成模块

Status: implemented

## 问题

统一解压调度后，`pipeline/decompression.rs` 同时承担两件事：

- Reader/decompressor 线程调度、块范围选择、内存与线程预算；
- writer 线程内的块排序、范围过滤、original-order 重排、skip-corrupted
  占位符、split R1/R2 路由、统计计数和事务提交。

这使一个文件超过 600 行，两个阶段的输入结构（`WriterThreadInput`、
`DecompressedResult`）与 writer 实现混在调度文件里。writer 还把结果打包成
五元组返回，调用方只能按位置理解 `reads/output_bytes/total_bases/corrupted/ms`。
另外，original-order 重排遇到越界 archive id 时只 `warn` 后 `continue`，
可能提交一份静默缺读段的输出。

## 决定

把 writer 阶段整体移到 `pipeline/decompression_writer.rs`：

- `BlockResult`、`WriterPlan`、`WriterStats` 成为 writer 模块的显式契约；
- `spawn_writer_thread` 返回 `WriterStats` 结构体，不再返回五元组；
- `decompression.rs` 只保留 Reader→Decoder 调度、`prepare` 和块范围选择；
- 所有输出目的地创建收敛到 `OutputWriters::open`（single/stdout/split）；
- `WriteProgress` 统一维护范围过滤、archive 读段坐标和输出计数；
- original-order 在重排前校验
  `forward.len() == decoded_reads.len() == total_archive_reads`；
  映射越界改为 `FqcError::Format`，不再静默跳过并提交截断输出；
- 输出事务语义不变：只有 writer 完整成功后才 `commit`，任何错误都会
  drop 临时文件。

## 备选方案

- **继续留在 `decompression.rs`，只拆私有函数**：调度与输出组装仍共享同一
  组结构体和导入，文件边界不表达职责边界，否决。
- **为 archive-order / original-order 建 trait 双实现**：两者共享范围过滤、
  split 路由、占位符和事务提交，动态分派会把这些共享逻辑再次拆散；当前用
  一个 `WriteProgress` + 最后的重排步骤更直接。
- **保留 `warn + continue`**：reorder map 已经在 `load_reorder_map` 验证为
  置换，越界代表内部不变量被破坏；静默提交缺读段输出比报错更危险，否决。
- **把 split 路径派生也移出 writer 模块**：路径派生只服务 split 输出，跟随
  `OutputWriters` 更内聚，保留。

## 后果

- `decompression.rs` 约 430 行，只表达线程调度；writer 模块约 500 行，
  集中表达“解码结果如何变成有序输出”；
- writer 的输入、输出和统计字段有名字，调用方不再依赖五元组位置；
- 非法 original-order 映射会失败并回滚输出，而不是成功退出但少读段；
- `.fqc` wire format、CLI 参数、统计字段和正常输出字节均不变。

## 验证

- `cargo fmt --all -- --check`、`cargo clippy --all-targets --locked -- -D warnings`、
  `cargo test --lib --tests --locked`（33 组）、`cargo doc --no-deps --locked`、
  `cargo +1.75.0 check --all-targets --locked`、`npm run verify-notes` 通过；
- 真实语料旧二进制 vs 新 writer：
  - illumina / nanopore / mito SE 默认解压逐字节一致；
  - mito SE 的 archive / pipeline / streaming 三种归档默认解压逐字节一致；
  - split-PE、split-PE + range 的 R1/R2 逐字节一致；
  - original-order 与输入逐字节一致，original-order + range 与旧二进制一致；
  - range、header-only 逐字节一致；
- `test_original_order_skip_corrupted_emits_placeholders` 通过 forward map
  逐原始坐标校验占位符位置。
