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

- `src/app/input/input_router.rs`, `src/app/input/owner.rs`, `src/app/dialogs/host.rs`, `src/app/surfaces/dialogs.rs`
  - Input owner, dialog owner, modal priority, and scoped dispatch must stay aligned.
  - 输入 owner、dialog owner、模态优先级与作用域分发必须继续一致。
- `src/ui/components/toolbar/mod.rs`, `src/ui/dialogs/toolbar_menu_dialog.rs`, `src/ui/components/toolbar/theme_combo.rs`
  - Toolbar focus visibility, overlay chooser behavior, and the remaining raw theme popup must keep keyboard and viewport behavior stable.
  - 工具栏焦点可见性、overlay 选择器行为，以及剩余的主题 popup 必须继续保持稳定的键盘流和视口边界。
- `src/ui/dialogs/common.rs`, `src/ui/dialogs/picker_shell.rs`, `src/ui/dialogs/help_dialog.rs`, `src/ui/dialogs/keybindings_dialog.rs`
  - Workspace dialog shells must keep fixed header/footer, pane-owned scrolling, and layered `Full / Compact / Hidden` behavior.
  - workspace dialog shell 必须继续保持固定 header/footer、pane 自有滚动，以及 `Full / Compact / Hidden` 的层级退场行为。
- `src/ui/dialogs/connection_dialog.rs`, `src/ui/dialogs/import_dialog/mod.rs`, `src/ui/dialogs/export_dialog.rs`, `src/ui/dialogs/create_db_dialog.rs`, `src/ui/dialogs/create_user_dialog.rs`, `src/ui/dialogs/ddl_dialog.rs`
  - Form dialogs must keep one main scroll owner, fixed footer actions, and text-entry-safe shortcut behavior.
  - 表单类 dialog 必须保持单一主体滚动、固定 footer 动作区，以及不会抢走文本输入的快捷键行为。
  - Manual smoke for `CreateUserDialog`: once a non-SQLite connection is active, verify `Ctrl+Shift+U` can open the dialog, `q` can close it, and a narrow floating viewport still keeps the footer visible while the main form and SQL preview remain scroll-safe.
  - `CreateUserDialog` 手工冒烟：在 active non-SQLite 连接下验证 `Ctrl+Shift+U` 能拉起 dialog、`q` 能关闭，且窄浮窗里 footer 仍可见，主体表单与 SQL 预览区继续保持安全滚动。
- `src/app/runtime/database.rs`, `src/app/runtime/request_lifecycle.rs`, `src/app/runtime/handler.rs`, `src/app/surfaces/render.rs`
  - Query execution must preserve request-id isolation, user-cancel vs stale split, active-tab mirrors, and explicit error rendering.
  - 查询执行链必须继续保持请求 ID 隔离、用户取消与 stale 分流、active tab 镜像一致，以及显式错误渲染。
- `src/ui/components/grid/actions.rs`, `src/ui/components/grid/keyboard.rs`, `src/app/runtime/handler.rs`
  - Grid saves must keep context isolation, failure-state retention, keyboard edit semantics, and same-table refresh on success.
  - grid 保存必须继续保持上下文隔离、失败保留编辑、键盘编辑语义，以及成功后回到同一表视图。
  - Query-result headers must remain readable across focused, filtered, and unfocused columns under the active theme; do not let non-focused headers fall back to effectively invisible implicit colors.
  - 查询结果表格列头在当前主题下必须继续保持可读：焦点列、筛选列和普通列都要有明确可见的前景色，不能再次退化成“只有焦点列可见”。
  - Manual smoke: open any wide result table and confirm column names such as `id / customer_id / label / recipient_name / ...` remain readable together instead of only the focused header standing out.
  - 手工冒烟：打开任意多列表结果表，确认 `id / customer_id / label / recipient_name / ...` 这类列名会一起保持可读，而不是只剩当前焦点列显眼。
