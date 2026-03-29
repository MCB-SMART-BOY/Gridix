# Testing Guide | 测试指南

## 1. Quick Commands | 常用命令

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
nix --extra-experimental-features 'nix-command flakes' flake check --no-write-lock-file
python scripts/check_doc_links.py
```

Recommended local order / 建议本地执行顺序：
1. `cargo fmt`
2. `cargo clippy`
3. `cargo test`

## 2. Test Layout | 测试结构

Current test files in `tests/` include:
- `autocomplete_tests.rs`
- `core_tests.rs`
- `database_tests.rs`
- `ddl_dialog_tests.rs`
- `ddl_tests.rs`
- `edge_regression_tests.rs`
- `export_tests.rs`
- `formatter_tests.rs`
- `grid_tests.rs`
- `mysql_cancel_integration.rs`
- `ssh_tests.rs`
- `syntax_tests.rs`
- `ui_dialogs_tests.rs`

## 3. Integration Test (MySQL) | MySQL 集成测试

This test is ignored by default and requires MySQL service.  
该测试默认忽略，需要本机或 CI 提供 MySQL 服务。

```bash
GRIDIX_IT_MYSQL_HOST=127.0.0.1 \
GRIDIX_IT_MYSQL_PORT=3306 \
GRIDIX_IT_MYSQL_USER=gridix \
GRIDIX_IT_MYSQL_PASSWORD=gridix \
GRIDIX_IT_MYSQL_DB=gridix_test \
cargo test --test mysql_cancel_integration -- --ignored --nocapture
```

## 4. CI Coverage | CI 覆盖范围

- `.github/workflows/docs.yml`
  - Markdown local-link validation (`scripts/check_doc_links.py`).
- `.github/workflows/build.yml`
  - Cross-platform release build checks (Linux/Windows/macOS ARM).
- `.github/workflows/mysql-integration.yml`
  - Scheduled MySQL cancellation integration validation.

## 4.1 Edge Regression Suite | 边缘回归测试

The `edge_regression_tests.rs` suite focuses on:
`edge_regression_tests.rs` 重点覆盖：

- Autocomplete boundary behavior (unicode cursor, out-of-range cursor, dedup, result cap).
  自动补全边界行为（Unicode 光标、越界光标、去重、结果数量上限）。
- Session/tab boundary behavior (invalid index remove/switch).
  会话与标签页边界行为（越界索引删除/切换）。
- Welcome onboarding state machine transitions.
  欢迎页新手引导状态机流转。

Run only edge regressions:
仅运行边缘回归测试：
```bash
cargo test --test edge_regression_tests
```

## 5. High-Risk Areas To Verify | 高风险回归区域

Before merging changes touching these modules, run focused checks:
涉及以下模块时建议重点回归：

- `src/ui/components/sql_editor.rs`
  - `Tab` completion acceptance and cursor position.
  - `Tab` 补全确认后光标位置是否正确。
- `src/app/keyboard.rs`
  - Global shortcut conflicts with local area shortcuts.
  - 全局快捷键是否与局部快捷键冲突。
- `src/ui/components/grid/keyboard.rs`
  - Focus transfer and editing mode transitions.
  - 焦点转移与编辑模式切换。
- `src/ui/dialogs/help_dialog/*`
  - Layout stability and topic navigation.
  - 布局稳定性与知识点导航链路。

## 6. Issue Reproduction Template | 问题复现模板

When filing or validating a bug:
提交或验证缺陷时建议记录：

- Version / 版本
- OS / 系统环境
- Steps / 复现步骤
- Expected vs Actual / 预期与实际
- Logs or screenshot / 日志与截图
