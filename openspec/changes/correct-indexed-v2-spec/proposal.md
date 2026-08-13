# Change Proposal: correct-indexed-v2-spec

## Metadata

- Status: `Applying`
- Repository: `open-genomics/fq-compressor-rust`
- Audit base: `1a2a2161bed88df5e21ce6e83ac3b668a93a44b0`
- Capability: `archive-format`
- Task IDs: `FQCR-SPEC-001`, `ORG-GOV-001`, `ORG-CONTRACT-001`
- Decision IDs: `FQC-DEC-001`
- Reviewer: organization owner

## Why

公开格式规范与审计提交的真实 reader/writer 不一致：codec ID 的高/低 nibble 语义、checksum ID `0`、版本兼容范围和所谓 v1 fallback 均有错误陈述。第三方或未来维护者照文档实现会生成当前 reader 无法读取的 archive。

## Changes

**Indexed v2 wire contract**

- From: 文档中的 identifier 和兼容声明不能准确描述当前 bytes。
- To: `fqc-indexed/v2` 规范精确记录 magic、版本、codec/checksum 编码和 rejection 行为，并由冻结 archive fixture 保护。
- Reason: 让 bytes、reader 和规范形成单一事实。
- Impact: 不改变既有 CLI、`.fqc` 后缀、magic 或算法；文档中错误的兼容承诺被收窄。

## Scope

- 建立最小仓库内 `openspec/` 上下文和 `archive-format` change；
- 将根 `AGENTS.md` 的重型 spec-management 禁令收窄为：高风险契约使用纯 Markdown change，不引入 CLI/Node/编辑器元工具；
- 修正 `docs/reference/format-spec.md` 及直接引用该表的文档；
- 添加结构级测试和至少一个由审计基线 writer 生成的冻结 fixture/manifest；
- 确认未知 codec/checksum/version 被确定拒绝。

## Out of scope

- 不改 `fqc` 产品名或 `.fqc` 后缀；
- 不改 magic、archive layout 或压缩算法；
- 不实施四 stream codec dispatch、资源预算、原子输出、CI 或跨族识别；
- 不新增 v1 reader。
- 不恢复旧 OpenSpec CLI、Node/docs site、编辑器 skills 或治理样板。

## Compatibility and rollback

这是规范校正和 characterization change。既有合法 indexed v2 archive 必须继续可读；若测试暴露 writer 与 reader 自身不一致，停止 apply，另建行为修复 change。回滚文档不能回滚已经冻结并验证的 bytes 事实。

## Approval

- Decision scope: `FQC-DEC-001` 已批准保留 `fqc/.fqc`
- Apply approval: `authorized by organization owner`
