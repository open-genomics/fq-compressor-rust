# Agent Note: 格式家族由归档 magic 区分，fqc/.fqc 名称共享不改

Status: implemented

## 问题

本实现与 C++ `fq-compressor` 共享产品名 `fqc` 和扩展名 `.fqc`，但格式家族不同：本仓库是 `fqc-indexed/v2`，C++ 是 `fqc-sequential/v2`。用户 PATH 上拿错二进制时，Rust reader 把 sequential magic 当普通坏 magic 处理，报错不指向正确工具；反之亦然。不做处理意味着这个混淆会永远以"无法解析"的形态出现，没有修复指引。

## 决定

两个实现都保留 `fqc` / `.fqc`，不引入区分后缀或改名（openspec `project.md` 记录的决策 `FQC-DEC-001`）；格式家族由归档头 8 字节 magic 区分。reader 在读取版本字节之前先用 `classify_magic` / `magic_dispatch_error` 分类：sequential magic 映射为 `UnsupportedFormat`，报错文案锁定点名 `fqc-sequential/v2` 与 `open-genomics/fq-compressor`；未知或截断 magic 仍报 `Format`。本实现不解码 sequential 归档，只承诺识别并指路。`info`/`verify`/`decompress` 三个入口行为一致；C++ 的 `frozen_se.fqc` 拷贝为 fixture 锁定该行为（`tests/fixtures/foreign-sequential-v2/`）。

出处：openspec 归档变更 `recognize-sequential-fqc-family`（`FQC-FAMILY-001`，2026-08-18 归档）。

## 备选方案

- **加独立后缀或产品改名** — 最彻底：拿错二进制在文件名层面就不可能；否决理由：`FQC-DEC-001` 明确选择共享名称——改名与生态分裂的代价大于一个清晰的报错，且已存在的 `.fqc` 文件不随名走。
- **解码 sequential 归档实现互通** — 用户体验最好；否决理由：明确列为 out of scope，跨家族解码是把两套格式演进耦死，识别并指路已覆盖实际痛点。
- **按通用 `Format` 错误处理 sequential magic** — 改动最小；否决理由：这正是问题本身，报错必须点名对方家族与正确仓库才有指引价值。

## 后果

- **收益**：拿错工具的用户得到指向 `open-genomics/fq-compressor` 的明确错误；magic 分类先于版本/头解析，坏输入的面最小。
- **代价与已知上限**：识别不等于兼容，两家族永久互拒——若未来要互通须另立新决定而非放宽此检查；magic 前 8 字节成为跨仓库的稳定契约，任何改 magic 的格式演进必须同时更新两侧分类逻辑。

## 验证

`src/archive/` 的 magic 分类函数与 `UnsupportedFormat` 错误路径；fixture 测试在 `tests/fixtures/foreign-sequential-v2/`。
