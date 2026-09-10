# fq-compressor-rust 收尾实施计划

> **状态（2026-08-20 收口）：** Task 1-4、4bis、5-7 全部完成并提交（`55da121`、`ecf3a3c`）；
> Task 8 门禁 3× 全量零失败。交付物：`docs/hotspot-report.md`、`docs/real-corpus.md`、阶段计时 summary、
> pipeline 长读空归档修复、move-based 建块。真实测量值已写入对应报告。

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 收尾 fq-compressor-rust：结构性修复 e2e 并行 flaky、补齐分段计时并测热点、用真实语料补生产规模压缩/吞吐证据、同步文档与 CHANGELOG。

**Architecture:** 四个独立 workstream（WS1 测试隔离 / WS2 计时+热点 / WS3 真实语料 / WS4 文档），按 WS1 → WS2 → WS3 → WS4 顺序执行。WS2 的计时字段为 WS3 测量提供数据；WS3 的测量结果回填 WS4 文档。全部变更以现有 `cargo fmt/clippy/test/doc` 门禁验证，不破坏 `.fqc` 格式字节，零 unsafe，不新增 CI。

**Tech Stack:** Rust (MSRV 1.75.0)、rayon、crossbeam-channel、zstd、clap；bash + curl + gzip（语料拉取）；ENA 公开语料。

**Spec:** 无独立 spec 文件 —— 本收尾是用户直接授权（AskUserQuestion：范围="完整三件套"，性能深度="测量+顺手优化"；"全部开始"批准）。约束来源为 `AGENTS.md` 与 `docs/performance-roadmap.md`。

## Global Constraints

- **MSRV 1.75.0**：新代码不得使用 1.75 之后的 Rust 语法/API（如 `let ... else` 可用，`is_some_and` 需 1.70+ 可用；`u64::div_ceil` 1.73+ 可用）。
- **`#![deny(unsafe_code)]`**：所有新代码零 unsafe，仅用 std 原子与 channel。
- **门禁**（每次提交前必须全绿）：
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --lib --tests`（集成测试需项目根 cwd）
  - `cargo doc --no-deps`
- **格式字节兼容**：本计划只改性能/统计/测试/文档，不改任何压缩格式字节或 codec 序列；现有 round-trip / 格式契约测试必须全绿。
- **不新增 CI、不加依赖**：全部变更只用 std + 现有依赖（rayon、crossbeam-channel、zstd、flate2）。
- **破坏性变更纪律（AGENTS.md）**：lib API 若需签名变更（本计划避免），需在 CHANGELOG 注明；本计划不改 `GlobalAnalyzer::analyze` 签名。
- **语料不入库**：`corpus/` 加入 `.gitignore`；测量命令可复现，切片 sha256 记录在 `docs/real-corpus.md`。
- **集成测试运行前提**：测试用 `/tmp` 下的独立目录；本计划 WS1 强化为每个实例唯一目录。

---

## 文件结构（新建 / 修改）

| 文件 | 责任 | 变更 |
|---|---|---|
| `tests/test_e2e.rs` | 端到端测试 | WS1：TempFile 改为每实例唯一目录 |
| `src/engine/compression_engine.rs` | 压缩引擎（archive/streaming/pipeline 分派） | WS2：ProcessingStats 加 4 个阶段字段；run_archive 计时；run_pipeline 传递计时 |
| `src/pipeline/mod.rs` | PipelineStats 定义 | WS2：加 4 个阶段字段 |
| `src/pipeline/compression.rs` | 压缩 pipeline | WS2：run() 计时；run_paired/run_interleaved 置 0 |
| `src/pipeline/decompression.rs` | 解压 pipeline | WS2：reader/decomp/writer 三阶段计时 |
| `src/commands/compress.rs` | compress summary | WS2：summary 打印阶段计时 |
| `src/commands/decompress.rs` | decompress summary + 串行解压 | WS2：DecompressStats 加字段、run_parallel 计时、run_pipeline 传递、summary 打印 |
| `scripts/fetch_real_corpus.sh` | 拉取真实语料 | WS3：新建（镜像 C++） |
| `.gitignore` | 忽略语料/产物 | WS3：加 `corpus/` |
| `docs/real-corpus.md` | 真实语料测量报告 | WS3：新建 |
| `docs/hotspot-report.md` | 热点测量报告 | WS2：新建 |
| `docs/performance-roadmap.md` | 路线图状态 | WS4：active slice 标记完成 |
| `CHANGELOG.md` | 变更日志 | WS4：[Unreleased] 补条目 |
| `README.md` | 项目说明 | WS4：一致性检查 |

---

### Task 1: 修复 e2e 并行 flaky（WS1）

**背景（已实测）**：最初一次并行全量 `cargo test --lib --tests` 有 6 个 e2e 测试失败，均为 `/tmp/fqc_e2e_tests/` 下文件 not-found；standalone 重跑与后台 4 次全量复现全部通过。结论为共享临时目录的并发竞态（无法稳定复现但结构上不安全）。修复：每个 `TempFile` 实例使用**唯一目录**，`Drop` 只清理自己的文件与目录，从根本上消除跨测试共享目录。

**Files:**
- Modify: `tests/test_e2e.rs:36-53`（`TempFile` 定义）

**Interfaces:**
- 不变：`TempFile::new(name: &str) -> TempFile`、`TempFile::path(&self) -> &str`、`impl Drop`。所有测试调用点无需改动。
- 新增私有辅助：`static NEXT_TEMP_ID: AtomicU64`（仅测试模块内）。

- [ ] **Step 1: 改写 `TempFile` 为唯一目录**

把 `tests/test_e2e.rs` 顶部 helpers 区（第 36-53 行）替换为：

```rust
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

