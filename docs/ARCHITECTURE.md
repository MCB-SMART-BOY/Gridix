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

## 2. Runtime Model | 运行时模型

- UI thread runs egui frame updates.
  UI 主线程负责 egui 帧渲染。
- Async DB work runs on Tokio runtime.
  数据库异步任务运行在 Tokio runtime。
- Results are returned through `std::sync::mpsc` messages.
  结果通过 `std::sync::mpsc` 消息回传。

Message types are defined in:
- `src/app/message.rs`

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

## 4. Focus & Keyboard Routing | 焦点与键盘路由

- Global shortcuts are handled in `src/app/keyboard.rs`.
  全局快捷键处理在 `src/app/keyboard.rs`。
- Local area handling:
  - DataGrid: `src/ui/components/grid/keyboard.rs`
  - SQL editor: `src/ui/components/sql_editor.rs`
  - Query tabs/toolbar/sidebar each handle local navigation.

Design rule:
- Global `Tab` focus cycle is blocked when SQL editor completion is active.
  当 SQL 编辑器补全激活时，全局 `Tab` 焦点循环会被阻止。

## 5. State & Persistence | 状态与持久化

Configuration type:
- `src/core/config.rs::AppConfig`

Persisted content includes:
- connections, theme mode/preset, ui scale
- keybindings
- query history and per-connection command history
- onboarding progress

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