- `src/ui/panels/sidebar/*`, `src/app/runtime/database.rs`, `src/app/surfaces/dialogs.rs`
  - Sidebar traversal and destructive actions must keep explicit targets, connection context, and a stable focus graph.
  - 侧边栏遍历与危险操作必须继续保持显式目标、连接上下文和稳定焦点图。
  - Focused anchors should keep both the sidebar entrypoints and the app-level confirmation chain covered: `SidebarDeleteTarget` construction, `handle_sidebar_actions()` saving `pending_delete_target` and opening `DeleteConfirm`, and `confirm_pending_delete()` clearing saved state after dispatch.
  - focused 锚点除了覆盖 sidebar 入口外，还应继续锁住 app 级确认链：`SidebarDeleteTarget` 构造、`handle_sidebar_actions()` 保存 `pending_delete_target` 并打开 `DeleteConfirm`，以及 `confirm_pending_delete()` 在分发后清理已保存状态。
- `src/core/keybindings.rs`, `src/core/commands.rs`, `src/ui/shortcut_tooltip.rs`
  - Registry ids, inherited bindings, diagnostics, and tooltip metadata must stay in sync with runtime keymap behavior.
  - 注册表 id、继承绑定、诊断信息与 tooltip 元数据必须继续与运行时 keymap 行为一致。
- `src/core/theme.rs`, `src/ui/styles.rs`, `src/ui/components/er_diagram/*`, `src/ui/components/toolbar/*`
  - Theme changes must keep light/dark readability and avoid reintroducing dark-only foreground assumptions.
  - 主题调整必须继续保证明暗主题可读性，避免重新引入只适配暗色背景的前景色假设。
- `src/app/input/input_router.rs`, `src/app/runtime/er_diagram.rs`, `src/ui/components/er_diagram/state.rs`, `src/ui/components/er_diagram/render.rs`
  - ER top-level combos must keep `navigation / viewport` scope boundaries stable: opening with `Ctrl+R` must now enter ER focus directly, viewport-mode `q` still closes ER, `r / Shift+L / f` still work there, `Ctrl+R` must still route `ToggleErDiagram` even when ER itself owns focus, open/refresh/reload must reset `interaction_mode` back to navigation, theme-switch renders must not wipe the current ER viewport/selection state, `FocusErDiagram -> ToggleErDiagram` must restore a legal non-ER workspace focus (or fall back to `DataGrid`) without leaving hidden ER scope behind, `ShowHistory` must remain blocked in ER scope, `Shift+J / Shift+K` must keep relation-adjacent navigation deterministic and layout-independent, and `Shift+Arrow` must keep geometry-adjacent navigation directional, local-only, and deterministic for a fixed layout.
  - ER 顶层组合链必须继续保持 `navigation / viewport` 作用域边界稳定：`Ctrl+R` 打开 ER 时现在必须直接切入 ER 焦点，视口模式内的 `q` 仍要关闭 ER，`r / Shift+L / f` 仍要可用，`Ctrl+R` 在 ER 自己持有焦点时仍必须继续路由到 `ToggleErDiagram`，open/refresh/reload 必须把 `interaction_mode` 重置回浏览态，theme switch render 也不能把当前 ER 的视口/选中状态洗掉，而 `FocusErDiagram -> ToggleErDiagram` 还必须恢复一个合法的非 ER workspace 焦点（目标不可用时回退到 `DataGrid`），且不能残留隐藏 ER scope；`ShowHistory` 仍必须在 ER scope 下保持被阻止；新增的 `Shift+J / Shift+K` 关系邻接导航也必须继续保持确定性，不能受当前几何布局影响；新增的 `Shift+Arrow` 几何邻接则必须继续保持方向性、局部性，并且在固定布局下具有确定性。
  - Focused anchors should also keep the new geometry-adjacency boundaries explicit: `Shift+Arrow` must remain `NoOp` in `er_diagram.viewport`, and geometry navigation must not pre-sync app-level `selected_table` before `OpenSelectedTable`.
  - focused 锚点还应继续显式锁住新的几何邻接边界：`Shift+Arrow` 在 `er_diagram.viewport` 中必须保持 `NoOp`，且几何邻接在 `OpenSelectedTable` 之前不得提前同步 app 层 `selected_table`。