/// Unique-per-instance counter so parallel tests never share a temp dir.
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// RAII guard that removes its own file and unique directory on drop.
///
/// Each instance gets its own directory under `<tmp>/fqc_e2e_tests/<id>`,
/// so concurrent test runs can never observe or delete each other's files.
struct TempFile(String);

impl TempFile {
    fn new(name: &str) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, AtomicOrdering::Relaxed);
        let dir = std::env::temp_dir().join("fqc_e2e_tests").join(id.to_string());
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir.join(name).to_string_lossy().to_string())
    }
    fn path(&self) -> &str {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        if let Some(parent) = std::path::Path::new(&self.0).parent() {
            let _ = std::fs::remove_dir(parent); // best-effort; only removes own empty dir
        }
    }
}
```

> 注意：文件顶部已有 `use std::io::BufReader;` 等 import；若 `AtomicU64`/`Ordering` 未导入，把上面 `use` 行加进现有 import 块（不要重复）。`std::path::Path` 已在同文件用到（`test_data_dir` 用了 `std::path::Path::new`），无冲突。

- [ ] **Step 2: 运行 e2e 确认编译与行为不变**

Run: `cargo test --test test_e2e`
Expected: 全部通过（10/10，与改动前一致；TempFile 目录唯一化不改变任何测试断言）。

- [ ] **Step 3: 清理旧的共享目录残留**

Run: `python3 -c "import shutil,os; p=os.path.join('/tmp','fqc_e2e_tests'); shutil.rmtree(p, ignore_errors=True)"`
说明：旧版遗留的共享目录不再被使用，删除以免混淆后续复测。

- [ ] **Step 4: 连续 3 次全量套件验证 flaky 收敛**

Run: `for i in 1 2 3; do cargo test --lib --tests --quiet 2>&1 | grep -E "test result" ; done`
Expected: 3 行均为 `test result: ok`（0 failed）。若任何一行失败，记录失败测试并调查后再继续（不得带着失败进入 WS2）。

- [ ] **Step 5: 提交**

```bash
git add tests/test_e2e.rs
git commit -m "test(e2e): give every TempFile a unique temp dir to fix parallel flakiness"
```

---

### Task 2: 给三处统计结构加阶段计时字段（WS2 · 第一步）

**背景**：`PipelineStats`（pipeline 模式）目前只有总 `processing_time_ms`，`ProcessingStats`（engine）与 `DecompressStats`（命令层）没有分段计时。为测热点（parser/reorder/pipeline/archive-write），给三个结构统一加 4 个阶段字段，语义为：
- `parse_ms`：压缩=解析输入；解压=读取块
- `reorder_ms`：压缩=全局分析/重排（解压恒 0）
- `process_ms`：压缩/解压并行 worker 聚合时间（CPU 时间求和，非墙钟）
- `write_ms`：写出（序列化）

**Files:**
- Modify: `src/pipeline/mod.rs:43-52`（PipelineStats）
- Modify: `src/engine/compression_engine.rs:26-40`（ProcessingStats）及其 5 处构造点
- Modify: `src/commands/decompress.rs:59-68`（DecompressStats）

**Interfaces:**
- 三个结构各新增 4 个 `pub u64` 字段：`parse_ms, reorder_ms, process_ms, write_ms`。
- `PipelineStats` 与 `ProcessingStats` 均 derive `Default`，新字段默认 0，已有 `{ .. }` 字面量构造点必须显式补 4 个字段（否则编译错误，属预期——编译会提示逐个补齐）。
- 本任务只加字段 + 修编译，不填充真实计时（Task 3-5 填充）。

- [ ] **Step 1: `PipelineStats` 加字段**

在 `src/pipeline/mod.rs` 第 51 行 `reorder_map_written: bool,` 之后追加：

```rust
    // Stage timings (ms). Serial stages are wall-clock; process_ms
    // aggregates parallel worker time across threads.
    pub parse_ms: u64,
    pub reorder_ms: u64,
    pub process_ms: u64,
    pub write_ms: u64,
```

该字段新增会让 `src/pipeline/compression.rs` 的三处 `PipelineStats { ... }` 构造点（`run` 约 353 行、`run_paired` 约 481 行、`run_interleaved` 约 599 行）与 `src/pipeline/decompression.rs` 的 `run`（约 400 行）编译报错——Step 5 编译时逐个补 `parse_ms: 0, reorder_ms: 0, process_ms: 0, write_ms: 0`（Task 3/4 再填充真实值；`run_paired`/`run_interleaved` 保持 0）。

- [ ] **Step 2: `ProcessingStats` 加字段**

在 `src/engine/compression_engine.rs` 第 39 行 `elapsed_seconds: f64,` 之后追加：

```rust
    /// Stage timings (ms). Serial stages are wall-clock; process_ms
    /// aggregates parallel worker time across threads.
    pub parse_ms: u64,
    pub reorder_ms: u64,
    pub process_ms: u64,
    pub write_ms: u64,
