# Gridix Keyboard Guide | 键盘指南

> Applies to the `v4.0.x` scoped keymap model.
> 适用于 `v4.0.x` 作用域化键位模型。
> Some keys are user-configurable in keybinding settings.
> 部分快捷键可在“快捷键设置”中自定义。
>
> Actual runtime bindings come from `~/.config/gridix/keymap.toml`.
> 运行时实际生效的键位以 `~/.config/gridix/keymap.toml` 为准。
> If you changed them, prefer the in-app hover tooltip and Help panel.
> 如果你改过键位，请优先以应用内悬停提示和帮助面板为准。

## 1. Focus First | 先看焦点
- `next_focus_area` / `prev_focus_area` (default: `Tab` / `Shift+Tab`): cycle major focus areas (Sidebar -> DataGrid -> SQL Editor).
  `next_focus_area` / `prev_focus_area`（默认：`Tab` / `Shift+Tab`）：循环切换主区域焦点（侧边栏 -> 数据表格 -> SQL 编辑器）。
- `hjkl`: navigate inside current area.
  `hjkl`：在当前焦点区域内导航。
- Same key can mean different actions in different areas.
  同一按键在不同区域语义可能不同。

Example / 示例：`F5`
- In SQL Editor: execute SQL.
  在 SQL 编辑器：执行 SQL。
- Outside SQL Editor: refresh connection/table state.
  在编辑器外：刷新连接/表状态。

## 2. Minimal Global & Workspace Fallback | 最小全局与工作区回退快捷键

These bindings do not all live at the same priority level.
这些绑定并不处于同一优先级层。

### 2.1 Minimal global fallback | 最小全局回退

Only these app actions stay in the minimal global fallback set. They run only after dialog scope, text-entry child scope, focused local scope, and workspace fallback have all declined the key.
只有这些应用动作保留在最小全局回退集合中。只有在 dialog 作用域、文本输入子作用域、当前局部作用域和 workspace fallback 都没有消费按键时，它们才会触发。

| Key | Action |
|---|---|
| `F1` | Help & Learning / 打开或关闭帮助与学习 |
| `Ctrl+N` | New connection / 新建连接 |
| `Ctrl+P` | Command palette / 打开命令面板 |
| `Ctrl++` / `Ctrl+-` / `Ctrl+0` | Zoom in/out/reset / 缩放界面 |

### 2.2 Workspace fallback and scoped defaults | 工作区回退与作用域默认绑定

These defaults are command-mode bindings, not global-first keys. They may be blocked by dialogs, recording, completion, text entry, or current-scope local bindings.
这些默认绑定属于命令模式绑定，不是 global-first 按键。dialog、录制态、补全、文本输入以及当前作用域的局部绑定都可以先于它们消费输入。

The appearance and sidebar jump bindings in this section are action-backed workspace fallbacks now, not special router-only keys. Rebinding them in `keymap.toml` changes the key without changing router semantics.
本节中的外观与侧边栏跳转绑定现在也都是 action-backed 的 workspace fallback，不再是 router 专用硬编码按键。修改 `keymap.toml` 只会改变绑定，不需要改 router 语义。

| Key | Action |
|---|---|
| `Ctrl+Shift+N` | New table / 新建表 |
| `Ctrl+Shift+D` | New database / 新建数据库 |
| `Ctrl+Shift+U` | New user / 新建用户 |
| `Ctrl+T` | New query tab / 新建查询标签页 |
| `Ctrl+W` | Close active tab / 关闭当前标签页 |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next/previous query tab / 下一个或上一个标签页 |
| `Ctrl+B` | Toggle sidebar / 显示或隐藏侧边栏 |
| `Ctrl+J` | Toggle SQL editor / 显示或隐藏 SQL 编辑器 |
| `Ctrl+H` | Toggle history panel / 显示或隐藏历史面板 |
| `Ctrl+R` | Toggle ER diagram / 显示或隐藏 ER 关系图 |
| `Ctrl+E` | Export result / 导出结果 |
| `Ctrl+I` | Import data/SQL / 导入数据或 SQL |
| `Ctrl+S` | Save table edits / 保存表格修改 |
| `Ctrl+G` | Goto line / 跳转到行 |
| `Ctrl+K` / `Ctrl+L` | Clear search / clear SQL input / 清空搜索或 SQL |
| `Ctrl+D` | Toggle day/night mode / 切换日间夜间模式 |
| `Ctrl+Shift+T` | Open theme picker / 打开主题选择器 |
| `Ctrl+1..6` | Jump sidebar sections / 快速定位侧边栏分区 |
| `Alt+K` | Open keybinding settings / 打开快捷键设置 |

