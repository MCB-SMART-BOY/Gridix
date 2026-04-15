# Keyboard Focus RFC | 焦点驱动键盘系统 RFC

This document defines the guiding input model for current and near-term Gridix.
本文定义 Gridix 当前与近期演进阶段的输入模型。

Status for `v5.0.0`: the input owner, dialog host, scope-aware keymap router, and most high-frequency local command paths are implemented. The remaining disputes are about local policy and state boundaries, not a global-first router.
`v5.0.0` 状态：输入所有者、对话框宿主、scope-aware keymap router，以及大多数高频局部命令路径都已经落地。当前剩余分歧主要是局部策略和状态边界，而不是 global-first 路由本身。

Implemented foundation:
已落地基础：
- app-level `active_dialog_owner` is authoritative; `src/app/dialogs/host.rs` keeps visibility snapshot and helper logic around it.
  app 级 `active_dialog_owner` 是权威来源；`src/app/dialogs/host.rs` 负责围绕它的可见性快照与辅助逻辑。
- `src/app/input/owner.rs` names frame-level keyboard owners: modal, text entry, select, command, recording, disabled.
  `src/app/input/owner.rs` 明确每帧键盘所有者：模态、文本输入、选择、命令、录制、禁用。
- `src/app/input/input_router.rs` consumes the active dialog owner before scoped keymap dispatch.
  `src/app/input/input_router.rs` 在作用域 keymap 分发前消费 active dialog owner。

## 1. Problem Statement | 问题定义

The global-first router problem is mostly addressed, but a few local semantics still need explicit policy:
全局优先的路由问题已经基本处理完，但少数局部语义仍需要继续明确策略：

- Grid edge behavior can accidentally look like panel traversal if left implicit in local handlers.
  如果继续把 grid 边界行为藏在局部 handler 里，就容易让它看起来像跨面板跳转。
- Editor-local execution/completion handling must not fall back to hard-coded keys outside the editor scope.
  编辑器局部的执行/补全处理不能再退回到 editor scope 之外的硬编码按键。
- The same key may still drift semantically if scope responsibility is not kept explicit.
  如果不继续保持作用域职责明确，同一个按键的语义仍可能再次漂移。

Current contentious areas:
当前仍有争议的区域：

- Grid edge transfer is now explicit, but whether `h / Left` at the first column should remain a sidebar transfer is still a product-policy decision, not a router bug.
  Grid 边界转移已经显式化，但“首列按 `h / Left` 是否应继续返回侧边栏”仍然是产品策略问题，而不是路由 bug。
- Sidebar traversal now treats `l` as “go deeper” instead of “go to next panel”; this is intentional, but it differs from older broad `h/l` interpretations.
  侧边栏当前把 `l` 定义为“进入更深层”，而不是“去下一个 panel”；这是有意设计，但与早期宽泛的 `h/l` 解释不同。
- Picker dialogs now use explicit workspace shells, and toolbar action/create choosers already moved to overlay dialogs. The remaining open question is the last raw popup/overlay cluster, not the global routing model.
  picker 对话框现在已经使用显式 workspace shell，toolbar 的 action/create 选择器也已迁到 overlay dialog。当前剩余问题是最后一批 raw popup/overlay，而不是全局路由模型。

## 2. Design Goals | 设计目标

- Focus decides scope before shortcuts are resolved.
  先判定焦点作用域，再解析快捷键。
- Text input always wins over normal command keys.
  文本输入永远优先于普通命令键。
- The same key can safely mean different things in different scopes.
  同一按键可以在不同作用域安全复用。
- Routing behavior must be explainable with one focus graph.
  输入路由必须能用一张焦点图解释清楚。
- Global shortcuts must be minimal and stable.
  全局快捷键必须极少且稳定。

## 3. Core Concepts | 核心概念

### 3.1 Focus Scope | 焦点作用域

The new router resolves keys against a scope path:
新的路由器按作用域路径解析按键：

- `global`
- `toolbar`
- `query_tabs`
- `sidebar.connections`
- `sidebar.databases`
- `sidebar.tables`
- `sidebar.filters.list`
- `sidebar.filters.input`
- `sidebar.triggers`
- `sidebar.routines`
- `grid.normal`
- `grid.select`
- `grid.insert`
- `editor.normal`
- `editor.insert`
- `dialog.*`

### 3.2 Input Mode | 输入模式

Each focus scope may expose one mode:
每个作用域可以暴露一个模式：

- `command`
- `text_entry`
- `select`
- `recording`
- `disabled`

Example:
例如：

- `sidebar.filters.list` -> `command`
- `sidebar.filters.input` -> `text_entry`
- `editor.insert` -> `text_entry`
- `keybindings_dialog.recording` -> `recording`

