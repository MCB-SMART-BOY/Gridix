# Gridix Keyboard Guide | 键盘指南

> Applies to `v3.3.x` default keymap.  
> 适用于 `v3.3.x` 默认键位。  
> Some keys are user-configurable in keybinding settings.  
> 部分快捷键可在“快捷键设置”中自定义。
>
> Actual runtime bindings come from `~/.config/gridix/keymap.toml`.  
> 运行时实际生效的键位以 `~/.config/gridix/keymap.toml` 为准。
> If you changed them, prefer the in-app hover tooltip and Help panel.  
> 如果你改过键位，请优先以应用内悬停提示和帮助面板为准。

## 1. Focus First | 先看焦点
- `Tab / Shift+Tab`: cycle major focus areas (Sidebar -> DataGrid -> SQL Editor).
  `Tab / Shift+Tab`：循环切换主区域焦点（侧边栏 -> 数据表格 -> SQL 编辑器）。
- `hjkl`: navigate inside current area.
  `hjkl`：在当前焦点区域内导航。
- Same key can mean different actions in different areas.
  同一按键在不同区域语义可能不同。

Example / 示例：`F5`  
- In SQL Editor: execute SQL.
  在 SQL 编辑器：执行 SQL。
- Outside SQL Editor: refresh connection/table state.
  在编辑器外：刷新连接/表状态。

## 2. Global Shortcuts | 全局快捷键

| Key | Action |
|---|---|
| `F1` | Help & Learning / 打开或关闭帮助与学习 |
| `Ctrl+N` | New connection / 新建连接 |
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
| `Ctrl+F` / `Ctrl+Shift+F` | Add/clear filter / 添加或清空筛选 |
| `Ctrl+G` | Goto line / 跳转到行 |
| `Ctrl+K` / `Ctrl+L` | Clear search / clear SQL input / 清空搜索或 SQL |
| `Ctrl+D` | Toggle day/night mode / 切换日间夜间模式 |
| `Ctrl++` / `Ctrl+-` / `Ctrl+0` | Zoom in/out/reset / 缩放界面 |
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

Note / 说明：`Tab` in SQL editor is consumed by completion first, not focus cycle.
在 SQL 编辑器中，`Tab` 优先用于补全，不会触发全局焦点循环。

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
- First column + `h` -> Sidebar.
  第一列按 `h` -> 侧边栏。
- Last row + `j` -> SQL Editor.
  最后一行按 `j` -> SQL 编辑器。
- First row + `k` -> Query Tabs.
  第一行按 `k` -> 查询标签栏。

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
- `j/k`: move selection; `Enter`/`l`: open/select; `h`: go back or collapse.
  `j/k` 上下移动；`Enter`/`l` 打开或确认；`h` 返回或折叠。
- `Ctrl+1..6`: quick section jump (connections, databases, tables, filters, triggers, routines).
  `Ctrl+1..6` 快速定位分区（连接、数据库、表、筛选、触发器、存储过程）。

## 8. Dialogs & Panels | 对话框与面板
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
