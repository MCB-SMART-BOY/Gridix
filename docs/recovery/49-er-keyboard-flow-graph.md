# ER Keyboard Flow Graph

## Scope

这份文档现在记录 ER 键盘流从设计进入实现后的当前状态与下一步。

目标：

- 让 ER 从“鼠标画布工具”变成 Gridix 的正式键盘工作区
- 明确进入、停留、退出、关闭四种语义
- 明确哪些动作属于 workspace 级，哪些属于 ER 局部 scope

本文件不直接记录视觉实现。

## 1. Confirmed Current Behavior

当前已经可以确认的行为：

1. `Ctrl+R` 通过 `ToggleErDiagram` 打开/关闭 ER。
   - 即使当前已在 `er_diagram` 或 `er_diagram.viewport`，这个 toggle 也必须继续可用。
   - 但这不等价于把 `Ctrl+H` 一类其他 workspace overlay shortcut 一并放进 ER scope。
   - [src/app/action/action_system.rs](../../src/app/action/action_system.rs)

2. ER 局部快捷键现在只在 ER 显式聚焦时消费。
   - [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs)

3. 当前局部快捷键集合是：
   - `j / k`: previous / next table
   - `h / l`: left / right geometric navigation inside ER
   - `Enter / Right`: open selected table into the main workspace
   - `Left / Esc`: return to the most recent legal non-ER workspace area
   - `q`: close ER
   - `r`: refresh
   - `Shift+L`: layout
   - `f`: fit view
   - `+`: zoom in
   - `-`: zoom out
   - `v`: toggle viewport mode

4. 当前 ER 局部 scope 已继续拆成两层：
   - `er_diagram`：浏览语义
   - `er_diagram.viewport`：视口语义
   - `v` 的模式切换现在只经由 router 生效，render 不再重复消费同一按键

5. 进入 `er_diagram.viewport` 后：
   - `h / j / k / l`: pan
   - `Esc`: exit viewport mode back to navigation
   - `q`: close ER
   - `r / Shift+L / f / +/-`: remain available

6. 当前全局焦点图里已经有 `FocusArea::ErDiagram`。
   - [src/ui/mod.rs](../../src/ui/mod.rs)

7. 当前 `Esc` 在浏览态 ER scope 内不再关闭 ER，而是返回最近一个合法的非 ER workspace 区域；只有 ER 未聚焦时，workspace overlay fallback 才会关闭 ER。
   - [src/app/input/input_router.rs](../../src/app/input/input_router.rs)

8. 打开 ER 现在会直接切入 ER 焦点；关闭 ER 时若当前焦点在 ER，会回退到最近一个合法的非 ER workspace 区域，并在目标不可用时回退到 `DataGrid`。
   - [src/app/input/input_router.rs](../../src/app/input/input_router.rs)

## 2. Why Current Keyboard Semantics Are Not Enough

当前已经收掉的问题是：

- hover 不再抢键
- 鼠标点击 ER 才会显式进入 ER scope
- 用户现在可以稳定判断“我是否在 ER 焦点里”

当前剩下的问题是：

1. 当前返回历史只覆盖 major workspace area，不覆盖 `Toolbar / QueryTabs / Dialog`
2. 当前已经同时存在三条 additive 浏览链：`j/k` 的稳定线性选表、`Shift+J/K` 的关系邻接、`Shift+Arrow` 的几何邻接；后续若继续扩展，必须避免把这三层重新搅回同一条模糊语义
3. 当前没有独立 detail mode，而且这条方向现已冻结；`OpenSelectedTable` 继续作为现行 contract，直接离开 ER 回主工作区

当前已新增的底线是：

- 键盘把 `selected_table` 改到别的表后，下一帧必须把该表滚回当前可见视口
- 这条“selection follows viewport”现在已经落地；在此基础上，浏览态又新增了 additive 的关系邻接与几何邻接，但它们都不替换原有线性浏览
- 默认完成态布局现在也已切到“关系优先但仍可预测”：加载中的骨架先走稳定 grid，finalize 后若存在显式或推断关系，则会自动走关系层级种子 + force-directed refine；只有空关系图才保持 grid
- 关系优先布局现在还会对同层兄弟表做关系邻居重心排序，不再把原始输入顺序直接映射成同层横向顺序
- 当前 ER render 还新增了本地视图密度层：`All / Focus / ExplicitOnly` 边模式与 `Standard / KeysOnly` 表卡密度都只作用于 ER 局部浏览体验，不改变键盘 contract，也不反向污染 app-level 当前表

