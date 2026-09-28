# Agent Note: 采用 .agents/notes 决策记录并停用 openspec 变更流程

Status: implemented

## 问题

仓库对"为什么这样改、放弃了什么"没有统一的检索去处。`openspec/` 只覆盖高风险改动的提案与 spec 增量，归档后按 capability 合并，被否路线与妥协理由散落其中，无法按生命周期检索；普通非平凡改动则完全没有记录义务。后来的维护者（人或 Agent）只能重新推导已否决的路线。

## 决定

`.agents/notes/` 是本仓库唯一的决策记录系统。路径即身份：`{proposed,implemented,rejected,archived}/{feature,bug-fix,simplification,architecture,process,testing}/yyyy-mm-dd-topic.md`，格式与分类门禁由 `.agents/skills/write-notes-like-deepseek/scripts/` 下的 tsx 脚本强制，经根目录 `package.json` 的 npm scripts 调用（`npm run verify-notes`）。校验只在本地跑，不进 CI——四门禁 Rust CI 保持纯 Rust。

`openspec/` 变更流程停用：不再新建 `openspec/changes/`。既有内容保留：`openspec/specs/` 继续作为能力规范参照，`openspec/changes/archive/` 保留历史。在途变更（`document-fqc-format-family`、`harden-indexed-v2-decode-integrity`）可在旧流程下完成后归档，其决策要点以 Note 落入本树。

`docs/architecture/decisions/` 的 ADR 体系同样并入本树：三篇 ADR 已折叠为 `implemented/architecture/` 下的 Note（块索引格式、三种执行模式、组件分离编码），docs 不再保留独立的决策记录目录。`AGENTS.md` 与 `CONTRIBUTING.md` 已同步指向本约定。

## 备选方案

- **并行双层**（openspec 管高风险提案，notes 管决策记录）— 分工最清晰、各司其职；否决理由：业余单维护者的项目承担不起两套文书流程，边界模糊（"这条写哪边"）的结果往往是两边都不写。
- **notes 只补 openspec 缺口** — 保留 openspec 对高风险改动的 spec 增量纪律；否决理由：openspec 的 proposal/design 与 Note 的 Problem/Alternatives 高度重叠，部分覆盖制造持续的归类成本。
- **维持只用 openspec** — 零迁移成本；否决理由：它是 spec 增量视角，没有 `rejected/` 生命周期与就地同步契约，被否路线仍会重演。

## 后果

- **收益**：非平凡改动的动机、被否路线、验证表面集中一处且可按 lifecycle/class 物理切片检索；格式纪律有脚本兜底而非靠自觉。
- **代价与已知上限**：本地校验引入 Node/tsx 工具链（不进 CI，缺失时 `npx` 按需拉取）；openspec 的 spec 增量强制力随之失效，高风险格式改动失去 proposal/design/tasks 的硬门禁——若高风险改动重新需要 spec 级契约，重访本决定而不是悄悄恢复 openspec；历史决策仍在 `openspec/changes/archive/` 中，检索需跨目录。

## 验证

`npm run verify-notes` 在仓库根目录跑通全部三条校验线；`AGENTS.md`「非平凡改动」一节与 `CONTRIBUTING.md` 流程段均以本树为准。
