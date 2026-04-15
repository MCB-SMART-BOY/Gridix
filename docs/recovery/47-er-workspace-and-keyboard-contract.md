# ER Workspace And Keyboard Contract

## Scope

这份文档只定义 ER 主线进入实现前必须先冻结的 3 个 contract：

- ER 在 Gridix 里的 workspace 角色
- ER 的权威状态边界
- ER 的键盘流与焦点语义

本文件不实现代码，也不直接改视觉。

## Confirmed Facts

当前仓库中已经可以确认的事实：

1. 顶层显隐当前由 `DbManagerApp.show_er_diagram` 驱动。
   - [src/app/input/input_router.rs](../../src/app/input/input_router.rs)
   - [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs)

2. `ERDiagramState.show` 已删除。
   - 运行期显隐权威继续只保留 `DbManagerApp.show_er_diagram`

3. ER 不是 dialog，也不是 overlay。
   - 当前是中心工作区里的并列 surface
   - [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs)

4. ER 的键盘输入已进入显式 focus owner 模型的第一阶段。
   - `FocusArea::ErDiagram` 已落地
   - `consume_er_diagram_key_action()` 现在只在 ER 显式聚焦时触发
   - 当 `show_er_diagram == false` 时，残留的 `FocusArea::ErDiagram` 不再继续持有 `FocusScope::ErDiagram`
   - [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs)

5. 当前全局焦点图里已经有 `FocusArea::ErDiagram`。
   - [src/ui/mod.rs](../../src/ui/mod.rs)

6. ER 的异步回包直接写 `er_diagram_state`。
   - `load_er_diagram_data()`
   - `handle_foreign_keys_fetched()`
   - `handle_er_table_columns_fetched()`
   - [src/app/runtime/er_diagram.rs](../../src/app/runtime/er_diagram.rs)
   - [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs)

7. ER 的视觉 token 仍主要来自 `RenderColors::from_theme()` 的内部色板。
   - [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs)

## 1. Workspace Role Contract

### 1.1 Product role

ER 的第一阶段产品定位应固定为：

- `workspace companion pane`

不是：

- dialog
- full-screen graph tool
- 独立子应用

这意味着：

- ER 应继续作为中心工作区中的并列 surface 存在
- 它的第一任务是“关系浏览”，不是“画布编辑”
- 它必须能和 `DataGrid / SQL Editor / Sidebar` 形成清晰的焦点切换关系

### 1.2 Visibility authority

实现前先在设计上冻结：

- `show_er_diagram` 是唯一显隐权威源

相关入口：

- `set_er_diagram_visible()`
- `toggle_er_diagram_visibility()`
- `AppAction::ToggleErDiagram`

当前结论：

- 运行期显隐路径已统一汇到 `set_er_diagram_visible_with_notice(...)`
- 返回历史 contract 现已补到 app-level workspace 状态：
  - 从 ER 返回或关闭且当前焦点在 ER 时，会恢复最近一个合法的非 ER workspace 区域
  - 当前仍不恢复 `Toolbar / QueryTabs / Dialog`

### 1.3 Open / close semantics

建议冻结为：

- 打开 ER：
  - 保留当前连接上下文
  - 触发一次受控数据加载
  - 不改写主查询结果状态
  - 直接切入 ER 焦点，避免已打开 ER 却仍由 `DataGrid` 吃掉 `h/j/k/l`
- 关闭 ER：
  - 只关闭 workspace companion pane
  - 不清空主业务查询链路
  - 若当前焦点在 ER，则回退到最近一个合法的非 ER workspace 区域
  - 若历史目标当前不可用，则回退到 `DataGrid`

## 2. State Ownership Contract

当前 `er_diagram_state` 混放了多类状态。实现前先按语义拆账，而不是立刻拆代码。

### 2.1 Loaded graph

应包含：

- `tables`
- `relationships`

生产者：

- `load_er_diagram_data()`
- `handle_er_table_columns_fetched()`
- `handle_foreign_keys_fetched()`

约束：

