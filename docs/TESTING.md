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
- `src/app/input/input_router.rs` and `src/app/input/owner.rs`
  - Active input owner, text-entry priority, and scoped keymap dispatch.
  - 当前输入所有者、文本输入优先级与作用域 keymap 分发。
  - `next_focus_area` / `prev_focus_area` must remain workspace fallback actions; changing their bindings from `Tab` / `Shift+Tab` to `Ctrl+Tab` variants should not require router changes.
  - `next_focus_area` / `prev_focus_area` 必须继续是 workspace fallback action；即使把默认绑定从 `Tab` / `Shift+Tab` 改为 `Ctrl+Tab` 变体，也不应要求修改 router 语义。
  - Current-scope keymap actions must continue to beat workspace fallback shortcuts such as `next_focus_area` or `Ctrl+D` theme toggle.
  - 当前作用域 keymap 动作必须继续优先于 workspace fallback 快捷键，例如 `next_focus_area` 或 `Ctrl+D` 主题切换。
  - `editor.insert.confirm_completion(Tab)` must continue to outrank `next_focus_area(Tab)`.
  - `editor.insert.confirm_completion(Tab)` 必须继续优先于 `next_focus_area(Tab)`。
  - `Ctrl+D` / `Ctrl+Shift+T` / `Alt+K` / `Ctrl+1..6` must keep routing through action-backed workspace fallback instead of reintroducing direct router-only key branches.
  - `Ctrl+D` / `Ctrl+Shift+T` / `Alt+K` / `Ctrl+1..6` 必须继续通过 action-backed workspace fallback 分发，不能重新引入 direct router-only 的按键分支。
- `src/core/keybindings.rs`
  - Missing `keymap.toml` must initialize from defaults, partial files must merge without rewriting disk, and diagnostics must surface unknown sections/actions, invalid bindings, exact conflicts, inherited shadowing, and text-entry plain-character rejection.
  - 缺失的 `keymap.toml` 必须从默认值初始化；局部文件必须以补齐合并方式加载且不重写磁盘；diagnostics 必须覆盖未知 section/action、非法绑定、同 scope 冲突、继承遮蔽和文本输入作用域普通字符拒绝。
- `src/ui/dialogs/keybindings_dialog.rs`
  - Scope tree, action list, binding source, and diagnostics placeholder must stay aligned with runtime bindings instead of falling back to a flat action table.
  - 作用域树、动作列表、绑定来源与 diagnostics 占位必须继续与运行时键位一致，不能退回平铺动作表格。
  - The dialog must expose the real `sidebar.filters.list` scope, keep the legacy-import affordance when `config.toml.keybindings` still differs from defaults, and allow copying the current `keymap.toml` path.
  - 对话框必须暴露真实的 `sidebar.filters.list` scope；当 `config.toml.keybindings` 仍与默认值不一致时保留 legacy 导入入口；并允许复制当前 `keymap.toml` 路径。
  - Scoped action override rows such as `toolbar.refresh` must surface inherited/global source state, local override state, and same-scope diagnostics through the editor instead of hiding behind the legacy local-shortcut list.
  - `toolbar.refresh` 这类 scoped action override 条目必须在编辑器里暴露继承全局、局部覆盖和同 scope 诊断，不能继续藏在旧的局部快捷键列表之后。
  - Text-entry scopes such as `editor.insert` and `sidebar.filters.input` must only list text-entry-safe scoped actions in the editor, and must not expose command-mode-only actions like `refresh`.
  - `editor.insert` 与 `sidebar.filters.input` 这类文本输入作用域在编辑器中只能列出 text-entry-safe 的 scoped action，不能继续暴露 `refresh` 这类只在 command mode 有效的动作。
- `src/app/dialogs/host.rs` and `src/app/surfaces/dialogs.rs`
  - Only the active dialog should process input and produce dialog results.
  - 只有当前 active dialog 能处理输入并产生对话框结果。
- `src/core/commands.rs` and `src/ui/shortcut_tooltip.rs`
  - Scoped command ids must stay unique, and every legacy `LocalShortcut` must have registry metadata.
  - 作用域命令 id 必须保持唯一，每个遗留 `LocalShortcut` 都必须有注册表元数据。
