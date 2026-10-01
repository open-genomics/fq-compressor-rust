# Agent Note: openspec 在途变更全部归档收口

Status: implemented

## 问题

变更工作流已于 2026-09-27 由笔记体系取代（见 [agent-notes-replace-openspec](2026-09-27-agent-notes-replace-openspec.md)），但目录里残留两个非终态变更：`harden-indexed-v2-decode-integrity` 停在 `Applying`（tasks 0/8），而其全部工作实际已随 `21308f0` 落地；`document-fqc-format-family` 的 tasks 1-3 已完成并验证（README 共存对照表与 PATH 提醒随 `f46bb69` 落地），归档卡在"等待独立 reviewer"。僵尸状态会误导后续维护者与自动化 agent 重做或误判进度——本次项目扫描即因此立卡。

## 决定

两个在途变更于 2026-10-01 按原工作流的归档步骤收口；此后 `openspec/` 全目录只读，不再存在在途变更。

- `harden-indexed-v2-decode-integrity`：tasks 全部勾选，verification 填入证据（实现提交 `21308f0`；2026-10-01 复核时 fmt / clippy / lib+tests / doc 门禁与冻结 fixture 验证全部通过），Status 置 `Archived`，移入 `archive/2026-10-01-harden-indexed-v2-decode-integrity/`。其格式决策以 [indexed v2 可选块校验和与 ABC v3 codec 标识](../architecture/2026-10-01-indexed-v2-optional-block-checksum-abc-v3-codec.md) 补记。
- `document-fqc-format-family`：delta 同步为 `openspec/specs/format-governance/spec.md`（同名格式族共存的文档约束），`project.md` 能力表与归档清单同步更新，归档至 `archive/2026-10-01-document-fqc-format-family/`。
- 独立 reviewer 缺位的处理：原实现（2026-08 会话）与本次收口（2026-10-01 会话）不是同一智能体会话；收口会话逐条复核 delta scenario 与 README 现状（grep 证据已录于原 `verification.md`）后签署 Ready to archive。此替代审查方式记录于此，供未来同类收口参考。

## 备选方案

- **直接删除两个变更目录** — 最强论据：工作流已退役，给已死流程做归档纯属仪式；否决理由：`openspec/changes/archive/` 是决策证据链（`project.md` 明言其为 historical evidence），删除会丢掉 design / verification 中的论证细节。
- **维持原状不动** — 最强论据：零工作量、零风险；否决理由：`Applying`/`Proposed` 僵尸状态已在实践中造成误读，且 harden 变更缺决策笔记的缺口正是僵尸状态的衍生品。
- **恢复 openspec 工作流做完整评审** — 最强论据：最符合原流程的字面要求；否决理由：与 2026-09-27 的退役决定直接冲突，为一个纯文档变更重启整套流程得不偿失。

## 后果

- **收益**：`openspec/` 状态与事实一致（全部终态）；能力规格表新增 `format-governance`；后续 agent 不会再读到"0/8 任务进行中"的假信号。
- **代价**：`openspec/AGENTS.md` 描述的工作流与目录现状彻底脱钩（该文件已带 Superseded 横幅）；未来若要变更能力规格，路径是写笔记并直接修改 `openspec/specs/`，这条纪律靠自觉维持、无脚本兜底。

## 验证

`npm run verify-notes` 通过；归档后 `openspec/changes/` 顶层仅剩 `archive/`，无任何非终态变更目录。