- 只表示“当前加载出的 ER 图内容”
- 不能兼任显隐语义

### 2.2 Viewport state

应包含：

- `pan_offset`
- `zoom`
- `dragging_table`
- `drag_start`
- `interaction_mode`

约束：

- 这是局部视图状态
- 不应影响连接、查询、tab 或 grid 业务状态
- `interaction_mode` 当前只允许在 `Navigation / Viewport` 两个局部模式间切换
- `clear()` 必须把 `interaction_mode` 保守重置回 `Navigation`

### 2.3 Selection state

应包含：

- `selected_table`
- `ERTable.selected`
- `pending_selection_reveal`

风险：

- 与 app 层 `selected_table` 同名但不同义
- 后续实现必须明确“ER 选中表”与“主业务当前表”何时同步、何时不同步

当前已落地：

- 任何通过键盘导航或进入 ER scope 重新确认的选中项，都会先标记 `pending_selection_reveal`
- 真正的视口修正继续延迟到 render 拿到 canvas 尺寸后执行，而不是把 viewport 尺寸反向塞回 input/router
- 新增的几何邻接导航也只允许改写 ER 自己的 `selected_table`；在 `OpenSelectedTable` 之前，它不应直接同步 app 层 `selected_table`

### 2.4 Load lifecycle

应包含：

- `loading`
- `needs_layout`

约束：

- 只描述 ER 自己的数据加载与布局阶段
- 不应承担显隐控制

### 2.5 Visibility ownership cleanup

- 旧 `ERDiagramState.show` 已移除

当前结论：

- ER loaded graph / viewport / selection / lifecycle 现在不再混入显隐语义
- 运行期显隐继续只由 `show_er_diagram` 承担
- 隐藏 ER 后，input owner 也不会再因为陈旧 `FocusArea::ErDiagram` 而残留在 ER scope

## 3. Keyboard Flow Contract

### 3.1 Why current behavior is insufficient

当前已经收掉的核心问题是：

- 输入所有权不再依赖 `has_focus() || hovered()`
- hover 现在只负责 hover 反馈，不再负责抢键

当前已收口的结论是：

- 第一阶段 TUI 风格导航语义已落地，返回历史也已补到 app-level workspace 状态
- bare `h / l` 已回收到 ER 内部的左右几何邻接，`Enter / Right` 继续打开当前表，`Left / Esc` 继续返回主工作区，`Shift+L` 继续承担 relayout
- `j/k` 当前仍是稳定线性选表，但选中项已不再允许停留在视口外；render 会在下一帧把它滚回可见区域
- 现在又新增了 additive 的关系邻接导航：`Shift+J / Shift+K` 只在当前表的关联集合内按稳定全局表顺序前进/后退，不替换原有 `j/k` 的线性浏览
- 本轮又新增了 additive 的几何邻接导航：`Shift+Left / Shift+Down / Shift+Up / Shift+Right` 只按当前表卡的几何中心和方向选择最近邻，不替换线性或关系邻接
- ER 默认完成态布局也已从“始终 grid”收口为“加载中先 grid skeleton，finalize 后若存在关系则自动走关系层级种子 + force-directed refine；只有空关系图才保持 grid”
- 关系层级种子现在不再让同层兄弟表继续吃原始输入顺序，而是先做稳定名称初始化，再按关系邻居重心做轻量 sweep，因此同层横向顺序也开始体现关系结构
- `v` 现在只由 router 承担视口模式切换，ER render 不再重复消费同一局部快捷键；显式聚焦后一次按键只能在 `Navigation / Viewport` 间切一次
- 关系优先布局现在还会参考真实表卡尺寸：层级种子不再继续用固定 `180x200` 近似，大表在默认完成态下也不应再轻易互相压住
- ER 当前又显式拆出“语义图摘要 -> 布局策略 -> 视图密度”三层：`ERGraphSummary` 只负责总结组件与关系结构，`ERLayoutStrategy` 只负责完成态布局路径选择，而 `ERCardDisplayMode / EREdgeDisplayMode` 只负责 ER 本地表卡与边的密度展示，不反向污染 app-level 业务状态

