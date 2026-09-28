# Agent Note: 压缩侧提供 archive / streaming / pipeline 三种执行模式

Status: implemented

## 问题

使用场景横跨三档：本地小文件（内存充足、可全量分析、要最佳压缩率）、大型测序数据（内存受限、须流式）、管道集成与多核吞吐（阶段分离、并行）。单一执行路径无法同时优化内存占用与吞吐量。

## 决定

`CompressionEngine` 以 `ExecutionMode` 路由三种模式，输出均为同一 `.fqc` 格式、CLI 语义一致：

- **Archive（默认）**：全量摄入 + 全局分析 + 可选 minimizer 重排，压缩率最优，内存随输入规模增长。
- **Streaming（`--streaming`）**：单遍增量处理、禁用重排，内存有界，适合大文件与受限环境。
- **Pipeline（`--pipeline`）**：读取/压缩/写入分阶段并行，背压缓冲，吞吐优先；重排能力受限（短读单端）。

三模式共享 FASTQ 解析器、压缩算法、归档写入器与类型定义，各自持有独立执行路径可独立演进。

出处：原 `docs/architecture/decisions/002-three-execution-modes.md`（ADR-002，已采纳），2026-09-27 折叠进本树。使用侧选择指南见 `docs/how-to/modes.md`。

## 备选方案

- **单一模式 + 参数调节** — 最强论据：最简单直观、无需用户选择；否决理由：无法同时优化内存与性能，大文件场景直接受限。
- **双模式（archive + streaming）** — 最强论据：覆盖主要场景且实现适中；否决理由：管道集成与多核并行场景支持不足，吞吐优化空间受限。

## 后果

- **收益**：每类场景有明确推荐路径；各模式可独立优化演进；格式一致保证下游解压无需感知模式。
- **代价与已知上限**：三条执行路径都要维护与测试（每模式独立测试 + 跨模式集成测试）；pipeline 是分段执行路径而非严格低内存摄入，该边界必须在文档中反复澄清。若某模式长期无差异化价值（如 pipeline 吞吐优势被 archive 追平），应删减而非保留。

## 验证

模式枚举与路由在 `src/engine/compression_engine.rs`（`ExecutionMode::{Archive,Streaming,Pipeline}`）；请求归一化在 `src/engine/compression_request.rs`。