## 3. Target Position In Focus Graph

### 3.1 ER should become a major workspace area

当前已落地：

- ER 是一个独立 major focus area
- 对应 `FocusArea::ErDiagram`

这意味着：

- ER 不再只是中央面板右侧的鼠标画布
- 它已经和 `Sidebar / DataGrid / SqlEditor` 一样，拥有显式键盘 owner 身份

### 3.2 Proposed major focus cycle

当 `show_er_diagram == true` 时，主区域循环建议变成：

- `Sidebar -> DataGrid -> ErDiagram -> SqlEditor`

当 `show_er_diagram == false` 时：

- 保持当前 `Sidebar -> DataGrid -> SqlEditor`

不建议把 `Toolbar / QueryTabs` 纳入主循环，它们仍保持现有的局部导航入口。

### 3.3 Open now intentionally enters ER focus

当前已冻结：

- `ToggleErDiagram` 打开 ER 时会直接把焦点切入 ER
- 显式聚焦继续由独立 `FocusErDiagram` 承担，默认绑定 `Alt+R`，用于 ER 已打开但焦点已离开后的回焦

原因：

- 结果表 / SQL 工作区仍在场时，如果打开 ER 却不入焦，默认 `h/j/k/l` 很容易继续被主工作区消费
- 当前产品定位已把 ER 视为显式 major workspace area，而不是只读 companion overlay

例外：

- **推测**：帮助学习路径若明确是“打开 ER 供浏览”，未来可以单独提供“打开并进入 ER”动作
- 但这不应成为默认 contract

## 4. Entry / Stay / Exit / Close Semantics

### 4.1 Entry

当前已支持的进入方式：

1. `next_focus_area` / `prev_focus_area`
2. 鼠标点击 ER 画布或表节点
3. 显式 action：`focus_er_diagram`
   - 默认绑定：`Alt+R`
   - 只在 ER 已打开时可用
   - 只切入 `FocusArea::ErDiagram`，不打开、不关闭、不 reload

不建议：

- 仅靠 hover 进入 ER scope

### 4.2 Stay

一旦 ER 拥有焦点：

- ER scope 内命令应优先于 workspace fallback
- 鼠标 hover 不再改变 owner
- 只有显式焦点切换、鼠标点击其他区域或关闭 ER 才会离开
- 当前停留语义已分成两层：
  - 默认 `er_diagram`：关系浏览
  - 次级 `er_diagram.viewport`：画布视口操作

### 4.3 Exit

当前已实现：

- `Esc / h / Left`：退出 ER 焦点，恢复最近一个合法的非 ER workspace 区域

当前已实现的 close fallback：

- 关闭 ER 时若当前焦点在 ER，回退到最近一个合法的非 ER workspace 区域；若不可用则回退到 `DataGrid`

当前 contract：

- 只记忆 `Sidebar / DataGrid / SqlEditor`
- 不记忆 `Toolbar / QueryTabs / Dialog`

### 4.4 Close

当前已存在：

- `ToggleErDiagram`：打开/关闭 ER
- `q`：关闭 ER

当前结论：

- `Esc` 已切到“退回主工作区”
- `q` 已接管本地关闭语义

## 5. Navigation Contract

### 5.1 Principle

ER 的下一阶段第一语义仍应是“关系浏览”，不是“画布操作”。

因此：

- `hjkl` 不应默认等于 pan
- 只有显式进入 `viewport mode` 后，`hjkl` 才切到 pan

### 5.2 Stage-1 keyboard semantics

当前第一阶段已冻结成更稳的 TUI 风格：

- `j / k`
  - 在可选表之间移动
- `Shift+J / Shift+K`
  - 仅在当前表的关联集合内移动
  - 关联集合按稳定全局表顺序解释，不依赖当前几何布局
- `Shift+Left / Shift+Down / Shift+Up / Shift+Right`
  - 仅在当前布局内做方向性几何邻接
  - 优先选择请求方向内的最近候选；若无轴向候选，再保守回退到同方向对角候选
- `Enter / Right`
  - 打开当前选中表的业务上下文
  - 例如同步到主表格视图 / 侧边栏 / 当前表工作区
- `h / Esc`
  - 返回主工作区
- `q`
  - 关闭 ER

这样设计的好处：