当前主要剩余风险不再是 keyboard contract 本身，而是几何邻接是否还需要进一步启发式调优；独立 detail mode 现阶段已明确冻结，不进入实现。

### 3.2 Required future focus identity

当前已落地：

- ER 拥有独立焦点身份：`FocusArea::ErDiagram`
- 主循环已把 ER 纳入 `Sidebar -> DataGrid -> ErDiagram -> SqlEditor`
- `ToggleErDiagram` 从隐藏打开 ER 时现在也会直接切入 `FocusArea::ErDiagram`
- 鼠标点击 ER 画布或工具条会把焦点显式切入 ER
- 已新增显式 workspace action：`FocusErDiagram` / `focus_er_diagram`
- 默认绑定当前是 `Alt+R`
- 该动作只在 ER 已打开时可用；它只切入 `FocusArea::ErDiagram`，不改显隐，也不触发 reload

当前下一步：

- 不再讨论“是否需要独立焦点身份”
- 返回历史已不再是 open contract
- 后续只继续在这个显式焦点身份之上处理残余快捷键兼容与更强导航，而不是再回退到 hover/canvas owner

### 3.3 Entry / stay / exit

当前已落地的第一阶段语义：

- 进入 ER：
  - 来自显式 workspace action
  - 来自显式 `FocusErDiagram`
  - 或从主工作区切焦点进入
- 停留在 ER：
  - 当前局部命令只由 ER scope 消费
  - 默认子作用域是 `er_diagram`
  - `v` 会切到 `er_diagram.viewport`
- 打开当前表：
  - `OpenSelectedTable` 仍会把详情承载回主工作区
  - 当前证据链是 `selected_table` 同步到 app 层后直接分发 `QuerySelectedTable`
  - 它不是未来 detail mode 的过渡壳层，而是现行 contract 本身
- 顶层 toggle：
  - `Ctrl+R` 仍通过 `ToggleErDiagram` 打开/关闭 ER
  - 即使当前已在 `er_diagram` 或 `er_diagram.viewport`，也不能把这个 toggle gate 掉
  - 但这不意味着其他 workspace overlay shortcut 会一起在 ER scope 放开
- 离开 ER：
  - `Esc / h / Left`：退出 ER 焦点，恢复最近一个合法的非 ER workspace 区域
- `q` 或 `ToggleErDiagram`：关闭 ER
- 停留在视口模式：
  - `h / j / k / l`：平移画布
  - `Esc`：只退出视口模式，回到浏览模式
  - `q`：仍可直接关闭 ER
  - `r / Shift+L / f / +/-`：继续可用

### 3.4 Navigation policy

对 Gridix 这种键盘流软件，当前已经落成的第一阶段原则是：

- `hjkl` 的默认语义应是“语义导航”
- 只有显式进入次级视口模式后，`hjkl` 才切到画布平移

当前第一阶段已经是：

1. `j / k`
   - 按 `tables` 的稳定数据顺序在线性表列表里移动选择
2. `h / l`
   - 在 ER 浏览态内执行左右几何邻接
   - 只改写 ER 局部 `selected_table`
3. `Enter / Right`
   - 进入当前选中表的业务上下文
   - 例如同步到主数据表视图或切换当前表
   - 焦点当前回落到 `DataGrid`
4. `Esc / Left`
   - 返回主工作区
5. `q`
   - 关闭 ER
6. `Shift+L`
   - relayout
7. `v`
   - 在浏览模式与视口模式之间切换
8. 视口模式中的 `h / j / k / l`
   - 只承担平移，不再继续做浏览导航
9. `Shift+Left / Shift+Down / Shift+Up / Shift+Right`
   - 在浏览模式内按当前布局做方向性几何邻接
   - 只增强局部 ER 浏览，不直接切主业务当前表

原因：

- 这更接近 TUI/文件管理器式关系浏览
- 也更符合 Gridix 当前“焦点先于按键解释”的整体模型
- 同时保留了图形画布必须存在的局部平移能力，但不再让它默认压过关系浏览语义

