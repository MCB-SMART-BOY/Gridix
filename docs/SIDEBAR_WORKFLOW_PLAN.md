# Sidebar Workflow Plan | 侧边栏工作流改造方案

This document defines how the sidebar should behave as a keyboard-first workflow area.  
本文定义侧边栏作为键盘优先工作区时的行为方式。

## 1. Current Problems | 当前问题

- Too many panels are visible by default.
  默认展开面板过多。
- `j/k` only move inside one list, but users expect panel-to-panel traversal.
  `j/k` 现在只在单个列表内移动，用户却期望能跨面板流转。
- Filter panel is visually present but not functionally first-class.
  筛选面板虽然可见，但还不是一等键盘工作区。
- Trigger/routine panels add noise for beginners.
  触发器/存储过程面板对新手来说噪音过大。

## 2. Default Layout | 默认布局

Default visible panels:
默认可见面板：

- `Connections`
- `Filters`

Default hidden panels:
默认隐藏面板：

- `Triggers`
- `Routines`

Database and table lists remain inside the connection panel stack.
数据库与表列表仍属于连接面板的层级内部。

## 3. Focus Graph | 焦点图

Target order:
目标顺序：

`Connections -> Databases -> Tables -> Filters -> Triggers -> Routines`

Rules:
规则：

- `h`: move to previous panel/layer
- `l`: move to next panel/layer
- `j/k`: move within current list
- optional edge transfer: if cursor is already at boundary, `j/k` may transfer to next/previous panel

Edge transfer should be configurable.
边界转移应可配置。

Suggested config:
建议配置项：

```toml
[sidebar]
edge_transfer = true
```

## 4. Filter Panel As Workspace | 将筛选面板提升为工作区

### 4.1 Sub-modes | 子模式

- `filters.list`
- `filters.input`

### 4.2 Required commands | 必备命令

- `j/k`: select previous/next rule
- `a`: add rule below
- `A`: append rule at end
- `x`: delete current rule
- `space`: enable/disable current rule
- `o`: toggle logic (`AND/OR`)
- `[` / `]`: previous/next column
- `-` / `=`: previous/next operator
- `l`: enter value input
- `Escape`: leave value input

### 4.3 Visual state | 视觉状态

Each rule should clearly show:
每条规则应清晰显示：

- enabled state
- logic connector
- column
- operator
- current value
- keyboard focus state

## 5. Panel Visibility Rules | 面板显隐规则

- Triggers/routines stay hidden unless explicitly opened.
  触发器/存储过程默认隐藏，除非用户主动打开。
- If metadata loading returns empty, do not auto-open those panels.
  若元数据为空，不自动展开这些面板。
- Help text should explain hidden panels as optional advanced areas.
  帮助文档中要把这些面板标记为可选高级区域。

## 6. Visual Cleanup | 界面整理

Current panel chrome is too repetitive.
当前面板边框和标题重复度太高。

Target cleanup:
目标整理方向：

- compact section headers
- unified panel toolbar
- stronger focus highlight
- fewer always-visible controls
- better empty-state messaging

## 7. Migration Steps | 迁移步骤

### Phase 1
- change default panel visibility
- add edge-transfer state and config
- define sidebar focus graph centrally

### Phase 2
- split filter list/input scopes
- implement filter keyboard actions
- remove filter-related global actions

### Phase 3
- redesign panel headers and toggles
- add sidebar-specific regression tests

## 8. Acceptance Criteria | 验收标准

- New user sees only essential panels by default.
  新手默认只看到必要面板。
- User can navigate from table list to filter list without mouse.
  用户无需鼠标即可从表列表进入筛选面板。
- User can fully add/edit/remove filters by keyboard.
  用户可纯键盘完成筛选条件新增、编辑、删除。