```

- [ ] **Step 3: 补齐 `ProcessingStats` 5 处构造点**

`src/engine/compression_engine.rs` 中所有 `ProcessingStats { ... }` 字面量加 4 个字段，取值按模式：
- `run_archive`（约 343-350 行）：暂填 `parse_ms: 0, reorder_ms: 0, process_ms: 0, write_ms: 0`（Task 3 填充真实值）
- `run_pipeline`（约 549-556 行）：`parse_ms: stats.parse_ms, reorder_ms: stats.reorder_ms, process_ms: stats.process_ms, write_ms: stats.write_ms`
- `run_streaming_single` / `run_streaming_paired` / `run_streaming_interleaved`（约 744/857/974 行）：全填 0（流式单遍循环不细分段，见 Global Constraints 记录）

- [ ] **Step 4: `DecompressStats` 加字段**

在 `src/commands/decompress.rs` 第 66 行 `elapsed_seconds: f64,` 之后追加：

```rust
    /// Stage timings (ms). Serial stages are wall-clock; process_ms
    /// aggregates parallel worker time.
    pub parse_ms: u64,
    pub reorder_ms: u64,
    pub process_ms: u64,
    pub write_ms: u64,
```

- [ ] **Step 5: 编译 + 测试**

Run: `cargo build --all-targets && cargo test --lib --quiet`
Expected: 编译通过（若某处 `ProcessingStats { }` 漏补字段会编译报错，逐个补齐）；lib 测试全绿。本任务不改任何行为，无新增测试。

- [ ] **Step 6: 提交**

```bash
git add src/pipeline/mod.rs src/engine/compression_engine.rs src/commands/decompress.rs
git commit -m "feat(stats): add parse/reorder/process/write stage timings to all stats structs"
```

---

### Task 3: 填充压缩阶段计时（WS2 · 第二步）

**Files:**
- Modify: `src/engine/compression_engine.rs`（`run_archive`、`run_pipeline`）
- Modify: `src/pipeline/compression.rs`（`run()`）

**Interfaces:**
- 消费 Task 2 的 4 个字段。
- 产出：`run_archive` 返回的 `ProcessingStats` 带真实阶段计时；`CompressionPipeline::stats()` 返回的 `PipelineStats` 带真实阶段计时（run 路径）。

- [ ] **Step 1: `run_archive` 计时 —— parse（读入）**

`src/engine/compression_engine.rs` `run_archive` 开头（第 146 行 `let input = request.input.resolve();` 之后、`let records = ...` 之前）插入：

```rust
        let t_parse = std::time::Instant::now();
```

在 `let total_bases: u64 = ...`（第 169 行）之前插入：

```rust
        let parse_ms = t_parse.elapsed().as_millis() as u64;
```

- [ ] **Step 2: `run_archive` 计时 —— reorder（全局分析 + 建块）**

在 `// Phase 1: Global analysis (reordering)`（第 198 行）之前插入：

```rust
        let t_reorder = std::time::Instant::now();
```

在 `let block_read_sets: Vec<(u32, Vec<ReadRecord>)> = ...` 的 `.collect();`（第 282 行）之后插入：

```rust
        let reorder_ms = t_reorder.elapsed().as_millis() as u64;
```

- [ ] **Step 3: `run_archive` 计时 —— process（并行压缩）**

把第 292-298 行的 `par_iter` 压缩块替换为带计时聚合的版本（`process_ns` 用 `AtomicU64` 累加各 worker 实际压缩耗时）：

```rust
        let process_ns = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let process_ns_ref = std::sync::Arc::clone(&process_ns);
        let compressed_blocks: Vec<Result<CompressedBlockData>> = block_read_sets
            .par_iter()
            .map(|(block_id, reads)| {
                let t = std::time::Instant::now();
                let mut compressor = BlockCompressor::new((*block_config).clone());
                let r = compressor.compress(reads, *block_id);
                process_ns_ref.fetch_add(
                    t.elapsed().as_nanos() as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
                r
            })
            .collect();
        let process_ms = (process_ns.load(std::sync::atomic::Ordering::Relaxed) / 1_000_000) as u64;
```

- [ ] **Step 4: `run_archive` 计时 —— write（顺序写出 + reorder map + finalize）**

在 `// Sequential write (file I/O must be ordered)`（第 300 行）之前插入：

```rust
        let t_write = std::time::Instant::now();
```

在 `writer.finalize()?;` 与 `output_tx.commit()?;`（第 331-332 行）之后插入：

```rust
        let write_ms = t_write.elapsed().as_millis() as u64;
```

- [ ] **Step 5: `run_archive` 填统计**

把 `run_archive` 底部的 `ProcessingStats { ... }`（第 343-350 行）改为：

```rust
        let stats = ProcessingStats {
            total_reads,
            total_bases,
            input_bytes: total_bases,
            output_bytes,
            blocks_written: blocks_written as u64,
            elapsed_seconds: 0.0, // Will be filled by command layer
            parse_ms,
            reorder_ms,
            process_ms,
            write_ms,
        };
```

- [ ] **Step 6: `run_pipeline` 传递计时**

