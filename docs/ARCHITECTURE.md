# Architecture Overview | 架构总览

## 1. High-Level Design | 高层设计

Gridix is an `eframe/egui` desktop app with a keyboard-first workflow.  
Gridix 基于 `eframe/egui`，强调键盘优先的桌面交互。

Main layers:
- `app/`: app state, orchestration, keyboard routing, async message handling.
  `app/`：应用状态、编排逻辑、键盘路由、异步消息处理。
- `database/`: DB drivers and connection/query abstractions.
  `database/`：数据库驱动、连接与查询抽象。
- `ui/`: visual components (grid/editor/sidebar/dialogs/help/welcome).
  `ui/`：可视组件（表格、编辑器、侧边栏、对话框、帮助、欢迎页）。
- `core/`: shared modules (config, theme, autocomplete, history, syntax, keybindings).
  `core/`：共享核心模块（配置、主题、补全、历史、语法、快捷键）。

Recent `app/` split highlights:
`app/` 近期拆分重点：
- `runtime/request_lifecycle.rs`: request id generation, pending task tracking, cancel flow.
  `request_lifecycle.rs`：请求 ID 生成、任务跟踪、取消流程。
- `dialogs/host.rs`: active dialog ownership, modal priority, and single dialog input owner.
  `dialogs/host.rs`：当前对话框所有权、模态优先级和单一对话框输入归属。
- `input/owner.rs`: frame-level keyboard owner (`modal`, `text_entry`, `command`, `recording`, `disabled`).
  `input/owner.rs`：每帧键盘所有者（`modal`、`text_entry`、`command`、`recording`、`disabled`）。
- `core/commands.rs`: scoped command metadata registry used by local shortcut compatibility, keymap settings, and tooltips.
  `core/commands.rs`：作用域命令元数据注册表，供局部快捷键兼容层、键位设置与提示使用。
- `surfaces/preferences.rs`: UI/theme/config preference update flow.
  `preferences.rs`：UI/主题/配置偏好更新流程。
- `runtime/metadata.rs`: metadata load request pipeline and stale-response guard.
  `metadata.rs`：元数据加载请求链路与过期响应保护。

## 2. Runtime Model | 运行时模型

- UI thread runs egui frame updates.
  UI 主线程负责 egui 帧渲染。
- Async DB work runs on Tokio runtime.
  数据库异步任务运行在 Tokio runtime。
- Results are returned through `std::sync::mpsc` messages.
  结果通过 `std::sync::mpsc` 消息回传。

Message types are defined in:
- `src/app/runtime/message.rs`

## 3. Query Execution Flow | 查询执行流程

1. User triggers query from SQL editor or shortcut.
   用户在 SQL 编辑器或快捷键触发执行。
2. App allocates request id and dispatches async task.
   应用分配请求 ID 并派发异步任务。
3. DB driver executes query (`sqlite` / `postgres` / `mysql`).
   由对应驱动执行查询。
4. Task sends `Message::QueryDone(...)` back to app.
   任务回传 `Message::QueryDone(...)`。
5. UI applies result only if request id matches latest context.
   UI 仅在请求 ID 匹配最新上下文时应用结果（避免过期回包污染）。

## 4. Focus, Dialog Ownership & Keyboard Routing | 焦点、对话框所有权与键盘路由

- Frame input ownership is resolved before command dispatch.
  每帧会先解析输入所有者，再分发命令。
- Active dialog ownership is resolved from app-level `active_dialog_owner`; `src/app/dialogs/host.rs` keeps the legacy visibility snapshot and open/close helpers.
  当前对话框所有权由 app 级 `active_dialog_owner` 决定；`src/app/dialogs/host.rs` 负责遗留可见性快照与 open/close helper。
- Scoped routing lives in `src/app/input/input_router.rs` and `src/app/input/owner.rs`.
  作用域路由位于 `src/app/input/input_router.rs` 与 `src/app/input/owner.rs`。
- Scoped local command metadata lives in `src/core/commands.rs`.
  作用域局部命令元数据位于 `src/core/commands.rs`。
- Dialog-local command dispatch goes through `ui::DialogShortcutContext`, which now supports direct command ids while keeping `LocalShortcut` as a compatibility adapter.
  对话框局部命令分发经由 `ui::DialogShortcutContext`，现在支持直接 command id，同时保留 `LocalShortcut` 作为兼容适配层。
- Local area handling:
  - DataGrid: `src/ui/components/grid/keyboard.rs`
  - SQL editor: `src/ui/components/sql_editor.rs`
  - Query tabs/toolbar/sidebar each handle local navigation.

Design rule:
- Active modal/dialog wins before workspace commands.
  当前模态对话框优先于工作区命令。
- Text entry wins over normal command keys.
  文本输入优先于普通命令键。
- Global `Tab` focus cycle is blocked when SQL editor completion is active.
  当 SQL 编辑器补全激活时，全局 `Tab` 焦点循环会被阻止。

Current limitation:
- `ui::LocalShortcut` is now a compatibility enum backed by the scoped command registry rather than the source of local command metadata.
  `ui::LocalShortcut` 现在是由作用域命令注册表驱动的兼容枚举，不再是局部命令元数据的唯一来源。
- Most high-frequency local paths already route through scoped commands, but a few component-level edit semantics still live inside widget-local code instead of a single reducer layer.
  大多数高频局部路径已经走 scoped command，但少量组件级编辑语义仍留在 widget 局部代码里，而不是统一 reducer 层。
- Picker dialogs now use movable/resizable workspace windows; the remaining UX variability is pane-collapse policy rather than raw key routing.
  picker 对话框现在是可拖拽、可缩放的 workspace 窗口；当前剩余的 UX 差异主要在 pane 折叠策略，而不是 raw key 路由。

Related docs:
- [KEYBOARD_FOCUS_RFC.md](KEYBOARD_FOCUS_RFC.md)
- [KEYMAP_TOML_SPEC.md](KEYMAP_TOML_SPEC.md)
- [recovery/11-core-flows-and-invariants.md](recovery/11-core-flows-and-invariants.md)
- [recovery/20-dialog-layout-audit.md](recovery/20-dialog-layout-audit.md)

## 5. State & Persistence | 状态与持久化

Configuration type:
- `src/core/config.rs::AppConfig`

Persisted content includes:
- connections, theme mode/preset, ui scale
- query history and per-connection command history
- onboarding progress

Shortcut bindings are persisted separately in `keymap.toml`; `config.toml.keybindings` is now only a legacy migration source.
快捷键现在单独持久化在 `keymap.toml`；`config.toml.keybindings` 只保留为遗留迁移来源。

Grid table workspace state is cached by `GridWorkspaceStore` under `(tab_id, connection, database, table)`. This keeps drafts isolated across both tabs and tables.
表格工作区状态由 `GridWorkspaceStore` 按 `(tab_id, connection, database, table)` 缓存，当前已经能同时隔离不同标签页与不同表的草稿状态。

## 6. Themes & Visual System | 主题与视觉系统

- Theme presets are defined in `src/core/theme.rs`.
- Default dark theme: `TokyoNightStorm`
- Default light theme: `TokyoNightLight`

主题预设定义见 `src/core/theme.rs`，默认深色为 `TokyoNightStorm`。

## 7. Safety & Robustness Notes | 稳定性说明

- Request-id guards prevent stale async response overwrite.
  通过请求 ID 防止过期异步响应覆盖当前状态。
- Config writes use atomic temp-file rename.
  配置写入采用临时文件 + 原子重命名。
- Unix config permission is set to `0600`.
  Unix 平台配置权限限制为 `0600`。
