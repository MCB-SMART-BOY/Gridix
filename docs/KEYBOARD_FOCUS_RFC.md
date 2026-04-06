# Keyboard Focus RFC | 焦点驱动键盘系统 RFC

This document defines the target input model for Gridix.  
本文定义 Gridix 未来的目标输入模型。

## 1. Problem Statement | 问题定义

Current keyboard handling is still largely global-first:
当前键盘处理仍然基本是“全局优先”：

- Global actions are checked in `src/app/keyboard.rs`.
  全局动作在 `src/app/keyboard.rs` 中优先判定。
- Local navigation is split across sidebar, grid, editor, and dialog modules.
  局部导航又分散在侧边栏、表格、编辑器、对话框各模块。
- The same key may be interpreted differently by multiple layers.
  同一个按键可能被多个层同时解释。

Observed failures:
已观察到的问题：

- Typing `i` in filter input may trigger focus or mode changes elsewhere.
  在筛选输入框里输入 `i` 时，可能触发其他区域的焦点或模式切换。
- Sidebar navigation is incomplete and inconsistent across panels.
  侧边栏导航不完整，不同面板间语义不一致。
- `Tab`, `F5`, `h/j/k/l`, `gg/G` do not share one routing rule.
  `Tab`、`F5`、`h/j/k/l`、`gg/G` 没有统一的输入路由规则。

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
- `Tab / Shift+Tab` major area switch
- zoom actions
- optional command palette

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

## 7. Migration Plan | 迁移方案

### Phase 1: Introduce router skeleton | 第一阶段：建立路由骨架

- Add `InputRouter` module in `app/`.
- Route `F1`, `Ctrl+N`, `Tab`, zoom only.
- Keep old handlers behind router dispatch.

### Phase 2: Sidebar and filter routing | 第二阶段：侧边栏与筛选

- Move sidebar key handling behind scope-aware dispatch.
- Split `sidebar.filters.list` and `sidebar.filters.input`.
- Remove filter-related global actions from app-level keyboard handler.

### Phase 3: Grid/editor convergence | 第三阶段：表格与编辑器收敛

- Normalize editor and grid mode transitions.
- Make edge focus transfer explicit, not hidden in local handlers.

### Phase 4: Dialog unification | 第四阶段：对话框统一

- Reuse the same scoped router in dialogs.
- Remove duplicated `j/k/h/l` parsing from popup windows.

## 8. Acceptance Criteria | 验收标准

- Typing in any text field never triggers non-text commands.
  在任何文本输入框中输入文字，都不会触发非文本命令。
- `Tab` never steals focus from editor completion or filter text input.
  `Tab` 不会从编辑器补全或筛选文本输入中抢走焦点。
- Sidebar panel traversal can be drawn and explained as one stable graph.
  侧边栏面板切换可以画成一张稳定的焦点图。
- The same key binding can exist in multiple scopes without ambiguity.
  同一键位可以在多个作用域并存且无歧义。

## 9. Risks | 风险

- Partial migration will temporarily increase complexity.
  半迁移阶段会短期增加复杂度。
- Existing implicit focus behavior may disappear and expose hidden dependencies.
  现有隐式焦点行为消失后，可能暴露隐藏耦合。
- Tests must be added before major router rewiring.
  必须先补测试，再重排输入路由。

## 10. Follow-up Docs | 后续文档

- [KEYMAP_TOML_SPEC.md](KEYMAP_TOML_SPEC.md)
- [SIDEBAR_WORKFLOW_PLAN.md](SIDEBAR_WORKFLOW_PLAN.md)

