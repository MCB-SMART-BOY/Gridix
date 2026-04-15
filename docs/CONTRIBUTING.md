# Contributing to Gridix | 贡献指南

Thanks for contributing to Gridix.  
感谢你参与 Gridix 项目。

For the repository-level checklist, read [README.md](README.md), [TESTING.md](TESTING.md), and [RELEASE_PROCESS.md](RELEASE_PROCESS.md) first.  
如需仓库级约定，请先阅读 [README.md](README.md)、[TESTING.md](TESTING.md) 与 [RELEASE_PROCESS.md](RELEASE_PROCESS.md)。

## 1. Development Setup | 开发环境
```bash
git clone https://github.com/MCB-SMART-BOY/Gridix.git
cd Gridix
cargo run
```

Linux dependencies / Linux 依赖：
- Debian/Ubuntu: `sudo apt install libgtk-3-dev libxdo-dev`
- Fedora/RHEL: `sudo dnf install gtk3-devel libxdo-devel`
- Arch: `sudo pacman -S gtk3 xdotool`

## 2. Before Opening PR | 提交 PR 前
Run these checks locally:
本地至少执行以下检查：
```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
python scripts/check_doc_links.py
```

Optional MySQL integration test / 可选 MySQL 集成测试：
```bash
GRIDIX_IT_MYSQL_HOST=127.0.0.1 \
GRIDIX_IT_MYSQL_PORT=3306 \
GRIDIX_IT_MYSQL_USER=root \
GRIDIX_IT_MYSQL_PASSWORD=secret \
GRIDIX_IT_MYSQL_DB=test \
cargo test --test mysql_cancel_integration -- --ignored --nocapture
```

Testing details: [TESTING.md](TESTING.md)  
测试细节见：[TESTING.md](TESTING.md)

## 3. Change Scope | 变更范围建议
- Keep PR focused on one problem.
  一个 PR 只解决一类问题。
- UI behavior changes should include keyboard/focus impact note.
  UI 行为改动请说明对键盘与焦点的影响。
- If shortcuts change, update [KEYBINDINGS.md](KEYBINDINGS.md).
  若快捷键变更，必须同步更新 [KEYBINDINGS.md](KEYBINDINGS.md)。

## 4. Code Style | 代码风格
- Follow `rustfmt` defaults.
  使用 `rustfmt` 默认风格。
- Prefer clear logic over clever one-liners.
  优先可读性，不追求炫技写法。
- Add comments only for non-obvious logic.
  仅在不直观逻辑处添加必要注释。

## 5. Commit Message | 提交信息
- Use clear, task-oriented messages.
  提交信息保持清晰、面向任务。
- Examples:
  - `fix(sql-editor): keep cursor after tab completion`
  - `feat(welcome): add db environment recheck action`
  - `docs: update getting started and troubleshooting`

## 6. Reporting Issues | 问题反馈
- Use GitHub Issues:
  <https://github.com/MCB-SMART-BOY/Gridix/issues>
- Please include:
  - Gridix version
  - OS and environment
  - database type
  - exact reproduction steps
  - expected vs actual behavior

## 7. Documentation Rule | 文档规则
- Behavior change -> update docs in same PR.
  行为变更应在同一 PR 中同步文档。
- Keep docs bilingual in one page when practical.
  文档优先保持中英同页。
- User-visible feature/fix changes should update `CHANGELOG.md`.
  面向用户的功能或修复变更应同步更新 `CHANGELOG.md`。
- Follow writing conventions in [DOCS_STYLE.md](DOCS_STYLE.md).
  文档撰写请遵循 [DOCS_STYLE.md](DOCS_STYLE.md) 规范。
- Release-related changes should update [RELEASE_PROCESS.md](RELEASE_PROCESS.md) when needed.
  发布流程相关变更应按需同步 [RELEASE_PROCESS.md](RELEASE_PROCESS.md)。