## 5.1 UX / Input Recovery Live Smoke | UX / 输入恢复主线 live 冒烟

当 `toolbar / sidebar connection-row / AboutDialog / workspace dialog header / ER top-level combo` 这批条目进入“已关闭 / 观察”状态后，建议至少补一轮 live smoke，而不是只依赖 unit tests：

1. `Alt+A` 打开操作菜单，确认 chooser 正常打开、顶部 header 为紧凑双区块、`q` 可关闭。
2. `Alt+N` 打开新建菜单，确认 chooser 正常打开、顶部 header 为紧凑双区块、`q` 可关闭。
3. `Alt+K` 打开快捷键设置，确认顶部搜索/重置区与 breadcrumb/提示区在默认宽度下并排展示。
4. `F1` 打开帮助与学习，确认顶部快捷键提示区与 breadcrumb/提示区在默认宽度下并排展示。
5. `Ctrl+P -> about -> Enter` 打开 About，确认首屏不再回到旧的厚重双 section 堆叠。
6. 右键 connection-row，确认菜单稳定弹出；点击 `删连 / 删库` 后必须进入 `DeleteConfirm`，而不是只构造 helper target。
7. 在 ER 已打开时验证 `Ctrl+R` 的关/开，以及 `Alt+R` 只负责聚焦 ER。
8. 沿 `OpenLearningSample -> WelcomeSetup -> 5 -> Ctrl+Shift+N` 打开 `DdlDialog`，在约 `760x560` 级窄窗口下确认 footer 仍可见，且“列定义 / 预览 SQL” 不会再把整窗顶出视口。
9. 沿 `Ctrl+P -> sample -> Enter -> Ctrl+R -> Alt+R -> Shift+Right/Down/Left/Up` 进入学习样例 ER，确认 `Shift+Arrow` 的几何邻接没有明显误跳，且选中项仍会随视口进入可见区域。
10. 在同一学习样例 ER 链路下继续执行 `Shift+L -> Shift+Down -> f -> Ctrl+Shift+T -> j -> Enter`，确认 relayout / fit view / theme switch 不会把 ER 当前布局、缩放和几何邻接稳定性一起冲掉；若 theme chooser 关闭后要继续走 ER 局部键盘浏览，可显式再按一次 `Alt+R` 重新聚焦。

注意：
- 当前 Wayland live smoke 已确认 `Alt+A / Alt+N / Alt+K / F1 / Ctrl+P -> about` 这些显式路径通过。
- `DdlDialog` 的 live 窄视口复核也已通过：沿 `OpenLearningSample -> WelcomeSetup -> 5 -> Ctrl+Shift+N` 打开“创建表”后，footer 已重新保持可见。
- `Ctrl+R` 的自动注入验证目前仍不可靠：`wtype` 与 `hyprctl dispatch sendshortcut` 在当前 grid 场景下会落成 bare `r` 行为，因此这条必须优先用实体键手动复核；在拿到实体键复核前，不要仅凭自动注入失败就重新打开 ER bug。
- ER 几何邻接已完成第一轮 live 观察：当前 Wayland 会话下，沿 `Ctrl+P -> sample -> Enter -> Ctrl+R -> Alt+R -> Shift+Right/Down/Left/Up` 进入学习样例 ER 后，方向跳转与 `selection follows viewport` 没有复现明显误跳；当前先保持观察，不继续猜测性调算法。
- ER 几何邻接已完成第二轮组合观察：继续执行 `Shift+L -> Shift+Down -> f -> Ctrl+Shift+T -> j -> Enter` 后，relayout / fit view / theme switch 也没有把当前 ER 布局、缩放和几何邻接稳定性冲掉；当前仍先保持观察，不继续猜测性调算法。

## 6. Issue Reproduction Template | 问题复现模板

When filing or validating a bug:
提交或验证缺陷时建议记录：

- Version / 版本
- OS / 系统环境
- Steps / 复现步骤
- Expected vs Actual / 预期与实际
- Logs or screenshot / 日志与截图
