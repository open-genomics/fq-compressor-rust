# Verification: document-fqc-format-family

## Metadata

- Verification status: `Implemented — evidence recorded, awaiting reviewer`
- Implementation HEAD: `2c5290186f4e2be5498e681e771376674f0c168f`
- Verifier: `(implementer; independent reviewer pending)`
- Verified at: `2026-08-19`
- Ready to archive: `no`

## Scope audit

- Expected files/modules: `README.md`、格式文档首页、`openspec/`
- Actual changed files: `README.md`（首屏对照表 + 安装 PATH 提醒）、
  `openspec/changes/document-fqc-format-family/tasks.md`、`verification.md`
- Unexpected changes: 无（`docs/reference/format-spec.md` 已含完整格式族共存说明与
  双方 magic，无需同步；未改任何 Rust 源文件）
- Existing user changes preserved: 是——apply 前工作区仅有未跟踪的
  `openspec/changes/document-fqc-format-family/`（本 change 的 spec 文件），
  已保留；无其他用户改动被覆盖

## Requirement traceability

| Requirement | Scenario | Test/command | Result | Evidence summary |
|---|---|---|---|---|
| Same-name format family coexistence is documented | User opens the README first screen | `grep` 首屏对照表 | pass | README 第 8–18 行：两仓库对照表（仓库/实现语言/格式族 ID/完整 magic/访问模型/对方链接） |
| Same-name format family coexistence is documented | User reads about cross-implementation decode | `grep` magic/解码说明 | pass | README 第 17–18 行：扩展名不能判定格式、reader 必须检查 magic、不能互相解码；第 14–15 行含双方完整 magic |
| Same-name binary PATH risk is documented | User installs both binaries | `grep` 安装说明 PATH 提醒 | pass | README 第 39–41 行（安装节）：同名 `fqc` 二进制 `PATH` 覆盖风险与 `which fqc` 检查 |
| No product-name or extension migration content | User searches documentation for migration claims | `grep` 迁移声明 | pass | `grep -nE '迁移\|改名\|更名\|renam\|migrat' README.md` 无匹配 |
| Coexistence claims stay within implemented behavior | User searches for automatic dispatch | `grep` 无自动分派声明 | pass | `grep -nE '自动分派\|自动识别' README.md` 无匹配；`grep -nE 'C\+\+ ?版\|Rust ?版'` 无匹配（无“C++/Rust 版本”暗示同格式） |

## Commands

| Command | Exit status | Result summary |
|---|---:|---|
| `git status --short` | 0 | `?? openspec/changes/document-fqc-format-family/`（spec 目录）+ ` M README.md` |
| `git rev-parse HEAD` | 0 | `2c5290186f4e2be5498e681e771376674f0c168f` = base commit，HEAD 无差异 |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo clippy --all-targets -- -D warnings` | 0 | 通过 |
| `cargo test --lib --tests` | 0 | 通过（首次全量运行时 `test_e2e_pipeline_respects_save_reorder_map_flag` 出现 1 次 flaky “channel closed” 失败；单测隔离重跑 3 次全过，完整 e2e 套件与全量测试重跑均通过，判定为既有并发 flaky，与本 docs-only change 无关） |
| `cargo doc --no-deps` | 0 | 通过，生成 target/doc/fqc/index.html |
| `grep -n '89 46 51 43 0D 0A 1A 0A' README.md` | 0 | 命中第 14 行 |
| `grep -n '46 51 43 56 32 0D 0A 1A' README.md` | 0 | 命中第 15 行 |
| `grep -nE 'C\+\+ ?版\|Rust ?版' README.md` | 1（期望无匹配） | 无“C++/Rust 版本”暗示同格式 |
| `grep -nE '自动分派\|自动识别' README.md` | 1（期望无匹配） | 无自动分派声明 |
| `grep -nE '迁移\|改名\|更名\|renam\|migrat' README.md` | 1（期望无匹配） | 无产品名/后缀迁移内容 |
| `grep -n 'PATH' README.md` | 0 | 命中第 39–41 行 |
| `git diff --check` | 0 | 无空白错误，diff 仅含 README.md |

## Not run

- Archive（4.1–4.3）：需 reviewer 确认 `Ready to archive: yes` 后执行；本次未归档。

## Residual risks

- 与 C++ 仓库 README 的对照表可能漂移；两个独立 change 需在同一契约下各自完成
  （`fq-compressor` 的 `document-fqc-format-family` 独立进行中）。
- `test_e2e_pipeline_respects_save_reorder_map_flag` 为既有并发 flaky 测试
  （“channel closed”竞态），重跑通过；非本 change 引入。
- 完整 magic 以 hex + escaped bytes（`0x89 46 51 43 0D 0A 1A 0A`，PNG 风格签名）
  展示；若未来格式族 ID 或 magic 变更，需同步本表与 `docs/reference/format-spec.md`。

## Verdict

实现完成，验收场景全部通过；`Ready to archive` 等待独立 reviewer 确认后置 yes 再归档。
