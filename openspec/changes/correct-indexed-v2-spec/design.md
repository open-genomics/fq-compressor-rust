# Design: correct-indexed-v2-spec

## Evidence

- `src/archive/format.rs` 定义 indexed magic、version 编码、codec/checksum 常量和 header/footer serialization；
- `src/archive/reader.rs` 在 magic 后读取版本，并拒绝非兼容 major；
- `docs/reference/format-spec.md` 是公开字节规范，但编号表与实现漂移；
- 当前 tests 主要证明 current writer/current reader round-trip，没有提交稳定 archive 作为 decoder 契约。

## Approach

1. 从 serialization/deserialization 代码提取精确字段表，规范只描述可验证 bytes；
2. 用审计基线生成最小 deterministic FASTQ 的 `.fqc` archive，manifest 记录 source commit、命令、输入/输出 SHA-256 和稳定结构；
3. decoder fixture 测试冻结读取能力；writer 测试只断言规范承诺的稳定字段，除非规范明确 canonical archive；
4. 对 codec family/version、checksum ID、format major 建立表驱动拒绝测试；
5. 修正文档，不顺手改变 reader 行为。

## Allowed surface

- `openspec/`
- `AGENTS.md` 中仅与 lightweight spec policy 冲突的一条规则
- `docs/reference/format-spec.md` 及直接链接/摘要
- `tests/` 与必要的 test support
- `tests/fixtures/` 或仓库现有等价位置
- `CHANGELOG.md`

仓库历史上删除的是较重的 OpenSpec/tooling system。本 change 只提交 plain Markdown artifacts；无运行时/构建依赖，也不生成工具专属配置。CHANGELOG 保留旧 Removed 记录，并新增当前 lightweight contract workflow 的事实。

## Fixture rules

- 输入足够小，可公开，不包含敏感数据；
- fixture manifest 包含生成器 commit、生成命令和 hashes；
- 当前 zstd 等依赖可能使 writer 全文件 bytes 随版本改变，因此 frozen archive 用于 decoder compatibility，writer 默认比较 header/field semantics；
- 任何人工修改 fixture 必须由 checksum/结构测试发现。

## Risk

主要风险是为了匹配旧文档而错误修改实现。缓解：本 change 以审计提交 bytes 为事实，只做文档与 characterization；行为修复另开 change。