### 3.3 Input Router | 输入路由器

The router should evaluate input in this order:
输入路由器应按以下顺序判定：

1. Active modal/dialog scope
   当前模态对话框作用域
2. Active text-entry child scope
   当前文本输入子作用域
3. Focused region local scope
   当前焦点区域的局部作用域
4. Workspace-level fallback
   工作区级兜底动作
5. Minimal global actions
   极少数全局动作

This order is the core change.
这个顺序就是本次重构的核心。

## 4. Global Shortcut Policy | 全局快捷键策略

Allowed global actions:
允许保留为全局动作的操作：

- `F1` help
- `Ctrl+N` new connection
- zoom actions
- optional command palette

`next_focus_area` / `prev_focus_area` are workspace-level fallback actions.
`Tab / Shift+Tab` are only their default bindings and must not be treated as unconditional
global-first keys.
`next_focus_area` / `prev_focus_area` 是工作区级兜底动作。
`Tab / Shift+Tab` 只是它们的默认绑定，不能再被实现为无条件的 global-first 按键。

Everything else should belong to a scope.
其余动作都应属于某个作用域。

Explicit non-global examples:
明确不应全局化的动作：

- `i` / `a` / `o`
- `j` / `k` / `h` / `l`
- `f`, `d`, `x`, `space`
- filter add/remove/toggle logic
- editor execution fallback keys when another text widget is focused

## 5. Scope Responsibilities | 作用域职责

### 5.1 Sidebar scopes | 侧边栏作用域

- `connections/databases/tables/triggers/routines`
  - list navigation
  - activation
  - panel-to-panel transfer
- `filters.list`
  - filter rule navigation
  - add/delete/reorder/toggle rule
  - switch operator/column/logic
- `filters.input`
  - pure text input
  - `Esc` back to `filters.list`

### 5.2 Grid scopes | 表格作用域

- `grid.normal`
  - Helix-style movement and commands
- `grid.select`
  - selection extension
- `grid.insert`
  - cell editing only

### 5.3 Editor scopes | 编辑器作用域

- `editor.normal`
  - movement, mode switch, history, execute/explain
- `editor.insert`
  - text editing
  - autocomplete acceptance/navigation
  - `Esc` exits completion first, then mode
  - execute/explain keys only work while the editor owns input

## 6. State Model Changes | 状态模型改造

Current state:
当前状态：

- `FocusArea`
- `sidebar_section`
- booleans such as `focus_sql_editor`, `show_autocomplete`

Target additions:
目标新增状态：

- `FocusScopePath`
- `InputMode`
- `InputOwner`
- `TextEntryGuard`
- `PendingFocusTransition`

This should replace scattered booleans over time.
这套状态应逐步替代分散布尔值。

## 7. Open Controversies | 当前仍有争议的代码边界
- `src/core/commands.rs` + `ui::LocalShortcut`: the scoped command registry is already authoritative, but `LocalShortcut` still exists as a compatibility adapter. New code should continue to prefer command ids directly.
  `src/core/commands.rs` + `ui::LocalShortcut`：当前权威来源已经是 scoped command registry，但 `LocalShortcut` 仍作为兼容适配层存在。新代码仍应优先直接使用 command id。
- `src/ui/panels/sidebar/connection_list.rs` and `src/ui/panels/sidebar/database_list.rs`: delete connection / delete database / delete table are now separated by explicit targets, but they still enter from multiple UI paths and therefore need focused regression coverage.
  `src/ui/panels/sidebar/connection_list.rs` 与 `src/ui/panels/sidebar/database_list.rs`：删连接 / 删库 / 删表现在已经由显式目标区分，但仍有多个 UI 入口，因此必须保持针对性的回归覆盖。

## 8. Acceptance Criteria | 验收标准

- Typing in any text field never triggers non-text commands.
  在任何文本输入框中输入文字，都不会触发非文本命令。
- `Tab` never steals focus from editor completion or filter text input.
  `Tab` 不会从编辑器补全或筛选文本输入中抢走焦点。
- Sidebar panel traversal can be drawn and explained as one stable graph.
  侧边栏面板切换可以画成一张稳定的焦点图。
- The same key binding can exist in multiple scopes without ambiguity.
  同一键位可以在多个作用域并存且无歧义。

## 9. Follow-up Docs | 后续文档

- [KEYMAP_TOML_SPEC.md](KEYMAP_TOML_SPEC.md)
- [recovery/11-core-flows-and-invariants.md](recovery/11-core-flows-and-invariants.md)
- [recovery/20-dialog-layout-audit.md](recovery/20-dialog-layout-audit.md)
