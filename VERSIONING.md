# 版本策略

`fqc` 使用简单语义化版本（SemVer 的子集）：

| 版本段 | 含义 |
|---|---|
| patch（0.1.x） | bug 修复、文档、内部重构，不改变行为 |
| minor（0.x.0） | 向后兼容的新功能 |
| major（x.0.0） | 破坏性变更（格式、CLI、行为） |

项目处于 experimental 阶段：破坏性变更被允许，向后兼容不保证
（见 [AGENTS.md](AGENTS.md) 定位声明）。

## 发布渠道

- **当前尚未发布到 crates.io**。`Cargo.toml` 的 `version` 字段照常维护，但分发渠道是
  **git tag + GitHub Release**。
- 历史发布：`v0.1.0`（2026-03-07）、`v0.1.1`（2026-04-16）。

## 发布流程

1. 整理 [CHANGELOG.md](CHANGELOG.md)：把 `[Unreleased]` 的内容归类到新版本小节
2. 更新 `Cargo.toml` 的 `version`（若适用）
3. 确认 CI 与本地门禁全绿
4. 在 `master` 打 `vX.Y.Z` 格式的 tag
5. 创建 GitHub Release，说明基于 CHANGELOG 的变更摘要
