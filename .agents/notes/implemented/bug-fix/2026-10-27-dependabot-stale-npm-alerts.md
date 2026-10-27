# Agent Note: 用干净 package-lock 替换残留 npm 依赖图

Status: implemented

## 问题

GitHub 默认分支有 19 条 Dependabot npm 告警，全部指向根目录
`package-lock.json` 中的旧 VitePress 依赖（markdown-it、mermaid、js-yaml 等）。
该文档站依赖和锁文件已在 2026-08-07 的 `b63272f` 中删除；当前根 `package.json`
是 Agent Note 工具，明确没有 npm 依赖，但 GitHub 的 npm dependency graph 仍保留
旧清单，新告警持续产生。

## 决策

为当前无依赖的 note 工具生成并提交一份最小 `package-lock.json`，只包含根包
`fqc-note-tooling`，不含任何第三方 npm 包。让默认分支的 npm 依赖图重新以当前
清单为准，而不是继续引用历史 VitePress 锁文件。

如果推送后旧告警仍未自动关闭，再用 Dependabot API 将其标记为 `not_used`，并在
dismiss comment 中指向本 Note；不通过升级未使用的 VitePress 依赖来“修复”。

## 备选方案

- **删除根 package.json** — 会让 npm 依赖图消失，但也会破坏文档中的
  `npm run verify-notes` 入口；否决。
- **升级旧 VitePress 依赖** — 这些包已不在当前构建中使用，升级只会维护无用的
  攻击面；否决。
- **只 dismiss 19 条告警** — 能清空列表，但不修复导致告警复现的依赖图状态；
  仅作为干净锁文件推送后仍残留时的收尾手段。
- **在 Dependabot 配置中忽略 npm** — 会掩盖未来的真实 npm 依赖问题；否决。

## 后果

- 仓库多一个约 10 行的无依赖锁文件，不改变运行、构建或 `npm run verify-notes`
  的行为。
- 依赖图刷新后，旧 VitePress 包不再属于默认分支的 npm 清单；若 GitHub 仍需人工
  收尾，告警将按 `not_used` 明确说明原因。
- 本次不升级 Rust 依赖，也不改变 Cargo.lock；Rust 侧安全审计仍由
  `cargo deny` 的四门禁覆盖。

## 验证

- `npm audit --package-lock-only --omit=dev`：`found 0 vulnerabilities`。
- 锁文件解析为 `lockfileVersion: 3`，`packages` 只有根条目，`npm ls --all`
  显示空依赖树。
- 推送 `d134e7d` 后等待 90 秒，open alerts 仍为 19；随后将旧告警逐条
  dismiss 为 `not_used`，复查 API：`open=0`（18 条显示 `dismissed`，另一条
  withdrawn advisory 已不再出现在列表中）。