- `src/ui/dialogs/common.rs`
  - `DialogShortcutContext::consume_command` and `resolve_commands` must preserve text-entry priority while consuming scoped command ids.
  - `DialogShortcutContext::consume_command` 与 `resolve_commands` 必须在消费作用域 command id 时继续保持文本输入优先级。
- `src/ui/dialogs/import_dialog/mod.rs` and `src/ui/dialogs/export_dialog.rs`
  - Format switching, refresh/export guards, and text-entry conflicts must stay scoped to the active dialog.
  - 格式切换、刷新/导出门禁与文本输入冲突必须继续限定在当前 active dialog 内。
- `src/ui/dialogs/connection_dialog.rs`
  - Database-type switching, SQLite browse gating, confirm validation, and file-picker side effects must stay routed through the connection dialog action path while preserving text-entry priority.
  - 数据库类型切换、SQLite 浏览门禁、确认校验以及文件选择副作用必须继续通过连接对话框动作路径分发，同时保持文本输入优先级。
- `src/ui/dialogs/create_db_dialog.rs`, `src/ui/dialogs/create_user_dialog.rs`, and `src/ui/dialogs/ddl_dialog.rs`
  - Confirm/dismiss and DDL column-navigation shortcuts must resolve through scoped command ids, and character keys like `j` must not override active text entry.
  - 创建数据库、创建用户与 DDL 对话框的确认/关闭及 DDL 列导航快捷键必须通过 scoped command id 解析，且 `j` 这类字符键不能覆盖当前文本输入。
- `src/ui/components/grid/keyboard.rs`
  - Focus transfer and editing mode transitions, especially the "reach last row first, then transfer on the next down movement" behavior.
  - 焦点转移与编辑模式切换，尤其是“先到最后一行，再由下一次下移触发编辑器切换”的行为。
- `src/ui/panels/sidebar/mod.rs` and `src/ui/panels/sidebar/state.rs`
  - Sidebar focus graph must stay centralized around `Connections -> Databases -> Tables -> Filters -> Triggers -> Routines`.
  - 侧边栏焦点图必须继续集中在 `Connections -> Databases -> Tables -> Filters -> Triggers -> Routines` 这一条主顺序上。
  - `filters.input` must keep plain-character typing local to the value editor, and `Esc` must return to `filters.list` without leaking commands outward.
  - `filters.input` 必须继续把普通字符输入限制在值编辑框内部，且 `Esc` 必须返回 `filters.list`，不能把命令泄漏到外层。
  - `edge_transfer` must keep `j/k` cross-panel transfer behind the config gate while preserving `h/l` layer traversal.
  - `edge_transfer` 必须继续把 `j/k` 跨 panel 转移放在配置开关之后，同时保持 `h/l` 的层级遍历语义。
- `src/ui/components/grid/actions.rs` and `src/app/runtime/handler.rs`
  - Grid saves must batch all SQL statements, keep edits on failure, and refresh back into the same table view on success.
  - 表格保存必须批量执行全部 SQL、失败时保留编辑状态、成功后刷新回同一张表视图。
- `src/core/theme.rs`, `src/ui/styles.rs`, and high-frequency shell widgets (`toolbar`, `query_tabs`, `sql_editor`, `help_dialog`, `sidebar`, `er_diagram`, `grid`)
  - Light themes must keep action labels, hints, menu items, and modal copy readable; avoid reintroducing hard-coded dark-only foreground colors.
  - 日间主题下必须保证操作标签、提示文案、菜单项与对话框正文可读，避免重新引入只适配暗色背景的前景色硬编码。
- `src/ui/dialogs/help_dialog.rs`, `src/ui/dialogs/keybindings_dialog.rs`, and `src/ui/dialogs/picker_shell.rs`
  - Picker dialogs must stay fixed-size, keep layered root/item/detail navigation stable, preserve click-to-open plus keyboard `h/j/k/l` semantics, and ensure detail scrolling never reintroduces horizontal growth.
  - picker 型对话框必须保持固定尺寸，稳定支持 root/item/detail 分级导航，保留单击打开与 `h/j/k/l` 键盘语义，并确保详情滚动不会重新引入横向扩张。
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