### 3.5 Command split

当前已落地的 split 是：

1. Workspace commands
   - open
   - close
   - refresh
   - fit view
   - relayout
   - return to main workspace
   - focus ER

2. ER navigation commands
   - next/previous table
   - geometric neighbor selection
   - open selected table
   - return to previous workspace area
   - close ER
   - toggle viewport mode

3. ER viewport commands
   - zoom in/out
   - pan
   - fit view
   - relayout
   - refresh
   - exit viewport mode
   - drag table

### 3.6 Detail-mode decision

当前结论：

- 不进入独立 `detail mode`
- ER 继续保持 companion pane 定位：负责关系浏览、方向跳转、视口操作与打开当前表
- 表详情、数据结果、主键拉取与 grid workspace 继续由主工作区承载

证据链：

1. `OpenSelectedTable`
   - [src/app/input/input_router.rs](../../src/app/input/input_router.rs)
2. `QuerySelectedTable`
   - [src/app/action/action_system.rs](../../src/app/action/action_system.rs)
3. `selected_table_query_effects()`
   - `switch_grid_workspace()`
   - `ExecuteSql`
   - `FetchPrimaryKey`

原因：

- 当前 ER state 只有 graph / viewport / selection / lifecycle，没有 detail pane 的 owner 或容器
- 现有主工作区已经承担“详情 + 数据 + grid workspace”主链
- 现在引入独立 detail mode 只会制造第二套详情 owner，并模糊 `ER selected_table` 与 app `selected_table` 的边界

当前结论：

- `consume_er_diagram_key_action()` 仍在 render 层承接画布/视口语义
- router 现在承接浏览语义与模式切换
- 两者已经由 `FocusScope::ErDiagram(Navigation|Viewport)` 和 `interaction_mode` 明确分层，而不再是单层 scope 内的混合解释

## 4. Token Map Contract

这一步仍不改视觉，只先定义映射原则。

### 4.1 What should become shared

应逐步对齐到 Gridix 主 token 的部分：

- toolbar chrome
- text hierarchy
- empty/loading 文案层级
- selection/highlight 颜色语义
- border / shadow / radius 规则

### 4.2 What should remain ER-specific

应保留“画布感”的部分：

- grid 背景节奏
- 关系线样式
- 缩放与画布操作反馈

目标不是把 ER 抹平成普通表单，而是：

- 让它看起来像 Gridix 的图形视图，而不是外来子系统

## 5. Next Non-Implementation Tasks

实现前，先按这个顺序推进：

1. 写清 ER 的显隐入口矩阵
   - 普通 action
   - help 学习入口
   - overlay close

2. 写清 ER 的状态账本
   - loaded graph
   - viewport
   - selection
   - lifecycle

3. 写清 ER 的键盘流图
   - 进入
   - 导航
   - 退出
   - 返回主工作区

4. 写清 token 映射表
   - 当前 ER 私有 token
   - 对应主 UI token

5. 再决定第一刀实现
   - 优先级建议：
     1. 收口显隐入口
     2. 让 ER 拥有显式 focus identity
     3. 再处理 token

当前这些材料已分别落到：

- [48-er-visibility-entry-matrix-and-state-ledger.md](./48-er-visibility-entry-matrix-and-state-ledger.md)
- [49-er-keyboard-flow-graph.md](./49-er-keyboard-flow-graph.md)

## 6. Do Not Do First

在上述 contract 未冻结前，不建议先做：

- 直接重绘 ER 视觉风格
- 继续强化 hover 驱动的快捷键模型
- 把 ER 直接改造成 dialog / modal
- 先把 `hjkl` 绑定到默认平移

## 7. Exit Criteria Before Implementation

只有下面 4 项都具备，才应进入 ER 实现：

1. `show_er_diagram` 的权威性结论已经固定
2. 重复显隐字段已经清理完毕
3. ER 的键盘流图已经能和现有 `KEYBOARD_FOCUS_RFC` 对齐
4. token map 已经列出“共享”与“保留特性”边界
