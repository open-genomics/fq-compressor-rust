# Agent Note: 普通文件输出走同目录临时文件 + rename 事务

Status: implemented

## 问题

ordinary、streaming、pipeline 三种 writer 曾直接在最终路径上 `File::create` / `FqcWriter::create`：`--force` 会在压缩完成前就截断既有归档，中途失败时最终路径留下一个看似完整的半成品。`decompress` 的 split-PE 也急切创建两个目标文件，风险相同。不做处理意味着一次失败压缩可能毁掉用户手里唯一完好的旧归档。

## 决定

普通文件输出统一经过 `src/io/output_transaction.rs` 这一个事务抽象：先写同目录临时文件，`flush`/`close` 成功后才 `rename` 到目标路径；中途失败时既有目标原样保留（`--force` 下旧文件也不丢）。所有压缩引擎与 pipeline 的 writer 创建点、`decompress` 的普通文件与 split-PE 输出、解压 pipeline 输出都走该抽象。`stdout` 不是文件，保持直通、不参与事务。行为与 C++ sequential 工具的 atomic replace 策略对齐。

出处：openspec 归档变更 `make-file-output-atomic`（`FQCR-IO-001`，2026-08-18 归档）。

## 备选方案

- **最终路径直写 + 失败后清理** — 少一个抽象层；否决理由：清理窗口内的崩溃仍留半成品，只有先建临时文件才是 fail closed。
- **stdout 一并事务化** — 行为更一致；否决理由：管道无法 rename，明确列为 out of scope。
- **跨文件原子多文件提交** — split-PE 两文件同时生效最干净；否决理由：跨文件系统的分布式提交超出范围，两文件各自事务已覆盖单文件半成品风险；平台 rename 限制已写入文档。

## 后果

- **收益**：`--force` 压缩中途失败不再毁掉既有归档；半成品永不以最终文件名出现；解压 split-PE 同样安全。
- **代价与已知上限**：split-PE 两输出独立提交，不是整体原子——一个成功一个失败时仍可能出现单文件残留；临时文件要求目标目录可写且与目标同文件系统。若未来需要多文件整体原子，须重访本决定。

## 验证

`src/io/output_transaction.rs` 是唯一抽象；集成测试 `tests/test_output_atomic.rs` 覆盖拒绝、force 成功、force 失败保留旧文件三种路径。