## 3. SQL Editor | SQL 编辑器

### 3.1 Modes | 模式
- `Insert` mode (default): type/edit SQL.
  `Insert` 模式（默认）：输入与编辑 SQL。
- `Normal` mode: Helix-style navigation.
  `Normal` 模式：Helix 风格导航。
- `Esc`: close completion popup first; then leave Insert mode.
  `Esc`：优先关闭补全弹窗；无弹窗时退出 Insert。
- `i / a / o`: enter Insert mode from Normal mode.
  `i / a / o`：从 Normal 进入 Insert。

### 3.2 Execute & Explain | 执行与分析
| Key | Action |
|---|---|
| `Ctrl+Enter` | Execute SQL / 执行 SQL |
| `F5` | Execute SQL / 执行 SQL |
| `F6` | Explain plan / 执行 EXPLAIN |

### 3.3 Completion & History | 补全与历史
| Key | Action |
|---|---|
| `Ctrl+Space` / `Alt+L` | Trigger completion / 触发补全 |
| `Tab` | Open completion or confirm selected item / 打开补全或确认补全项 |
| `Enter` | Confirm selected completion (popup open) / 补全弹窗打开时确认补全 |
| `Shift+↑ / Shift+↓` | SQL history navigation / 浏览 SQL 历史 |

Note / 说明：`next_focus_area` is a workspace fallback action. `Tab` in SQL editor is consumed by completion first, not focus cycle.
说明：`next_focus_area` 是 workspace fallback action。在 SQL 编辑器中，`Tab` 优先用于补全，不会触发主区域切换。

## 4. DataGrid | 数据表格

### 4.1 Navigation (Normal mode) | 导航（Normal 模式）
| Key | Action |
|---|---|
| `h/j/k/l` or arrows | Move cursor / 移动光标 |
| `10j` style count prefix | Repeat movement / 计数前缀重复移动 |
| `w / b` | Move one column right/left / 右移或左移一列 |
| `0` / `^` / `$` | Start/start/end of row / 行首或行尾 |
| `gg / G` | First/last row / 第一行或最后一行 |
| `gh / gl` | First/last column / 第一列或最后一列 |
| `Ctrl+u / Ctrl+d` | Half page up/down / 上下半页 |
| `PageUp / PageDown` | Page up/down / 翻页 |
| `Home / End` | Row start/end (`Ctrl` to file start/end) / 行首行尾（`Ctrl` 为全表首尾） |
| `zz / zt / zb` | Center/top/bottom viewport / 视图居中或置顶置底 |

### 4.2 Edit & Selection | 编辑与选择
| Key | Action |
|---|---|
| `i / a / c / r` | Enter edit / append / clear+edit / replace / 进入编辑、追加、清空后编辑、替换 |
| `v / x / % / ;` | Select mode / row select / select all / collapse selection / 选择模式、整行选择、全选、折叠选择 |
| `dd` or `Space d` | Mark delete row / 标记删除当前行 |
| `yy / p` | Copy row / paste / 复制行与粘贴 |
| `u / U` | Undo cell edit / unmark row delete / 撤销单元格修改、取消删除标记 |
| `o / O` | Insert row below/above / 在下方或上方插入新行 |
| `:w` or `Ctrl+S` | Save changes / 保存修改 |
| `q` or `:q` | Discard edits / 放弃修改 |
| `/` or `f` | Quick filter / filter current column / 快速筛选或当前列筛选 |

### 4.3 Edge Focus Transfer | 边界焦点转移
- `h / k` stay inside the grid at top/left edges.
  `h / k` 在左边界和上边界会停留在表格内部。
- Last row + `j` -> SQL Editor.
  最后一行按 `j` -> SQL 编辑器。

## 5. Query Tabs | 查询标签栏
| Key | Action |
|---|---|
| `h / l` or left/right | Switch tabs / 切换标签 |
| `Enter` or `j` | Move to DataGrid / 进入数据表格 |
| `k` | Move to Toolbar / 进入工具栏 |
| `d` | Close current tab (keep at least one) / 关闭当前标签（至少保留一个） |
| `Esc` | Back to DataGrid / 返回数据表格 |

