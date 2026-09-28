# Agent Note: 块内按组件分离编码（id/seq/qual/aux 四流）

Status: implemented

## 问题

FASTQ 记录的四个组件（ID、序列、质量、辅助）统计特性差异极大：ID 常有模式可循、序列有跨读段相似性、质量值可分箱。整记录混合编码把高熵与低熵数据揉在一起，无法对各组件选最优编码；需要决定组件在归档内的组织粒度。

## 决定

块内按组件分离编码：每个块携带 `id_stream`/`seq_stream`/`qual_stream`/`aux_stream` 四条流，块头记录各流偏移、大小与 codec id。编码按组件特性分派：ID 走 tokenize（模式检测失败回退 exact）或 discard；序列短读走 ABC、中长读走 Zstd；质量走 SCM/Zstd，`illumina8`/`qvz`/`discard` 为显式有损档；aux 存长度等辅助数据。组件分离使部分解压与组件级优化成为可能。

出处：原 `docs/architecture/decisions/003-component-encoding.md`（ADR-003，已采纳），2026-09-27 折叠进本树。流与 codec 定义见 `docs/reference/format-spec.md`、`CONTEXT.md`。

## 备选方案

- **整记录编码** — 最强论据：实现最简单、记录边界天然清晰；否决理由：无法按组件特性优化，序列相似性与质量可分箱性全部浪费。
- **字段级列式存储（全文件维度分离）** — 最强论据：同类数据全集聚集、压缩效率上限最高、支持部分解压；否决理由：破坏块级局部性，需重建记录，与块索引随机访问的设计冲突。

## 后果

- **收益**：每条流可用最优 codec；组件级优化（如 QVZ 码本、ID tokenize）能独立落地；可只解压所需组件。
- **代价与已知上限**：块头固定开销（约 104 字节，大块时可忽略）；块内须维护四流对齐，解码侧按偏移/大小还原记录。新增组件类型走 aux 流扩展而非新增流槽位。

## 验证

流布局在 `src/archive/format.rs` 的块头定义；编码分派在 `src/algo/block_compressor.rs` 与各 compressor 实现。