Task 2 Step 3 已把 `stats.parse_ms` 等拷进 `processing_stats`。核对 `src/engine/compression_engine.rs` `run_pipeline`（约 549-556 行）确实是 `parse_ms: stats.parse_ms, ...` 四行；若不是（编译错误会提示），改成上述形式。

- [ ] **Step 7: `src/pipeline/compression.rs::run()` 计时**

读 `src/pipeline/compression.rs` 确认 `run()` 的阶段边界（Phase 1 collect、Phase 1b analyze、Phase 2 reader/worker/writer），按与 run_archive 相同的四段插入计时：
- `parse_ms`：包住 Phase 1 `collect_all_within_archive_budget(...)`（含读取整个输入）——`let t_parse = Instant::now();` 放 Phase 1 前，读完后取 `elapsed`。
- `reorder_ms`：包住 Phase 1b `GlobalAnalyzer.analyze(&sequences)` + `ordered` 构造。
- `process_ms`：worker 闭包里 `compressor.compress(...)` 前后计时，累加进 `Arc<AtomicU64>`（用 `Ordering::Relaxed`），join 后取 `load / 1_000_000`。
- `write_ms`：writer 线程内 `write_block` 循环计时，随 writer handle 返回值带出。

writer handle 目前返回 `Result<u64>`（仅输出字节数，见 `src/pipeline/compression.rs:283`），把 `write_ms` 并入其返回值：改为 `Result<(u64, u64)>`（输出字节、write_ms），join 处（约 347 行）解构同步改二元组。`self.stats` 构造点（约 353 行）填入 `parse_ms`/`reorder_ms`/`process_ms`/`write_ms` 四个局部值，同时保留既有 `processing_time_ms = elapsed`。`run_paired` / `run_interleaved` 两处返回的 `PipelineStats` 保持 4 字段为 0（单线程顺序变体，Global Constraints 已记录，Task 2 已补 0）。

> 关键：4 个阶段计时都落入最终 `self.stats` 并流向 engine 的 `run_pipeline` → `ProcessingStats`。行号以实际读取为准，语义如上。

- [ ] **Step 8: 单元测试 —— 计时字段被填充**

新增 `tests/unit_stats_timing.rs`（或并入现有测试文件），至少一个测试：走 `CompressOptions::to_request()` 跑一次小档案压缩，断言 `ProcessingStats` 的 4 个计时字段合计 > 0。注意：`CompressionRequest` 没有 `Default`，必须经 `CompressOptions::to_request()`（默认 `mode=Archive`）构造，不要直接手写 `CompressionRequest { .. }`。示例：

```rust
use fqc::commands::compress::CompressOptions;
use fqc::engine::compression_engine::CompressionEngine;

#[test]
fn archive_stats_carry_stage_timings() {
    let mut seq = Vec::new();
    for i in 0..1000u32 {
        let id = format!("r{i}");
        let seq_body = "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT";
        writeln!(seq, "@{id}").unwrap();
        writeln!(seq, "{seq_body}").unwrap();
        writeln!(seq, "+").unwrap();
        writeln!(seq, "IIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIII").unwrap();
    }
    let dir = std::env::temp_dir();
    let input = dir.join("fqc_stats_timing_input.fastq");
    let out = dir.join("fqc_stats_timing_out.fqc");
    std::fs::write(&input, seq).unwrap();

    let opts = CompressOptions {
        input_path: input.to_string_lossy().to_string(),
        output_path: out.to_string_lossy().to_string(),
        show_progress: false,
        ..Default::default()
    };
    let outcome = CompressionEngine::new().run(opts.to_request()).unwrap();
    assert!(
        outcome.stats.parse_ms + outcome.stats.reorder_ms + outcome.stats.process_ms + outcome.stats.write_ms > 0,
        "archive compression should fill stage timings"
    );

    std::fs::remove_file(&input).ok();
    std::fs::remove_file(&out).ok();
}
```

> 顶部需 `use std::io::Write as _;` 才能用 `writeln!`。`CompressionEngine`/`CompressOptions` 均为 `pub`（test_e2e.rs 已使用同路径），可直接引用。若小输入下 `reorder_ms` 恰为 0（读太少、重排未触发），断言改为 `outcome.stats.processing_time_ms > 0 && outcome.stats.parse_ms + outcome.stats.process_ms + outcome.stats.write_ms > 0`。