## 6. Toolbar | 工具栏
| Key | Action |
|---|---|
| `h / l` or left/right | Move tool selection / 左右移动工具项 |
| `Enter` | Activate selected tool / 激活当前工具项 |
| `j` | Move to Query Tabs / 进入查询标签栏 |
| `Esc` | Back to Query Tabs / 返回查询标签栏 |

## 7. Sidebar | 侧边栏
- `j/k`: move within the current list; at list edges they can transfer to the previous/next sidebar workspace when edge transfer is enabled.
  `j/k` 在当前列表内移动；启用边界转移时，在列表边界可进入上一个或下一个侧边栏工作区。
- `sidebar.edge_transfer = false`: `j/k` stop at the current list edge and never cross into another sidebar panel.
  `sidebar.edge_transfer = false`：`j/k` 会停在当前列表边界，不会跨到其他侧边栏面板。
- `h/l`: move along the sidebar focus graph: Connections -> Databases -> Tables -> Filters -> Triggers -> Routines.
  `h/l` 沿侧边栏焦点图移动：连接 -> 数据库 -> 表 -> 筛选 -> 触发器 -> 存储过程。
- `Enter`: activate the selected connection/database/table/filter item.
  `Enter` 激活当前选中的连接、数据库、表或筛选项。
- `Ctrl+1..6`: quick section jump (connections, databases, tables, filters, triggers, routines).
  `Ctrl+1..6` 快速定位分区（连接、数据库、表、筛选、触发器、存储过程）。

### 7.1 Filters Workspace | Filters 工作区
- `filters.list` is command mode; `filters.input` is text-entry mode.
  `filters.list` 是命令模式；`filters.input` 是文本输入模式。
- `j / k`: previous / next rule.
  `j / k`：上一条 / 下一条规则。
- `a / A`: insert below / append at end.
  `a / A`：在当前项下方插入 / 追加到末尾。
- `x`: delete current rule.
  `x`：删除当前规则。
- `Space`: enable / disable current rule.
  `Space`：启用 / 禁用当前规则。
- `o`: toggle `AND / OR`.
  `o`：切换 `AND / OR`。
- `[` / `]`: previous / next column.
  `[` / `]`：上一列 / 下一列。
- `-` / `=`: previous / next operator.
  `-` / `=`：上一个 / 下一个操作符。
- `l`: enter value input when the current operator needs text.
  `l`：当当前操作符需要值时进入 value input。
- `Esc` in `filters.input`: return to `filters.list`.
  `filters.input` 中按 `Esc`：返回 `filters.list`。
- `Esc` in `filters.list`: move back to the previous sidebar layer, same as `h`.
  `filters.list` 中按 `Esc`：回到上一个侧边栏层级，语义与 `h` 相同。

## 8. Dialogs & Panels | 对话框与面板
Only the active dialog owns keyboard input. If multiple panels are open due to a workflow transition, the top-priority dialog handles keys first.
只有当前 active 对话框拥有键盘输入。如果工作流切换导致多个面板处于打开状态，最高优先级对话框会先处理按键。

| Key | Action |
|---|---|
| `Enter` | Confirm / 提交确认 |
| `Esc` or `q` | Cancel/close / 取消或关闭 |
| `y / n` | Confirm prompt / confirm reject / 确认框中同意或拒绝 |

## 9. Troubleshooting | 常见问题排查
- Key does nothing: check current focus area first.
  按键无效：先确认当前焦点区域。
- `F5` behaves differently: expected by design, depends on focus.
  `F5` 行为不同：这是设计行为，取决于焦点。
- `Tab` in SQL editor: completion first, not global focus switch.
  SQL 编辑器里的 `Tab`：优先补全，不是全局焦点切换。

## 10. Learning Order | 建议学习顺序
1. Learn focus flow: `Tab` + `hjkl`.
   先学焦点流转：`Tab` + `hjkl`。
2. Learn four core keys: `Ctrl+N`, `Ctrl+Enter`, `Ctrl+S`, `F1`.
   再掌握四个核心键：`Ctrl+N`、`Ctrl+Enter`、`Ctrl+S`、`F1`。
3. Then learn DataGrid edit combo: `i`, `dd/yy/p`, `:w`.
   最后学表格编辑组合：`i`、`dd/yy/p`、`:w`。
