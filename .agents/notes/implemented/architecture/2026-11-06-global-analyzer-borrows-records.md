# Agent Note: GlobalAnalyzer 直接借用 ReadRecord

Status: implemented

## 问题

`GlobalAnalyzer::analyze` 接收 `&[String]`，调用方必须先构造一份只含序列的
owned 副本：

- archive 每次压缩都执行 `records.iter().map(|r| r.sequence.clone())`，即使
  `--reorder false` 或输入是长读、分析器根本不会重排；
- pipeline 在 `reorder` 可能启用时也构造同样的副本，直到 auto 判定才决定是否
  重排。

对短读数据，这相当于在完整 `Vec<ReadRecord>` 之外再持有一整套 sequence
字符串和分配器元数据。100k × 150bp 的 release 冒烟中，archive 峰值 RSS 为
139 MB；同一输入走 pipeline 为 112 MB，差额主要来自这份不必要的副本。

## 决定

把 `GlobalAnalyzer::analyze` 以及相邻共享、全局共享、重排规划函数的参数改为
`&[ReadRecord]`，直接读取 `read.sequence`。archive 传 `&records`，pipeline 传
`&all_reads`，不再构造 `Vec<String>`。测试也改为构造 `ReadRecord` 并直接传入。

分析器只读取 `sequence`，不依赖 ID/quality/comment；这不会改变重排判定或块边界，
只是把输入视图从“序列字符串切片”改成“领域记录切片”。

## 备选方案

- **保留 `&[String]`，只在 `reorder == Never` 时跳过克隆**：能覆盖显式关闭重排的
  情况，但默认 `auto` 仍要先克隆才能判定；且调用方继续承担重复视图的义务。
- **保留 `&[String]` 并新增 `analyze_records`**：会留下两套入口和两套测试路径，
  没有兼容性收益；项目内部 API 允许 breaking change。
- **引入泛型 `SequenceView` trait**：可以让 `String` 与 `ReadRecord` 共用实现，但
  当前只有这两个短期用例，抽象税高于收益；直接使用领域类型更清晰。

## 后果

- archive 不再在全局分析阶段复制全部 sequence；pipeline 也少一次潜在复制。
- `GlobalAnalyzer` 的内部 API 发生 breaking change；它本来就是 `#[doc(hidden)]`
  的内部模块，测试已同步迁移。
- 重排 map、块边界、codec 输入和 `.fqc` 字节均不改变。
- 长度统计仍会在分析器内部构造一个 `Vec<usize>`；这是 O(reads) 的小对象，
  与整库 sequence 副本不是一个量级，暂不为它增加额外抽象。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- release 冒烟（100k × 150bp 随机读段，`--block-reads 1000 --reorder false`，
  `-t 4`）：archive 峰值 RSS 139,188 KB -> 121,244 KB，0.26 s；pipeline
  111,824 KB / 0.24 s；两者均产出 100 blocks、4,725,031 字节，解压结果与输入
  逐字节一致。