- [ ] **Step 9: 门禁 + 提交**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --lib --tests --quiet && cargo doc --no-deps`
Expected: 全绿。
```bash
git add src/engine/compression_engine.rs src/pipeline/compression.rs tests/
git commit -m "feat(stats): instrument compression stage timings (parse/reorder/process/write)"
```

---

### Task 4: 填充解压阶段计时 + summary 展示（WS2 · 第三步）

**Files:**
- Modify: `src/pipeline/decompression.rs`（`run()`）
- Modify: `src/commands/decompress.rs`（`run_parallel`、`run_pipeline`、`print_summary`）
- Modify: `src/commands/compress.rs`（`print_summary`）

**Interfaces:**
- 消费 Task 2 的 `PipelineStats`/`DecompressStats` 4 字段。
- 产出：压缩与解压两条命令的 summary 打印阶段计时；`run_parallel` 填充 `DecompressStats` 计时。

- [ ] **Step 1: `DecompressionPipeline::run()` 计时**

`src/pipeline/decompression.rs`：
- reader 线程闭包（第 219 行 `move || -> Result<()>`）改为 `-> Result<u64>`：在 `for block_id in start_block..end_block` 前 `let t = Instant::now();`，循环结束 `Ok(t.elapsed().as_millis() as u64)`。
- decomp 线程（第 266 行闭包）：在 `for task in rx.iter()` 外建 `let thread_ns = ...` 局部累加，`compressor.decompress_block(&task.block_data)` 前后计时（`Instant`），循环结束把 `thread_ns` fetch_add 进一个 `Arc<AtomicU64>`（在创建线程前 `let decode_ns = Arc::new(AtomicU64::new(0));`，克隆进每个线程）。
- writer 线程（第 306 行 `-> Result<(u64, u64)>`）改为 `-> Result<(u64, u64, u64)>`：在 `let mut pending` 前 `let t = Instant::now();`，`Ok((total_reads_written, total_output_bytes, t.elapsed().as_millis() as u64))`。
- 底部 join（第 388-397 行）解构同步改为三元组；`self.stats = PipelineStats { ... }`（第 400-408 行）补：

```rust
            parse_ms: reader_read_ms,
            reorder_ms: 0,
            process_ms: decode_ms,
            write_ms: writer_write_ms,
```

其中 `reader_read_ms = reader_handle.join()...??` 的返回值、`decode_ms = (decode_ns.load(Ordering::Relaxed) / 1_000_000) as u64`、`writer_write_ms` 取自 writer join 第三元。

- [ ] **Step 2: `run_parallel` 计时（默认串行-分批解压）**

`src/commands/decompress.rs` `run_parallel`（第 381 行起）在 `while block_start < block_count` 之前声明三个累加器：

```rust
        let mut parse_ms = 0u64;
        let mut process_ms = 0u64;
        let mut write_ms = 0u64;
```

在循环体内三处加计时：
- Phase 1 读块：在 `// Phase 1: Read block data sequentially` 前 `let t = Instant::now();`，该段 `for block_id in block_start..batch_end { ... }` 结束后 `parse_ms += t.elapsed().as_millis() as u64;`。
- Phase 2 并行解压：包住 `let results: Vec<...> = block_data_vec.into_par_iter().map(...)` 段，同样 `let t = Instant::now();` → 段末 `process_ms += t.elapsed().as_millis() as u64;`（此为墙钟近似，par_iter 内不逐 worker 计时，可接受——报告里注明）。
- Phase 3 写出：包住 `for result in sorted { ... }` 段，`write_ms += ...`。

在 `run_parallel` 返回前写入 `self.stats`：

```rust
        self.stats.parse_ms = parse_ms;
        self.stats.process_ms = process_ms;
        self.stats.write_ms = write_ms;
```

（`reorder_ms` 恒 0，无需赋值。）

- [ ] **Step 3: `run_pipeline` 传递计时（命令层解压）**

`src/commands/decompress.rs` `run_pipeline`（约 658-681 行）在从 `DecompressionPipeline::stats()` 拷统计处补：

```rust
        self.stats.parse_ms = stats.parse_ms;
        self.stats.process_ms = stats.process_ms;
        self.stats.write_ms = stats.write_ms;
```

（`reorder_ms` 保持 0。）

- [ ] **Step 4: compress summary 打印阶段计时**

`src/commands/compress.rs` `print_summary`（第 229-240 行）在 `println!("  Throughput: ...")` 之后、`println!("===========================");` 之前加：

```rust
        println!(
            "  Stage timings:    parse {:.0} ms | reorder {:.0} ms | process {:.0} ms | write {:.0} ms",
            self.stats.inner.parse_ms as f64,
            self.stats.inner.reorder_ms as f64,
            self.stats.inner.process_ms as f64,
            self.stats.inner.write_ms as f64
        );
```

- [ ] **Step 5: decompress summary 打印阶段计时**

`src/commands/decompress.rs` `print_summary`（第 706-719 行）在 `println!("  Throughput: ...")` 之后、`println!("=============================");` 之前加：

```rust
        println!(
            "  Stage timings:    parse {:.0} ms | process {:.0} ms | write {:.0} ms",
            self.stats.parse_ms as f64,
            self.stats.process_ms as f64,
            self.stats.write_ms as f64
        );
```

（`reorder_ms` 解压场景恒 0，不打印。）

