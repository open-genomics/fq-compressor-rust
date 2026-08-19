# Tasks: document-fqc-format-family

## 1. Baseline and tests

- [x] 1.1 记录 `git status --short`、HEAD 与 base commit 差异
      （`?? openspec/changes/document-fqc-format-family/`；HEAD = base =
      `2c5290186f4e2be5498e681e771376674f0c168f`，无差异）
- [x] 1.2 记录当前 README 首屏格式族说明；运行 `git diff --check`
      （原首屏：单行 blockquote，`fqc-indexed/v2` + magic + 一句与 C++ 区分；
      `git diff --check` 退出 0）
- [x] 1.3 添加文档一致性检查（grep/测试）：首屏共存对照表、完整 magic、
      无“C++/Rust 版本”暗示同格式、安装 `PATH` 风险提醒、无产品名/后缀迁移内容
      （具体 grep 命令见 3.1，全部通过）

## 2. Implementation

- [x] 2.1 在 README 首屏添加两个 `fqc`/`.fqc` 同名共存对照表（仓库、实现语言、
      格式族 ID、完整 magic、访问模型、对方链接）——README 第 8–18 行
- [x] 2.2 明确扩展名不能判定格式、reader 必须检查 magic、两个实现不能互相解码
      ——README 第 17–18 行
- [x] 2.3 安装说明提醒同名二进制 `PATH` 覆盖风险——README 第 39–41 行
- [x] 2.4 检查不再使用“C++/Rust 版本”暗示同格式的措辞；不宣称跨实现自动分派；
      确认无产品名/archive 后缀迁移内容
      ——`grep -nE 'C\+\+ ?版|Rust ?版'`、`grep -nE '自动分派|自动识别'`、
      `grep -nE '迁移|改名|更名|renam|migrat'` 均无匹配

## 3. Verification

- [x] 3.1 运行文档一致性检查（2026-08-19，全部通过）：
  - `grep -n '89 46 51 43 0D 0A 1A 0A' README.md`（完整 magic）→ 命中第 14 行
  - `grep -n '46 51 43 56 32 0D 0A 1A' README.md`（对方 magic）→ 命中第 15 行
  - `grep -nE 'C\+\+ ?版|Rust ?版' README.md`（无“C++/Rust 版本”暗示同格式）→ 无匹配
  - `grep -nE '自动分派|自动识别' README.md`（无自动分派声明）→ 无匹配
  - `grep -nE '迁移|改名|更名|renam|migrat' README.md`（无产品名/后缀迁移内容）→ 无匹配
  - `grep -n 'PATH' README.md`（安装 PATH 风险提醒）→ 命中第 39–41 行
  - 首屏（前 20 行）含 `fqc`/`.fqc` 共存对照表 → 命中第 8–18 行
- [x] 3.2 运行仓库标准门禁：
  - `cargo fmt --all -- --check` → 退出 0
  - `cargo clippy --all-targets -- -D warnings` → 退出 0
  - `cargo test --lib --tests` → 见 verification.md
  - `cargo doc --no-deps` → 见 verification.md
- [x] 3.3 逐条核对 delta spec scenarios 并填写 `verification.md`
- [x] 3.4 运行 `git diff --check`，确认 diff 仅含本仓库文档——退出 0，仅 README.md

## 4. Archive readiness

- [ ] 4.1 Reviewer 确认 `verification.md` 的 `Ready to archive: yes`
- [ ] 4.2 将 delta 同步到主规格
- [ ] 4.3 按日期归档 change