- 与 TUI 文件管理器/浏览器的“上下移动、向右打开、向左返回”一致
- 避免一开始就把图形二维方向映射得过度复杂

### 5.3 Deterministic selection order

为了让 `j / k` 可预测，第一阶段必须有稳定的表选择顺序。

当前顺序优先级：

1. 当前 `tables` 的稳定数据顺序
2. 若后续需要，再升级成显式 `selection_order`

不建议第一阶段就依赖：

- 当前屏幕位置
- force-directed layout 后的几何邻接

因为这些会让键盘流受布局算法影响。

当前结论：

- `j / k` 继续作为稳定、与布局无关的主浏览链
- `Shift+J / Shift+K` 继续作为关系邻接链
- `Shift+Arrow` 现已作为 additive 的几何邻接链落地，但只增强方向性跳转，不取代前两条稳定浏览链

## 6. Command Layer Split

### 6.1 Workspace-level commands

这些命令属于 workspace 层，不属于画布局部：

- toggle open/close
- focus ER
- return to previous workspace area

### 6.2 ER-local commands

这些命令现在继续分成两组：

- `er_diagram`
  - next/previous table
  - geometric neighbor selection
  - open selected table
  - return to previous workspace area
  - close ER
  - toggle viewport mode
- `er_diagram.viewport`
  - exit viewport mode
  - pan
  - refresh
  - fit view
  - relayout
  - zoom in/out

## 7. Shortcut Collision Resolution

此前的冲突已经收口：

- bare `l` 不再用于 `layout`
- bare `l` 已从“打开当前表”回收到 ER 内部的右向几何邻接
- `Enter / Right` 现在统一表示“打开当前选中表”
- `Shift+L` 继续承担 relayout

当前结论：

- ER 键盘流已经切到更强的 TUI `h/l` 主线
- `h/j/k/l` 的默认语义现在都留在 ER 内部：`j/k` 负责线性浏览，`h/l` 负责左右几何邻接；视口平移已被明确压到 `er_diagram.viewport`
- 当前 remaining work 不再是“bare key 该不该冲突”，而是更强导航模型与更高层的端到端回归覆盖

## 8. Mouse Parallel Contract

ER 仍然需要保留鼠标工作方式，但它不应再定义键盘 owner。

建议固定：

- 鼠标点击画布/表节点：进入 ER 焦点
- 鼠标拖拽：继续负责 pan / drag table
- 鼠标 hover：只负责 hover 效果，不负责抢占键盘语义

## 9. Verification Matrix Before Implementation

进入实现前，应先准备这些验证点：

1. `ToggleErDiagram` 打开时不抢焦点
2. `next_focus_area` 在 ER 可见时能进入 ER
3. 点击 ER 画布会让 ER 成为 focus owner
4. `Esc` 从 ER 返回主工作区，而不是直接关闭
5. `q` 或 toggle 才真正关闭 ER
6. `j / k` 的表选择顺序稳定，不受布局抖动影响
7. `Shift+J / Shift+K` 只在当前表的关联集合内按稳定全局表顺序移动，不依赖布局几何
8. `Shift+Arrow` 只按当前布局做方向性几何邻接；该方向没有候选时保持 no-op，且不直接同步主业务当前表
9. `Enter` 或 `l` 能把选中表同步回主业务流
10. `v` 进入视口模式后，`h/j/k/l` 只做平移，不再继续驱动表选择
11. 视口模式内 `Esc` 只退出到浏览态，而不是直接离开 ER
12. 关闭 / reload 后 `interaction_mode` 会回到 `Navigation`
13. 视口模式内 `q` 仍关闭 ER，`r / Shift+L / f` 仍保持可用，不会因为切到次级局部模式而失效
14. 跨主题 render 不会改写当前 `interaction_mode / selected_table / pan_offset / zoom`
15. `FocusErDiagram -> ToggleErDiagram` 从视口模式关闭时，会恢复最近一个合法的非 ER workspace 区域；若目标不可用，则回退到 `DataGrid`

## 10. Next Non-Implementation Step

当前 viewport mode、top-level combo coverage、关系邻接与几何邻接已经基本落地，下一步只剩两条更高层主线：

1. 观察当前 additive 几何邻接是否已足够，还是仍需要更强的方向启发式
2. 不再把独立 detail mode 作为近端实现前提；若未来重开，必须先证明主工作区详情链不再适合作为唯一详情承载层
