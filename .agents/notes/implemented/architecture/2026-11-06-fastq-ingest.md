# Agent Note: 共享完整 FASTQ 摄入路径

Status: implemented

## 问题

archive 与 pipeline 都需要“在 archive 内存预算内读完整输入”，但两边各自
分支：

- `engine::common::read_all_records` / `read_all_paired_records` /
  `read_all_interleaved_records` 服务 archive；
- `CompressionPipeline::run` / `run_paired` / `run_interleaved` 又各自打开
  stdin、单文件、paired、interleaved parser，并调用相同的
  `collect_*_within_archive_budget`。

两套代码在 stdin 判断、交错/双端 parser 选择和错误路径上重复。pipeline 位于
`engine` 之下，不能直接复用 `engine::common`，所以这个重复不会因为模块边界
自行消失。

## 决定

新增 `src/fastq/ingest.rs`，提供三个 `pub(crate)` 函数：

- `read_all_records(input_path, memory_limit_mb)`；
- `read_all_paired_records(input_path, input2_path, pe_layout, memory_limit_mb)`；
- `read_all_interleaved_records(input_path, pe_layout, memory_limit_mb)`。

它们只负责拓扑分支与 parser 选择，内存预算仍由 parser 的
`collect_*_within_archive_budget` 执行。`fastq` 是低层模块，archive、pipeline
和 engine 都可以依赖它；`engine::common` 删除这三个转发函数，archive_runner 与
CompressionPipeline 直接调用 `fastq::ingest`。

streaming 的长度预扫描保留在 `engine::common`：它需要在摄入前只读少量长度，
与“完整摄入”是不同语义，不应塞进 ingest 模块。

## 备选方案

- **把 ingest helper 留在 engine::common**：pipeline 无法依赖 engine，重复仍在；
- **在 pipeline 中复制并注释“与 archive 保持一致”**：正是本次要消除的漂移源；
- **引入泛型 `ReadSource` trait**：当前只有三种固定拓扑，trait 抽象税高于收益；
- **把长度预扫描也放进 ingest**：会把“完整摄入”与“流式前缀采样”混成一个接口，
  容易让调用方误用。

## 后果

- stdin / 单文件 / paired / interleaved 的完整摄入只有一份实现；
- archive 与 pipeline 共用同一 parser 选择和内存预算检查；
- 不改变 CLI 行为、错误文本或 `.fqc` 格式；
- `fastq` 模块新增一个 crate 内部子模块，不扩大稳定 API。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `npm run verify-notes`
- archive/pipeline/streaming 的 round-trip 由集成测试覆盖；真实语料为
  手工 release 冒烟，未纳入 CI。