- [ ] **Step 6: 门禁 + 提交**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --lib --tests --quiet && cargo doc --no-deps`
Expected: 全绿；`cargo test --test test_e2e` 仍全过（summary 输出不在断言范围，但格式不回归）。
```bash
git add src/pipeline/decompression.rs src/commands/decompress.rs src/commands/compress.rs
git commit -m "feat(stats): instrument decompression stage timings and print in summaries"
```

---

### Task 4bis: 修复 pipeline 长读 reorder-skip 空归档 bug（WS2 途中发现，已完成）

**背景（已实测）**：测量 ONT pipeline 时发现 `--pipeline` + 长读产出 **165 字节空归档**（0 blocks，exit 0）。根因：`GlobalAnalyzer::analyze` 对 non-Short 类跳过 reorder → `reverse_map` 为空 → pipeline 的 Phase 1b 只凭空 `reverse_map` 构建 `ordered` → 0 chunks。engine 的 `run_archive` 有正确的 `reordering_performed` 回退，pipeline 三处（run / run_paired / run_interleaved）没有。

**修复（commit 55da121，已提交）**：
1. 三处 Phase 1b 提升 `reordering_performed`；仅在 true 时用 `reverse_map` 构建 `ordered`，否则 `ordered = all_reads`；未 reorder 时 `forward_map`/`reverse_map` 置 `None`。
2. `build_flags` 第二参（IS_ORIGINAL_ORDER）从 `!self.config.enable_reorder` 改为 `!reordering_performed`（与 engine 一致，避免孤儿 reorder map 位）。
3. 回归测试 `test_e2e_pipeline_long_reads_no_reorder_nonempty_archive`（40 条 12kb 长读 → Long 类 → 非空归档 + round-trip）。
4. 顺手修复：单 block 解压 Normal mode 与 `run_original_order` 现在也报阶段计时（原先 0/0/0），不再误导热点报告。

**重测（release, --threads 8）**：ONT pipeline compress 9.56s（parse 151 / reorder 74 / process 8808 / write 9138 ms），输出 66,014,018 字节（与 streaming 66,013,771 几乎一致，符合长读不重排的预期）；decompress 9.67s（parse 58 / process 9408 / write 144 ms），round-trip `cmp` IDENTICAL。

---

### Task 5: 拉取真实语料 + 写热点报告（WS2/WS3 交汇）

**Files:**
- Create: `scripts/fetch_real_corpus.sh`
- Modify: `.gitignore`
- Create: `docs/hotspot-report.md`
- Create: `docs/real-corpus.md`

**Interfaces:**
- 产出：`corpus/SRR2962693_1.head200k.fastq`（Illumina WXS R1，200k reads，约 54 MiB）与 `corpus/DRR171398_1.head4k.fastq`（人类 MinION，4k reads，约 125 MiB）——与 C++ 侧相同切片，可直接横向对比。
- 产出：两份测量文档（真实数字在 Step 4 填入）。

- [ ] **Step 1: 写 `scripts/fetch_real_corpus.sh`（镜像 C++）**

```bash
#!/usr/bin/env bash
# 从 ENA 拉两份公开 FASTQ 的前缀切片，供 docs/real-corpus.md 复测。
# 切片不入库（.gitignore: corpus/）。head 截断 gzip 流时 curl 会报 23，属预期。
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${FQC_CORPUS_DIR:-$ROOT/corpus}"
readonly ILLUMINA_RECORDS=200000
readonly ONT_RECORDS=4000
mkdir -p "$OUT"

fetch_head() {
    local url="$1"
    local dest="$2"
    local records="$3"
    local lines=$((records * 4))
    if [[ -s "$dest" ]]; then
        echo "exists: $dest"
        return 0
    fi
    echo "fetch: $url -> $dest ($records records)"
    curl -L --fail --retry 3 --retry-delay 2 "$url" | gzip -dc | head -n "$lines" >"$dest"
    local status=${PIPESTATUS[0]}
    if [[ "$status" -ne 0 && "$status" -ne 23 ]]; then
        echo "download failed: curl exit $status" >&2
        return 1
    fi
}

# Illumina WXS R1，约 54 MiB。
fetch_head \
    "https://ftp.sra.ebi.ac.uk/vol1/fastq/SRR296/003/SRR2962693/SRR2962693_1.fastq.gz" \
    "$OUT/SRR2962693_1.head200k.fastq" \
    "$ILLUMINA_RECORDS" || exit 1

# 人类 MinION，约 125 MiB。ENA 头无 runid=，靠长读 + DRR accession 识别。
fetch_head \
    "https://ftp.sra.ebi.ac.uk/vol1/fastq/DRR171/DRR171398/DRR171398_1.fastq.gz" \
    "$OUT/DRR171398_1.head4k.fastq" \
    "$ONT_RECORDS" || exit 1

