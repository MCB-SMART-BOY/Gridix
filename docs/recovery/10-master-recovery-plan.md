# Gridix 6.0.0 Master Recovery Plan

## Scope

This plan now records the `v6.0.0` post-release closure state of the recovery stream carried forward in [Cargo.toml](../../Cargo.toml).

It does **not** propose a broad rewrite.  
It establishes a repair order for the flows that Gridix cannot afford to break, using the current codebase and the current dependency baseline.

Current release gate:

- Local AI / agent guidance is now treated as workstation-local material and should stay out of tracked repository policy docs.
- The release/distribution sequence is already documented in [docs/RELEASE_PROCESS.md](../RELEASE_PROCESS.md) and [docs/DISTRIBUTION.md](../DISTRIBUTION.md).
- The current recovery stream has now reached **`v6.0.0` post-release closure**: there are no new unblocked active implementation workstreams left in the recovery ledger, and the remaining items have been reduced to `observation / live smoke / downstream follow-up`.
- `v6.0.0` has already been committed, tagged, and published on GitHub Releases from this worktree.
- Downstream sync has been executed for AUR and Homebrew, and nixpkgs has been pushed to a clean fork branch with PR [NixOS/nixpkgs#510299](https://github.com/NixOS/nixpkgs/pull/510299).
- Current closure-review check status on this worktree:
  - `cargo fmt --check`: pass
  - `cargo test`: pass
  - `python scripts/check_doc_links.py`: pass
  - `cargo clippy --all-targets --all-features -- -D warnings`: pass
- Therefore the current phase is no longer "ready for release closure review" or "release execution"; it is now **post-release closure**, with the remaining work in this stream limited to external platform propagation, nixpkgs review/merge follow-up, and any newly confirmed bugs.

## A. Dependency Baseline

Repository truth is the combination of `Cargo.toml` and `Cargo.lock`.

| Dependency | Actual version | Why it matters |
|---|---:|---|
| `egui` | `0.34.1` | all dialogs, editor, grid, ER canvas, input surfaces |
| `eframe` | `0.34.1` | app shell, frame lifecycle, native integration |
| `egui_extras` | `0.34.1` | UI helpers and image support |
| `egui_glow` | `0.34.1` | native renderer pulled by `eframe` |
| `epaint` / `emath` | `0.34.1` | low-level paint/layout primitives used by egui |
| `image` | `0.25.10` | assets and image-backed UI surfaces |
| `rfd` | `0.17.2` | file dialogs |
| `syntect` | `5.3.0` | SQL syntax highlighting |

Important current fact:

- Grid and ER are **custom components**, not third-party widgets.
- There is no dedicated graph crate and no dedicated table crate in the dependency graph.
- Therefore dialog/layout compatibility work should target **`egui 0.34.1` APIs already used in the repo**, especially `egui::Window`, `ScrollArea`, and size/constraint methods in [src/ui/dialogs/common.rs](../../src/ui/dialogs/common.rs).

## B. Recovery Workstreams

### 1. Core Logic / Core Flows

Focus:

- startup/init
- connection lifecycle
- tab lifecycle
- SQL execute path
- query result render path
- grid save/refresh path
- destructive action path

Primary files:

- [src/app/mod.rs](../../src/app/mod.rs)
- [src/app/runtime/database.rs](../../src/app/runtime/database.rs)
- [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs)
- [src/app/runtime/request_lifecycle.rs](../../src/app/runtime/request_lifecycle.rs)
- [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs)

Primary objective:

- make the entrypoint -> state write -> runtime message -> render path explicit for every core flow.

### 2. Business Logic & State Integrity

Focus:

- state ownership in `DbManagerApp`
- duplicate mirrors (`self.*` vs `tab.*`)
- request tracking maps
- workspace identity and grid save context
- destructive-action target integrity

Primary files:

- [src/app/mod.rs](../../src/app/mod.rs)
- [src/app/state/mod.rs](../../src/app/state/mod.rs)
- [src/ui/components/query_tabs.rs](../../src/ui/components/query_tabs.rs)
- [src/ui/components/grid/state.rs](../../src/ui/components/grid/state.rs)
- [src/app/action/action_system.rs](../../src/app/action/action_system.rs)

Primary objective:

- reduce “looks like state” fields into clear categories: authority, mirror, cache, UI-only, transition residue.

### 3. Dialogs / Layout / Egui Compatibility

Focus:

- modal ownership
- open/close/confirm/cancel routing
- width/height constraints
- vertical/horizontal overflow
- workspace dialog pane sizing
- compatibility with `egui 0.34.1` `Window` / `Resize` / `ScrollArea` behavior

Primary files:

- [src/app/dialogs/host.rs](../../src/app/dialogs/host.rs)
- [src/app/surfaces/dialogs.rs](../../src/app/surfaces/dialogs.rs)
- [src/ui/dialogs/common.rs](../../src/ui/dialogs/common.rs)
- [src/ui/dialogs/picker_shell.rs](../../src/ui/dialogs/picker_shell.rs)
- [src/ui/dialogs/help_dialog.rs](../../src/ui/dialogs/help_dialog.rs)
- [src/ui/dialogs/keybindings_dialog.rs](../../src/ui/dialogs/keybindings_dialog.rs)
- [src/ui/dialogs/connection_dialog.rs](../../src/ui/dialogs/connection_dialog.rs)
- [src/ui/dialogs/import_dialog/mod.rs](../../src/ui/dialogs/import_dialog/mod.rs)
- [src/ui/dialogs/create_user_dialog.rs](../../src/ui/dialogs/create_user_dialog.rs)
- [src/ui/dialogs/ddl_dialog.rs](../../src/ui/dialogs/ddl_dialog.rs)

Primary objective:

- shell owns size constraints, content owns overflow.

### 4. ER Diagram Design Language

Focus:

- workspace role inside the central pane
- keyboard-flow identity and focus ownership
- visual language consistency
- theme token use
- toolbar/control chrome
- loading/empty/error states
- canvas interaction affordances
- state ownership for `show` vs data vs interaction

Primary files:

- [src/app/runtime/er_diagram.rs](../../src/app/runtime/er_diagram.rs)
- [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs)
- [src/ui/components/er_diagram/state.rs](../../src/ui/components/er_diagram/state.rs)
- [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs)
- [src/ui/components/er_diagram/layout.rs](../../src/ui/components/er_diagram/layout.rs)

Primary objective:

- make ER feel like a first-class Gridix surface instead of a separate visual subsystem, with explicit workspace ownership and keyboard semantics.

## C + D. Core Flows Ledger

### 1. Startup & Initialization

- Goal: boot into a valid app shell with runtime, config, keybindings, theme, and one usable query tab.
- Entry: [DbManagerApp::new()](../../src/app/mod.rs), [impl eframe::App::ui/on_exit](../../src/app/mod.rs).
- Authority: `app_config`, `keybindings`, `theme_manager`, `manager`, `tab_manager`, `runtime`.
- Invariants: runtime exists; config loads before first frame; theme and UI scale are applied before rendering; `QueryTabManager::new()` yields an initial tab; `refresh_welcome_environment_status()` runs once on boot.
- Known risks: recovery baseline docs are incomplete; `pending_toggle_dark_mode` looks like historical residue (`推测`); startup correctness is not yet traced in `docs/recovery/`.
- Related files: [src/app/mod.rs](../../src/app/mod.rs), [src/bootstrap.rs](../../src/bootstrap.rs), [src/app/surfaces/preferences.rs](../../src/app/surfaces/preferences.rs), [src/core](../../src/core).

### 2. Connect / Select Database / Disconnect

- Goal: connect to the intended datasource, load the right schema scope, and clear all state on disconnect.
- Entry: [save_connection_from_dialog()](../../src/app/runtime/database.rs), [connect()](../../src/app/runtime/database.rs), [select_database()](../../src/app/runtime/database.rs), [disconnect()](../../src/app/runtime/database.rs), [handle_connected_with_tables()](../../src/app/runtime/handler.rs), [handle_connected_with_databases()](../../src/app/runtime/handler.rs), [handle_database_selected()](../../src/app/runtime/handler.rs).
- Authority: `manager`, `manager.active`, `pending_connect_requests`, `pending_database_requests`, `sidebar_panel_state`, `autocomplete`.
- Invariants: only the latest connect/select response for a connection is accepted; disconnect cancels pending queries, clears pools/SSH tunnel, removes connection-scoped grid workspaces, and clears active connection state.
- Known risks: request maps and UI mirrors can drift under rapid reconnect/select; history restoration and connection context may still bleed across transitions (`推测`).
- Related files: [src/app/runtime/database.rs](../../src/app/runtime/database.rs), [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs), [src/database/connection.rs](../../src/database/connection.rs), [src/database/pool.rs](../../src/database/pool.rs), [src/database/ssh_tunnel.rs](../../src/database/ssh_tunnel.rs).

### 3. Tab Switch / Create / Close

- Goal: preserve each tab’s draft and workspace, and never leak active-tab state across tab boundaries.
- Entry: [open_new_query_tab()](../../src/app/input/input_router.rs), [select_next_query_tab()](../../src/app/input/input_router.rs), [select_previous_query_tab()](../../src/app/input/input_router.rs), [close_active_query_tab()](../../src/app/input/input_router.rs), [handle_tab_actions()](../../src/app/surfaces/render.rs).
- Authority: `tab_manager`; mirrors are `self.sql`, `self.result`, `self.last_query_time_ms`, current tab-local status fields.
- Invariants: before switching/closing, current draft and grid workspace are persisted; after switching, [sync_from_active_tab()](../../src/app/runtime/request_lifecycle.rs) restores the active tab mirrors; closing a tab removes its grid workspaces and cancels its pending request if needed.
- Known risks: still mirror-heavy by design; active-tab mirrors remain a structural cost even after the current recovery set was landed.
- Related files: [src/app/input/input_router.rs](../../src/app/input/input_router.rs), [src/app/runtime/request_lifecycle.rs](../../src/app/runtime/request_lifecycle.rs), [src/ui/components/query_tabs.rs](../../src/ui/components/query_tabs.rs), [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs).

### 4. SQL Edit & Execute

- Goal: execute the SQL visible in the active editor against the active connection and track it with a unique request ID.
- Entry: [SqlEditor::show()](../../src/ui/components/sql_editor.rs), [handle_sql_editor_actions()](../../src/app/surfaces/render.rs), [AppAction::RunCurrentSql](../../src/app/action/action_system.rs), [execute()](../../src/app/runtime/database.rs).
- Authority: `manager.active`, `next_query_request_id`, `pending_query_tasks`, `pending_query_connections`, active `QueryTab`.
- Invariants: empty SQL does not dispatch; execution requires an active valid connection; request IDs are non-zero and unique; dispatch clears stale per-tab render mirrors before the async request starts.
- Known risks: `self.sql` vs `tab.sql` is still a dual-source model; execution still depends on app-level pending-task registries plus tab-local mirrors.
- Related files: [src/ui/components/sql_editor.rs](../../src/ui/components/sql_editor.rs), [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs), [src/app/runtime/database.rs](../../src/app/runtime/database.rs), [src/app/runtime/request_lifecycle.rs](../../src/app/runtime/request_lifecycle.rs), [src/app/action/action_system.rs](../../src/app/action/action_system.rs).

### 5. Query Result Display

- Goal: show the correct active-tab result or error state, and never revive stale rows/messages/time from another tab or previous request.
- Entry: [handle_query_done()](../../src/app/runtime/handler.rs), [sync_from_active_tab()](../../src/app/runtime/request_lifecycle.rs), [classify_workspace_surface()](../../src/app/surfaces/render.rs), [render_sql_editor_in_ui()](../../src/app/surfaces/render.rs).
- Authority: currently split across `tab.result`, `tab.last_error`, `tab.last_message`, `tab.query_time_ms` and the app-level mirrors `self.result`, `self.last_query_time_ms`.
- Invariants: active render surface must be derived from the active tab; non-cancel query failure must surface as explicit error; new dispatch must clear stale tab-local result/time/message.
- Known risks: `self.result` remains a render mirror rather than a true authority; `self.sql` remains dual-source; explicit user-cancel UI is still absent even though runtime semantics are now split cleanly.
- Related files: [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs), [src/app/runtime/request_lifecycle.rs](../../src/app/runtime/request_lifecycle.rs), [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs), [src/ui/components/query_tabs.rs](../../src/ui/components/query_tabs.rs), [docs/recovery/02-query-execution-trace.md](./02-query-execution-trace.md).

### 6. Grid Edit & Save

- Goal: keep table edits isolated per workspace and refresh back into the same table view after a successful save.
- Entry: grid keyboard/actions in [src/ui/components/grid](../../src/ui/components/grid), [execute_grid_save()](../../src/app/runtime/database.rs), [handle_grid_save_done()](../../src/app/runtime/handler.rs), [refresh_table_after_grid_save()](../../src/app/runtime/database.rs).
- Authority: `grid_state`, `grid_workspaces`, `pending_grid_save_requests`, `pending_grid_refresh_restores`, `selected_table`.
- Invariants: edits remain until save success; save batches all SQL statements; success clears edits and restores the same workspace context; failure must not silently discard edits.
- Known risks: table context is spread across `selected_table`, active grid workspace, and save context; this is one of the highest-risk business flows according to test coverage and prior changelog notes.
- Related files: [src/ui/components/grid/mod.rs](../../src/ui/components/grid/mod.rs), [src/ui/components/grid/state.rs](../../src/ui/components/grid/state.rs), [src/app/mod.rs](../../src/app/mod.rs), [src/app/runtime/database.rs](../../src/app/runtime/database.rs), [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs), [tests/grid_tests.rs](../../tests/grid_tests.rs).

### 7. Destructive Actions

- Goal: destructive operations must execute only after explicit target-specific confirmation and must preserve connection context.
- Entry: `pending_delete_target` is set from the UI, [render_dialogs()](../../src/app/surfaces/dialogs.rs) opens confirmation, [confirm_pending_delete()](../../src/app/surfaces/dialogs.rs) dispatches to [delete_connection()](../../src/app/runtime/database.rs), [delete_database()](../../src/app/runtime/database.rs), [delete_table()](../../src/app/runtime/database.rs).
- Authority: `pending_delete_target`, `show_delete_confirm`, `pending_drop_requests`.
- Invariants: connection/database/table are different destructive targets; database/table deletion must use stored connection context, not current sidebar focus; active-state cleanup must happen if the removed object was currently active.
- Known risks: free-form SQL `DROP TABLE` is a parallel destructive path tracked through query handling; dialog host compatibility still relies on multiple booleans plus target payload. Current sidebar recovery work has re-aligned connection-row context menu entries, connection header delete buttons, and keyboard `d` so they all build the same `SidebarDeleteTarget` payload before entering `DeleteConfirm`; the latest fix also split the connection header’s label/toggle interaction surface away from the right-side action buttons so mouse destructive entrypoints no longer sit under the same parent click surface.
- Related files: [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs), [src/app/surfaces/dialogs.rs](../../src/app/surfaces/dialogs.rs), [src/app/runtime/database.rs](../../src/app/runtime/database.rs), [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs), [src/ui/panels/sidebar](../../src/ui/panels/sidebar).

### 8. Dialog Open / Close / Confirm / Cancel

- Goal: exactly one dialog owns input in a frame, content scrolls internally, and dialog shells stay inside the viewport.
- Entry: [dialog_host_snapshot()](../../src/app/dialogs/host.rs), [render_dialogs()](../../src/app/surfaces/dialogs.rs), [handle_dialog_results()](../../src/app/surfaces/dialogs.rs), input routing in [src/app/input/input_router.rs](../../src/app/input/input_router.rs).
- Authority: `active_dialog_owner` is now the primary runtime owner; `show_*` booleans, `*_dialog_state.show`, `command_palette_state.open`, and `DialogHostSnapshot` remain compatibility visibility layers.
- Invariants: `active_dialog()` chooses a single modal owner; `Esc`/confirm routes through the active scope; shell-level width/height constraints must be set once; overflow should be owned by inner `ScrollArea`s.
- Known risks: 多处固定宽度横排行在窄视口下仍可能横向挤压；长表单 live regression 仍未补齐；ER workstream 仍未进入状态权威源收口。
- Related files: [src/app/dialogs/host.rs](../../src/app/dialogs/host.rs), [src/app/surfaces/dialogs.rs](../../src/app/surfaces/dialogs.rs), [src/ui/dialogs/common.rs](../../src/ui/dialogs/common.rs), [src/ui/dialogs/picker_shell.rs](../../src/ui/dialogs/picker_shell.rs), [src/ui/dialogs/help_dialog.rs](../../src/ui/dialogs/help_dialog.rs), [src/ui/dialogs/keybindings_dialog.rs](../../src/ui/dialogs/keybindings_dialog.rs), [tests/ui_dialogs_tests.rs](../../tests/ui_dialogs_tests.rs).

### 9. ER Diagram Open & Interaction

- Goal: open ER from the current connection context, load table/relationship data, and provide pan/zoom/select/refresh interactions that look and feel like Gridix.
- Entry: [AppAction::ToggleErDiagram](../../src/app/action/action_system.rs), [AppAction::FocusErDiagram](../../src/app/action/action_system.rs), [set_er_diagram_visible()](../../src/app/input/input_router.rs), [load_er_diagram_data()](../../src/app/runtime/er_diagram.rs), [ERDiagramState::show()](../../src/ui/components/er_diagram/render.rs).
- Authority: `show_er_diagram` controls visibility; `er_diagram_state` holds loaded tables, relationships, loading, zoom, pan, selection, and drag state.
- Invariants: opening ER triggers data load only when there is an active connection and now also enters `FocusArea::ErDiagram`; explicit focus still enters `FocusArea::ErDiagram` without toggling visibility or reloading; state is cleared before reloading; all table columns and relationships eventually reconcile into a stable layout.
- Known risks: ER 现已具备第一阶段 TUI 式本地导航语义，返回历史会恢复最近一个合法的非 ER workspace 区域；最新收口后，bare `h / l` 已不再承担返回 / 打开，而是留在 ER 内部作为左右几何邻接，`Enter / Right` 继续打开当前表，`Left / Esc` 继续返回主工作区。本轮又把键盘流拆成 `er_diagram` 浏览子作用域与 `er_diagram.viewport` 视口子作用域，`v` 只负责在两者间切换，`Esc` 在视口模式内只退回浏览模式，不再越级离开 ER。视口模式对 `q` 关闭、`r / Shift+L / f` 组合、reload 后回到浏览态、跨主题渲染不改 ER 局部状态，以及 `FocusErDiagram -> ToggleErDiagram` 在视口模式下的 focus restore / fallback 分支都已有 focused coverage；`Ctrl+R -> ToggleErDiagram` 在 ER 导航态/视口态下的 focused coverage也已补齐，因此 ER 自身持有 focus 时不再把顶层 toggle 卡死，而 ER 从隐藏切到可见时也会直接拿到 focus，不再把 `h/j/k/l` 留给 `DataGrid`。`Ctrl+H` 这类其他 workspace overlay 仍不会因为进入 ER scope 被顺手放开。本轮又把 `Shift+Left/Down/Up/Right` 作为 additive 的几何邻接导航落地：它只按当前表卡几何位置在 ER 局部选择最近邻，不替换 `j/k` 线性浏览或 `Shift+J/K` 关系邻接，也不反向驱动主业务 `selected_table`。关系优先默认布局现在还会先按断开的关系簇和孤立表拆成独立组件，再分别做层级种子和全局 refine，因此“无关系的几簇被压成一锅粥”的剩余结构性风险已进一步下降。结合当前 `OpenSelectedTable -> QuerySelectedTable -> DataGrid` 主链，ER 继续保持 companion pane 定位，独立 `detail mode` 当前明确不进入实现。主要剩余风险已收窄到几何邻接与关系布局是否还需要进一步的 live 启发式调优。
- Related files: [src/app/runtime/er_diagram.rs](../../src/app/runtime/er_diagram.rs), [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs), [src/ui/components/er_diagram/state.rs](../../src/ui/components/er_diagram/state.rs), [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs), [src/ui/components/er_diagram/layout.rs](../../src/ui/components/er_diagram/layout.rs).

## E. Recovery Roadmap

### Phase 1: Establish Control, No Business Changes

- Keep the control set stable around `02 / 10 / 11 / 12 / 20 / 24`; merge short-lived fix notes back into these ledgers once the implementation lands.
- Produce missing flow traces for startup/init, connection lifecycle, grid save, destructive actions, dialogs, and ER.
- Freeze dependency baseline in recovery docs: `egui/eframe/egui_extras 0.34.1` and custom grid/ER ownership.
- Build a smoke matrix from existing tests: `database_tests`, `grid_tests`, `ui_dialogs_tests`, `edge_regression_tests`, `mysql_cancel_integration`.

### Phase 2: Validate High-Risk Root Causes

- Dialog horizontal overflow hotspots: the high-frequency slices (`theme chooser`, `ConnectionDialog`, `ImportDialog`, `DdlDialog`) have already landed, and the low-frequency long-form seeds now also each have at least one desktop live recheck. `CreateUserDialog` is no longer blocked on missing non-SQLite credentials after a temporary rootless MariaDB seed was established for verification. The shared row contract remains tracked in [43-dialog-responsive-row-design.md](./43-dialog-responsive-row-design.md).
- Grid save context integrity: keep verifying `selected_table` / `grid_state` / `pending_grid_save_requests` / `grid_workspaces` across save, failure, and tab switch.
- ER language/state split: audit `show_er_diagram`, `er_diagram_state`, direct visibility writes, keyboard ownership, hardcoded `RenderColors`, and toolbar affordances against Gridix theme tokens; detailed audit entry is now [44-er-ownership-and-design-audit.md](./44-er-ownership-and-design-audit.md), and the pre-implementation contracts are tracked in [47-er-workspace-and-keyboard-contract.md](./47-er-workspace-and-keyboard-contract.md), [48-er-visibility-entry-matrix-and-state-ledger.md](./48-er-visibility-entry-matrix-and-state-ledger.md), [49-er-keyboard-flow-graph.md](./49-er-keyboard-flow-graph.md), and [50-er-token-map.md](./50-er-token-map.md).

Current execution order before further implementation:

1. `CreateDbDialog` 已在 live 视口下复现 footer 不可见，并已完成最小修复；当前 Wayland 会话下的等价 live 复核已通过，但精确 `960x620` 点仍受窗口管理器高度钳制影响。
2. `ExportDialog` 的稳定结果集 seed 已建立：`OpenLearningSample -> WelcomeSetup -> Enter/5`。
3. `ExportDialog` 已完成第一轮 live：`960x620` 到 `760x560` 未再现 footer 漂移；此前更小视口触发的 render 层崩溃 `G41-B009` 已单独收口。
4. `DdlDialog` 已完成 live 桌面复核：在当前 Wayland 会话下沿 `OpenLearningSample -> WelcomeSetup -> 5 -> Ctrl+Shift+N` 打开“创建表”，约 `760x560` 级窄视口中 footer 重新保持可见；修复点已收敛到 DDL 自身的紧凑窗口 profile 与更保守的列区 / SQL 预览高度。
5. `CreateUserDialog` 已完成第一轮 non-SQLite live：本轮先在用户态初始化临时 MariaDB 实例（`127.0.0.1:33306`），再以临时 `XDG_CONFIG_HOME` 写入一条只供本轮验证使用的 MySQL 连接；沿 `Ctrl+B -> Ctrl+1 -> Enter -> Ctrl+Shift+U` 成功拉起 `CreateUserDialog`。在约 `900x650` 的浮动窄窗下，footer 仍保持可见，主体滚动与局部 SQL 预览滚动没有新冲突，`q` 也可正常关闭 dialog。
6. dialog overflow 主线已不再被 `CreateUserDialog` 的环境前提阻塞；后续除非出现新的 live 回归，否则这条主线转入 observation，不再继续空转。
7. ER 主线的 5 份前置材料已完成：workspace role、visibility entry matrix、state ledger、keyboard flow、token map。
8. ER 显隐入口首刀已落地：运行期显隐路径统一经过 `set_er_diagram_visible_with_notice(...)`。
9. ER lifecycle / merge 第二刀已落地：`loading` 现在等待 FK 与所有表列请求都完成，FK 徽标不再受回包顺序污染。
10. ER finalize 第三刀已落地：layout、空关系推断与最终 ready 提示现在统一在 ready 阶段决定，不再由 FK 回包抢先发提示。
10. ER keyboard owner 第四刀已落地：`FocusArea::ErDiagram` 已进入主循环，点击 ER 会显式入焦，hover 不再拥有键盘。
11. ER local navigation 第五刀已落地：`j/k` 线性选表，`Enter/Right` 打开当前表并回到 `DataGrid`，`h/Left/Esc` 返回主工作区，`q` 关闭 ER。
12. ER token 第六刀已落地：两波 token 现在都已回落到 `ThemePreset::colors()` + `egui::Visuals` 的派生规则。
13. ER 返回历史第七刀已落地：`Esc / h / Left` 与“关闭且当前焦点在 ER”现在都会恢复最近一个合法的非 ER workspace 区域；若目标不可用则回退到 `DataGrid`。
14. ER `l` contract 第十刀已落地：bare `l` 现在打开当前表，`Shift+L` 接管 relayout。
15. 顶层回归覆盖已补上第四批窄锚点：`ToggleErDiagram` 命令注册/可用性、ER 聚焦状态栏、Help->Relationships 学习入口、close -> focus restore helper、open/refresh shared load plan，以及 `ERDiagramResponse -> app-level surface dispatch`；下一步继续收窄到 open/refresh/theme switch/focus restore 的端到端 UI 组合链路。
16. ER 显式聚焦第十六刀已落地：新增 `FocusErDiagram` / `focus_er_diagram`，默认 `Alt+R`；它只在 ER 已打开时可用，只切入 `FocusArea::ErDiagram`，不改变显隐，也不触发 reload。
17. ER 视口模式第十七刀已落地：新增局部 `interaction_mode`，把 ER 键盘流拆成 `er_diagram` 浏览子作用域与 `er_diagram.viewport` 视口子作用域；`v` 在两者间切换，浏览态继续承载 `j/k` 线性浏览、`h/l` 左右几何邻接、`Enter/Right` 打开当前表与 `q` 关闭语义，视口态把 `h/j/k/l` 收回为平移，`Esc` 只退出视口模式回到浏览态，`q` 仍可直接关闭 ER。
18. ER 视口模式组合锚点第十八刀已落地：focused tests 现已锁住视口模式内 `q` 仍关闭 ER、`r / Shift+L / f` 仍保持可用，以及 `begin_loading()/reload` 会把 `interaction_mode` 保守重置回 `Navigation`。
19. ER 主题切换组合锚点第十九刀已落地：focused test 现已锁住跨 `ThemePreset` 渲染不会改写 ER 当前的 `interaction_mode / selected_table / pan_offset / zoom`；主题切换当前只应影响视觉 token，不得干扰局部浏览/视口状态。
20. ER focus-restore combo anchor 第二十刀已落地：app-level focused tests 现已锁住 `FocusErDiagram -> ToggleErDiagram` 从视口模式关闭时，会恢复最近一个合法的非 ER workspace 区域；若跟踪目标已不可用，则回退到 `DataGrid`，且隐藏后不会残留 ER input scope。
21. ER relation-adjacency 第二十一刀已落地：浏览态新增 `Shift+J / Shift+K`，会按稳定全局表顺序在当前表的关联集合内前进/后退；它只增强关系浏览，不替换原有 `j/k` 的线性选表。
22. `UX / Input Recovery Batch A` 已完成阶段性收口：toolbar trigger 独立快捷键/tooltip、toolbar 与 ER toolbar 的交互态 chrome、sidebar connection-row destructive entrypoint recovery、`Ctrl+R` 在 ER scope 内的 toggle gate、About 回摆、以及 `KeyBindingsDialog` / `HelpDialog` 顶部 header compression 均已落地；后续不再把这批条目作为 active implementation workstream，而是转入 live smoke / regression observation。
23. ER geometry-adjacency 第二十三刀已落地：浏览态新增 `Shift+Left / Shift+Down / Shift+Up / Shift+Right`，会按当前表卡中心点和方向锥体优先选择同方向最近邻；若没有轴向候选，则才保守回退到同方向对角候选。它是 additive 的局部浏览命令，不替换 `j/k` 线性或 `Shift+J / Shift+K` 关系邻接，也不直接同步主业务当前表。
24. ER relation-first default layout 第二十四刀已落地：加载中的 ER 仍先显示稳定 grid skeleton，但 `finalize_er_diagram_load_if_ready()` 现在会在关系 ready 后统一决定默认完成态布局；空关系图继续保持 grid，而存在显式或推断关系时会自动走关系层级种子 + force-directed refine。手动 `Shift+L` 现在也复用同一条关系优先路径，不再只做纯 force pass。
25. ER relation-seeded sibling ordering 第二十五刀已落地：`hierarchical_layout()` 的同层兄弟表不再继续依赖原始输入顺序，而是先按名称做稳定初始化，再按已知关系邻居的重心做轻量上下 sweep；因此关系优先布局现在不仅“有层级”，同层横向顺序也更贴近引用关系。
26. ER viewport toggle de-dup 第二十六刀已落地：`v` 的视口模式切换现在只通过 input router 生效，render 不再重复消费同一个局部快捷键；因此 ER 聚焦时 `v` 不会再出现同帧切了又切回去的假失效。
27. ER overlap-aware relation layout 第二十七刀已落地：`hierarchical_layout()` 现在会按真实表卡尺寸计算同层横向间距和层间纵向间距，而 `force_directed_layout()` 也改为按表卡中心与实际尺寸做分离；关系优先完成态不再继续只依赖固定 `180x200` 骨架和左上角点的斥力近似。
28. ER component-aware relation seeding 第二十八刀已落地：`relationship_seeded_layout()` 现在会先把断开的关系簇和孤立表拆成独立组件，再分别做层级种子并按组件边界留白后进入全局 force-directed refine；默认完成态和手动 `Shift+L` 不再把彼此完全无关的簇压进同一条横向种子带。
29. ER component row-packing 第二十九刀已落地：多组件关系种子现在会按组件面积估计目标行宽，并在种子阶段对断开的组件换排；默认完成态和手动 `Shift+L` 不再把所有无关组件只沿单一横带持续向右展开。
30. ER component priority anchoring 第三十刀已落地：多组件关系种子现在会按组件面积/宽度优先排序，再进入按行换排；较大的关系主簇会先占据左上锚点，小型孤立表不再仅因名字更靠前就把主簇挤到后排。
31. ER open-fit visibility 第三十一刀已落地：从隐藏态打开 ER 现在会请求一次延迟 `fit_to_view()`；如果默认完成态在旧 `25%` 下限下仍不足以容纳整图，ER 会把 `zoom` 进一步降到更低，直到主簇重新完整落回可见视口。当前这条自动 fit 只在 open 分支执行，不会把 reload/refresh 的“保留当前视图”合同一起洗掉。
32. ER generation redesign 第一阶段已落地：`finalize_er_diagram_load_if_ready()` 现在不再直接把 `tables + relationships` 喂给单一布局函数，而是先经 `analyze_er_graph()` 生成语义摘要，再按 `Grid / Relation / Component` 三种完成态策略选择布局路径；ER render 侧也补上了 `All / Focus / ExplicitOnly` 边显示模式、`Standard / KeysOnly` 表卡密度模式、关系邻域降噪与正交式边路由，因此当前主线已从“继续局部调布局参数”升级为显式语义图 + 策略布局 + 视图密度控制的第一阶段架构。

### Phase 2.5: UX / Input Recovery Batch A Closure

- 范围: `toolbar / sidebar connection-row / ER top-level input affordances / AboutDialog / workspace dialog headers`。
- 已收口条目:
  - toolbar `"⚡" / "+"` trigger 现在有独立入口、真实 tooltip，并恢复到“默认透明，仅在 hover / focus / selected 时显示 chrome”的交互态。
  - `ToolbarMenuDialog` 现已切到 `DialogWindow::workspace(...)`，支持 `Esc / Q`，顶部双区块也已压缩成紧凑 header。
  - sidebar connection-row 的右键菜单、`删连 / 删库` 按钮和 keyboard `d` 现已重新收敛到同一 `SidebarDeleteTarget -> pending_delete_target -> DeleteConfirm -> confirm_pending_delete()` destructive flow。
  - ER 顶层 `Ctrl+R` toggle、`Alt+R` focus、selection-visible、viewport mode、focus restore 与 relation-adjacency 第一刀均已落地，当前不再把“ER 自己持有 focus 时 `Ctrl+R` 失效”视为 open regression。
  - `AboutDialog` 已从厚重 section 堆叠回摆到更轻的品牌页结构；`KeyBindingsDialog` / `HelpDialog` 顶部连续双 toolbar 也已收敛到共享 header helper。
- 剩余风险:
  - 仍缺真正的 egui/widget 级 connection-row 右键弹层 live regression；当前 focused coverage 已锁住 target 构造与 app-level confirm chain，但“右键菜单稳定弹出”仍主要靠结构证据。
  - About 与 workspace dialog header compression 目前主要由结构测试和手动观感判断支撑，尚无截图级回归锚点。
  - `KeyBindingsDialog` 里显示用的 `dialog.toolbar_menu / dialog.toolbar_theme` taxonomy 已与 owner 一致，但运行时命令 id 仍保持 `toolbar.menu.* / toolbar.theme.*`；这是刻意保留的分层，不应再误判为冲突。
- 下一阶段入口:
  - 先做一轮 live UI smoke，重点覆盖 toolbar trigger hover/focus、connection-row 右键/删连、About 首屏观感、`KeyBindingsDialog` / `HelpDialog` 顶部信息密度。
  - 只有在 live smoke 重新复现时，才把这批条目重新升回 active bug；否则主线返回 `G41-B007` 的剩余 dialog live verification，以及 ER 是否需要独立 detail mode 的后续设计。
- 第一轮 live smoke（Wayland / Hyprland / `DP-1`）结果:
- `Alt+A`、`Alt+N`、`Alt+K`、`F1` 与 `Ctrl+P -> about` 的显式打开路径均通过；对应 chooser / workspace dialog / about 均能稳定打开并用 `q` 关闭。
- toolbar 默认态已不再出现常驻灰底；`KeyBindingsDialog` 与 `HelpDialog` 顶部 header 也已在默认 workspace dialog 宽度下以内联双区块展示。
- `AboutDialog` 当前 live 观感与自动化结构回归一致：已回摆成更轻的品牌页，不再是厚重的双卡片堆叠。
- `Ctrl+R` 的自动注入 smoke 目前仍不构成可信证据：`Alt+A -> 切换ER图` 可以正常打开 ER，命令面板链 `Ctrl+P -> toggle_er_diagram -> Enter` 现也已有 app-level focused regression 覆盖，说明 action / owner / render 主链仍通；但 `wtype` 与 `hyprctl dispatch sendshortcut` 在当前 grid 场景下都会落成 bare `r` 或不稳定的 palette 选择行为，因此这一步仍需实体键手动复核，暂不据此重新打开 ER bug。
- ER 几何邻接已完成首轮 Wayland live 观察：沿 `Ctrl+P -> sample -> Enter -> Ctrl+R -> Alt+R -> Shift+Right/Down/Left/Up` 进入学习样例 ER 后，方向跳转与 `selection follows viewport` 目前未复现明显误跳；当前不继续对算法下补丁，先维持“观察 / 启发式调优候选”状态。
- ER 几何邻接第二轮组合观察也已通过：在同一学习样例链路下继续执行 `Shift+L -> Shift+Down -> f -> Ctrl+Shift+T -> j -> Enter`，当前未复现 relayout / fit view / theme switch 之后的几何误跳；主题切换后 ER 的 `show_er_diagram`、缩放级别与当前布局均保持稳定。继续用 `Shift+Arrow` 浏览前，如需恢复 ER 键盘 owner，可显式再按一次 `Alt+R`，当前这更像既有显式聚焦模型，而不是新的 active bug。
- ER 关系优先布局第三轮 live 观察已完成：在学习样例默认完成态下，当前关系种子 + 同层重心排序未再出现明显“同层兄弟表直接继承输入顺序”的反例，主簇分组观感比纯 grid skeleton 更贴近关系结构。当前不继续猜测性调 `force_directed_layout()` 参数；若后续要继续观察 `Shift+L / f` 之后的 live 观感，应优先用实体键或显式回焦后的链路复核，而不是把自动注入导致的焦点漂移误记成布局 bug。
- ER 默认打开后的可见范围第四轮 live 观察已完成：在学习样例链路下重新执行 `Ctrl+P -> sample -> Enter -> 5 -> Ctrl+R` 后，ER 现在会在默认完成态自动降到约 `12%`，整簇重新完整落回右侧可见区，底部表不再继续被 `25%` 下限卡在视口外。当前不继续把这条问题维持为 active bug。
- 当前 ER 新收口的两个 confirmed bug 已从 active fix 转为 observation：一是 `v` 视口切换的 router/render 双消费；二是关系优先布局继续忽略真实表卡尺寸导致的大表重叠。focused tests 已锁住两条 contract，后续若仍有 live 重叠，再优先观察具体 schema，而不是先回退到旧 grid 骨架。
- 当前 ER 新增的第三条布局 contract 也已转入 observation：断开的关系簇与孤立表现在会先各自组件化种子，再进入全局 refine；若后续 live 仍出现“完全无关的两簇被压成一个主簇”的观感，再优先回看具体 schema 的组件边界，而不是先回退到单一全局层级种子。
- ER generation redesign 第二阶段第二刀已落地：当 ER reload 后的表集合与旧布局快照只部分重合时，当前会先跑一遍新的完成态策略布局，再把仍同名的旧表位置恢复回去；新增表继续使用这轮策略布局给出的新位置，并会在必要时对恢复回来的旧表做局部避让，因此同表集与“主体不变的小幅表集变化”现在都不再每次把整图洗掉。
- ER generation redesign 第二阶段第四刀已落地：当 partial reload 里的新增表本身和这些已恢复旧表存在关系时，当前不再只是“别撞上去”，而会优先贴近自己的关系邻居，再做最后一层局部避让；这样小幅 schema 漂移后的新表不再轻易被甩在旧簇很远的位置。
- ER generation redesign 第二阶段第五刀已落地：当 partial reload 里的新增表和已恢复旧表存在明确父/子方向关系时，当前不再只做“无方向地靠近邻居”，而会优先按关系方向选择上下插入位置；引用父表的新表会优先落在父表下方，被子表引用的新表则优先落在这些子表上方。
- ER generation redesign 第二阶段第六刀已落地：当 partial reload 里的新增表同时连到已恢复父表与子表时，当前不再在双向关系里简单退化成“落到父表下面”；局部插入现在会优先保留桥接表所在的上下层带，并在四向候选里按“总重叠最小、尽量留在原锚点垂直带内”的顺序选位，因此桥接表更稳定地落在已恢复父/子层之间，而不是继续被挤到整个子层下面。
- 阶段总复审结论：当前 recovery 主线里已经没有新的未阻塞 active implementation workstream；剩余条目都属于 `observation / live smoke / release closure`，或需要新的 confirmed bug 才值得重新开刀。
- release-closure review 进一步确认：当前若要继续推进主线，最合理的新 workstream 不是再修 recovery bug，而是单独开启一条 `lint / release gate closure` 小主线，专门处理 `cargo clippy --all-targets --all-features -- -D warnings` 目前报出的具体项。

### Phase 3: Minimal Repairs For Core Stability

- Fix the smallest confirmed core-flow issues first, one bug per patch.
- Priority order: remaining low-frequency dialog live verification only when credentials/seed exist, then ER 几何邻接的启发式观察与更高层浏览模型是否仍有必要的设计验证，再处理任何 newly confirmed grid/query regression.
- Every patch must come with:
  - one recovery note
  - one focused validation
  - one rollback path

### Phase 4: Unify UI Design Language

- Move dialogs toward a single shell/content discipline based on current `egui 0.34.1` `Window` + `ScrollArea` behavior.
- Reduce deep `ui.set_min_width/set_max_width` forcing in dialog content; keep width rules at the shell or pane-split layer.
- Preserve ER-specific canvas semantics while keeping them derived from Gridix theme primitives, and then resolve the remaining interaction-level issues (`l` conflict, return-history ownership).
- Align ER toolbar chrome, spacing, corner radii, empty/loading states, and keyboard hints with toolbar/dialog/grid conventions.

### Phase 5: Regression Hardening & Documentation

- Expand targeted tests where coverage is currently weak:
  - dialog overflow and pane sizing
  - ER interaction/state restore
  - destructive action context preservation
  - grid save end-to-end behavior
- Update `docs/CHANGELOG.md` only for user-visible behavior changes.
- Keep `docs/recovery/` as the source of truth for evidence and risk, not just patch notes.

## F. Current Priority Entry

当前剩余优先级不再在这份主计划里重复维护完整排行。  
以最新排序为准，请直接查看：

- [12-bug-ledger-4.1.0.md](./12-bug-ledger-4.1.0.md)

当前 active workstream 已不再停留在 `G41-B005` 的基础 contract 收口上。

当前 ER 侧剩余 follow-up 已收窄到：

- 几何邻接已落地后的继续观察与可能的启发式调优
- 是否还需要更高层浏览模型，而不是直接在 ER 内引入独立 `detail mode`
- ER generation redesign 第二阶段现已先收下“同表集 reload 保留布局”“表集变化时的交集保位”“新表对已恢复旧表的局部避让”“关系邻居驱动的新表局部插入”“父/子方向感知的新表插入”和“桥接表保持在已恢复父/子层之间”的六刀；剩余 follow-up 已收窄到 edge routing / minimap / pin/keep-layout UI / 更强的增量稳定布局，而不是再次回退到单函数启发式补丁

当前仍未完全关闭、但已不再主导主线的是：

- `G41-B007`：低频长表单 dialog 的后续回归观察（高频与低频切片现都至少完成一轮 live 验证）

当前 dialog 主线里不再存在环境 blocker；剩余的是观察项：

- `CreateUserDialog` / `ExportDialog` / `DdlDialog` 在不同窗口管理器与更小窄视口下是否会再次出现 footer 或横向挤压回归
- 当前若继续推进主线，应优先转入阶段收口、发布准备，或等待新的 confirmed bug，而不是继续对 observation 条目做猜测性补丁。