echo
wc -l -c "$OUT"/*.fastq
sha256sum "$OUT"/*.fastq
```

- [ ] **Step 2: `.gitignore` 加 `corpus/`**

`.gitignore` 追加一行：

```
corpus/
```

- [ ] **Step 3: 运行脚本拉取语料**

Run: `bash scripts/fetch_real_corpus.sh`
Expected: 两个文件存在，`wc` 输出 200000/800000 行（illumina）与 4000/16000 行（ONT），`sha256sum` 与 C++ 侧记录一致（若 C++ 侧已拉过同切片）。把 sha256 记到剪贴板/临时文件，Step 4 填入文档。网络不可用时（curl 非 23 失败），记录失败原因、跳过 Step 4 的实测、在报告里注明"未测量（网络不可用）"，不得伪造数字。

- [ ] **Step 4: 测量并写两份报告**

方法学（两个切片 × 三种模式 × lossless，固定 `-t 8`，同窗口 A/B，各跑 2 次取中位数）：
1. **压缩**：`target/release/fqc compress -i <slice> -o <out>.fqc`（archive 默认）、加 `--pipeline`（pipeline 模式）、加 `--streaming`（streaming 模式）。从 summary 读 ratio、MB/s、Stage timings。
2. **校验**：`target/release/fqc verify -i <out>.fqc --quick`，记录耗时。
3. **解压**：`target/release/fqc decompress -i <out>.fqc -o <out>_out.fastq`，从 summary 读 Stage timings、MB/s；`cmp <slice> <out>_out.fastq` 确认无损（ONT 切片若含超长读触发 long-read 分支则记录之）。
4. **重复**：每个 (slice, mode) 组合跑 2 次，取较快者为报告数字。
5. 构建 release：`cargo build --release`（集成测试需项目根 cwd；若 glibc `__tunable_is_initialized@GLIBC_PRIVATE` 报错，用 `PATH="/usr/bin:/bin:/usr/local/bin:$HOME/.cargo/bin"`）。

`docs/hotspot-report.md` 写入：
- 目的、环境（CPU 核数、`cargo build --release` 版本、日期）。
- 表格：`illumina-archive / illumina-pipeline / illumina-streaming / ont-archive / ont-pipeline / ont-streaming` 行的 `parse_ms / reorder_ms / process_ms / write_ms / 总时长 / MB/s / ratio`（解压行同样记录）。
- 结论：按每行占比，指出各模式下瓶颈（预期：illumina 短读 reorder 占比高、ONT 长读 process 占比高、write 在慢盘占高；以实测为准）。
- 若某模式 `process_ms` 占 `processing_time_ms` 的显著部分但已并行，注明"受限于块数/负载均衡"。

`docs/real-corpus.md` 写入：
- 语料来源（ENA accession、切片方法、sha256、行数/字节）、为什么选这两个（真实 Illumina WXS 短读 + 真实人类 MinION 长读）。
- 复现命令（Step 4 的 1-3 条原文）。
- 结果表：两种切片 × 三种模式，ratio + 压缩 MB/s + 解压 MB/s + verify 耗时。
- 与 C++ 横向对比列（若 C++ 侧 docs/performance 有数字则引用其数值；C++ 侧数字在本仓库之外，报告中标注"见 fq-compressor 项目报告"，不要臆造）。
- 无失真结论：`cmp` 全等。

- [ ] **Step 5: 门禁 + 提交**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --lib --tests --quiet && cargo doc --no-deps`（脚本/文档改动不影响编译，但跑一遍保险）
Expected: 全绿；`git status` 确认 `corpus/` 未跟踪（被 ignore）。
```bash
git add scripts/fetch_real_corpus.sh .gitignore docs/hotspot-report.md docs/real-corpus.md
git commit -m "docs(perf): real-corpus measurements and hotspot stage-timing report"
```

---

### Task 6: 按热点报告做机会性优化（WS2 · 第四步，条件门）

**Files:**
- Modify: `src/engine/compression_engine.rs`（`run_archive` 建块路径）
- Modify: `src/pipeline/compression.rs`（`run()` 重排重建路径）

**Interfaces:**
- 消费 `docs/hotspot-report.md` 的结论。
- 产出：若报告显示 reorder/建块是热点，则把重排后的块构建从"克隆"改为"移动"，压缩比不变（同样的读、同样的顺序），内存与分配减少。

**决策门**：读 `docs/hotspot-report.md`。若任一模式的 `reorder_ms` ≥ 该模式 `processing_time_ms` 的 20%，执行 Step 1-3；否则在报告中追加一行"reorder 非瓶颈，跳过移动优化（YAGNI）"并只做 Step 4（验证）直接提交。

**背景（代码事实）**：`run_archive` 建块时每个 read 被克隆一次（无重排时 `records[i].clone()`，有重排时 `records.get(orig_id).cloned()`），且之前还克隆了全部 `sequences: Vec<String>` 供分析。pipeline 的 `run()` 同样在 `ordered`（逐条 `clone`）+ `chunks(block_size).to_vec()` 中克隆全部读。

- [ ] **Step 1: `run_archive` 建块改移动**

`src/engine/compression_engine.rs` `run_archive` 中 `let records = ...` 之后（第一个使用前）把 `records` 绑定为可变并消费式取用。将第 254-282 行的建块段替换为：

```rust
        let mut records = records;
        let block_read_sets: Vec<(u32, Vec<ReadRecord>)> = analysis
            .block_boundaries
            .iter()
            .filter_map(|boundary| {
                let start = boundary.archive_id_start as usize;
                let end = boundary.archive_id_end as usize;

                let block_reads: Vec<ReadRecord> = if analysis.reordering_performed && !analysis.reverse_map.is_empty()
                {
                    (start..end)
                        .filter_map(|archive_id| {
                            analysis
                                .reverse_map
                                .get(archive_id)
                                .and_then(|&orig_id| records.get_mut(orig_id as usize).map(std::mem::take))
                        })
                        .collect()
                } else {
                    (start..end.min(records.len())).map(|i| std::mem::take(&mut records[i])).collect()
                };

                if block_reads.is_empty() {
                    None
                } else {
                    Some((boundary.block_id, block_reads))
                }
            })
            .collect();
```

前提（已核对）：`block_read_sets` 之后 `records` 不再被使用（`total_bases`/`total_reads`/`global_header` 都在之前），移动安全。`analysis.reordering_performed` 对所有 boundary 恒定，两条分支互斥，不会把同一 `orig_id` 取两次。

- [ ] **Step 2: pipeline `run()` 重建改移动**

`src/pipeline/compression.rs` `run()`：把 `let ordered: Vec<ReadRecord> = result.reverse_map.iter().map(|&orig_idx| all_reads[orig_idx as usize].clone()).collect();` 改为移动版（`all_reads` 先 `let mut all_reads = all_reads;`，`all_reads.get_mut(orig_idx as usize).map(std::mem::take)`），再把 `chunks(block_size).to_vec()` 改为 `std::mem::take` 切分（`while !ordered_reads.is_empty() { let n = ...min(block_size); chunks.push(ordered_reads.drain(..n).collect()); }`）。保持块顺序与内容一致。

- [ ] **Step 3: 优化前后对比 + 单测**

- 跑 Task 5 的 illumina-archive 压缩命令，记录优化后 `reorder_ms`/总时长/MB/s，对比 `docs/hotspot-report.md` 的优化前数字，把 before/after 追加到报告（新小节"Opportunistic optimization: move-based block build"）。
- 单测：`cargo test --lib --tests` 全绿（e2e round-trip 已覆盖顺序与内容一致性；若现有 e2e 覆盖不足，补一个"压缩产物字节相同"断言——压缩后 `verify --quick` 通过即可，不需要字节级断言）。

- [ ] **Step 4: 门禁 + 提交**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --lib --tests --quiet && cargo doc --no-deps`
Expected: 全绿（含全部 e2e）。
```bash
git add src/engine/compression_engine.rs src/pipeline/compression.rs docs/hotspot-report.md
git commit -m "perf(reorder): move reads into blocks instead of cloning (archive + pipeline)"
```

---

### Task 7: 文档 / CHANGELOG 收口（WS4）

**Files:**
- Modify: `docs/performance-roadmap.md`
- Modify: `CHANGELOG.md`
- Modify: `README.md`

**Interfaces:**
- 消费 Task 5 的测量结论与 Task 6 的 before/after。

- [ ] **Step 1: 更新 `docs/performance-roadmap.md`**

- 找到标记为"热点测量 :active"的 gantt slice，改状态为完成，并追加 1-2 行结果摘要（指向 `docs/hotspot-report.md` 与 `docs/real-corpus.md` 的结论：各模式瓶颈与优化前/后数字）。
- 若 roadmap 中有"下一刀"待办，标记为已完成或明确推迟理由，不留 dangling 待办。

- [ ] **Step 2: 更新 `CHANGELOG.md` [Unreleased]**

在 `[Unreleased]` 下按现有条目风格追加（保留既有条目不动）：

```markdown
### Fixed
- 修复 e2e 测试在并行全量运行时的偶发失败（每个 TempFile 使用唯一临时目录）。

### Added
- 压缩/解压统计新增分段计时（parse/reorder/process/write），summary 与 hotspot 报告可查看各阶段耗时。
- `scripts/fetch_real_corpus.sh` 拉取真实 ENA 语料；`docs/real-corpus.md` 记录生产规模压缩比/吞吐证据。
- `docs/hotspot-report.md`：解析/重排/压缩/写出四阶段热点测量。

### Performance
- 重排建块路径改为移动而非克隆，减少归档压缩峰值内存与分配（依热点报告实测）。
```

- [ ] **Step 3: README 一致性检查**

读 `README.md`：确认未引用已过时的数字/行为（如压缩比声明、性能表）。若 README 有性能数字且与 `docs/real-corpus.md` 冲突，更新或加指向报告的链接。只做最小修正，不扩写。

- [ ] **Step 4: 门禁 + 提交**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --lib --tests --quiet && cargo doc --no-deps`
Expected: 全绿。
```bash
git add docs/performance-roadmap.md CHANGELOG.md README.md
git commit -m "docs: close out performance roadmap slice, CHANGELOG, and README consistency"
```

---

### Task 8: 收尾验证门禁 + 提交收口

**Files:**
- 无代码改动（只验证 + 收口提交）。

**Interfaces:**
- 消费全部前序任务。

- [ ] **Step 1: 全量门禁 ×2**

Run（两遍，确认无 flaky 残留）:
```bash
for i in 1 2; do cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --lib --tests --quiet 2>&1 | grep -E "test result|error" ; done
cargo doc --no-deps
```
Expected: 每遍 `test result: ok` 全绿，无 clippy/fmt 报错，`cargo doc` 无 warning。

- [ ] **Step 2: 确认零 unsafe 与无格式破坏**

Run: `grep -rn "unsafe" src/ | grep -v "//" | grep -v "^.*unsafe_code" || echo "no unsafe in src"`
Expected: `no unsafe in src`。
说明：格式字节未改（WS1-WS4 只动测试/统计/文档/建块方式，无 codec/格式改动）；e2e 全绿即格式契约回归验证。

- [ ] **Step 3: 提交收口**

```bash
git status --short
git log --oneline -12
git commit -am "chore: wrap-up validation pass" 2>/dev/null || git commit -m "chore: wrap-up validation pass"
```
若 Step 3 无未提交改动则跳过提交（`git status` 干净即可）。

- [ ] **Step 4: 汇总报告**

向主对话交付中文收尾报告，包含：四 workstream 完成情况、每项验证结果、`docs/real-corpus.md` 与 `docs/hotspot-report.md` 的关键数字摘要（ratio/MB/s/各阶段占比）、优化前后对比、提交 hash 列表、以及"剩余事项"（若网络不可用等导致某项未完成，明确列出）。
