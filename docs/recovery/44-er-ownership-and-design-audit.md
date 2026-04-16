# ER Ownership And Design Audit

## Scope

这份文档现在处理 `G41-B005` **剩余 open 部分** 的设计与审计边界：

- ER 显隐谁是权威源
- ER 状态边界怎么分
- ER 设计语言为什么仍然像独立子系统
- 下一步应该先验证什么，而不是直接改视觉

本文件先做审计和设计，不实现代码。

## Current Entry Chain

当前 ER 的打开链路是：

1. `AppAction::ToggleErDiagram`
   - [src/app/action/action_system.rs](../../src/app/action/action_system.rs)
2. `set_er_diagram_visible(...)`
   - [src/app/input/input_router.rs](../../src/app/input/input_router.rs)
3. `load_er_diagram_data()`
   - [src/app/runtime/er_diagram.rs](../../src/app/runtime/er_diagram.rs)
4. `render.rs` 中根据 `self.show_er_diagram` 决定是否进入左右分栏
   - [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs)
5. `self.er_diagram_state.show(...)`
   - [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs)

结论：

- 顶层可见性当前由 `DbManagerApp.show_er_diagram` 驱动
- ER 不是 dialog，也不是 overlay
- 它实际上是中心 workspace 的一个并列 surface

## Authority Audit

### 1. Visibility authority

当前证据：

- [src/app/mod.rs](../../src/app/mod.rs) 持有 `show_er_diagram: bool`
- [src/app/input/input_router.rs](../../src/app/input/input_router.rs)
  - `set_er_diagram_visible()`
  - `toggle_er_diagram_visibility()`
- `render.rs` 也是根据 `self.show_er_diagram` 决定是否渲染 ER 分栏

当前代码中已经不再保留 `ERDiagramState.show`。

判断：

- `show_er_diagram` 是当前实际权威源
- 旧 `ERDiagramState.show` 已确认为历史残留并移除

### 2. State ownership

当前 `er_diagram_state` 混放了 3 类东西：

1. loaded graph data
   - `tables`
   - `relationships`
2. viewport / interaction state
   - `pan_offset`
   - `zoom`
   - `dragging_table`
   - `selected_table`
3. loading / layout flags
   - `loading`
   - `needs_layout`

判断：

- 这些东西不该再继续被当成“同一种状态”
- 但现在还不适合直接拆代码

下一步应该先在设计上把它们分成：

- `ERLoadedGraph`
- `ERViewportState`
- `ERLoadLifecycle`

### 3. Business-state isolation

当前好的一面：

- `load_er_diagram_data()` 主要只写 `er_diagram_state` 与 `notifications`
- render 中 ER 也是独立 surface

当前风险：

- 打开/关闭 ER 的行为混在中心区布局里，而不是独立 workspace contract
- `selected_table` 这类语义和主业务侧的 `selected_table` 名称相近，后续很容易被误接线

结论：

- ER 目前看起来没有直接污染主查询结果状态
- 但边界不够显式，后续容易出现串语义

## Design Language Audit

### 1. Why it still feels like a side subsystem

证据集中在 [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs)：

- `RenderColors::from_theme()` 维护了一套 ER 自己的颜色系统
- 顶部工具条仍大量使用：
  - `🔄`
  - `⊞`
  - `⛶`
  - `↺`
- 表格卡片、网格背景、关系线、空态和 loading 的视觉规则都在 ER 内部自定义

这意味着：

- ER 并没有复用 Gridix 现有的 shell token
- 它更像“应用里嵌了一个单独画布工具”

当前状态补充：

- 第一波 shared token unification 已落地：card/background、text、border、selection 与基础 toolbar chrome 已回落到 `ThemePreset::colors()` + `egui::Visuals` helper。
- 第二波 token 也已落地：`grid_line`、`relation_line`、`pk/fk`、`table_shadow`、`text_type` 现在仍保留 ER 画布特性，但已经改成从主主题系统派生，而不再是绝对 RGB。

### 2. What should become shared

后续设计上，至少应统一：

- toolbar chrome
- empty/loading typography
- selection/highlight token
- border/shadow/corner radius language
- light/dark theme 下的文本可读性规则

不建议直接做成：

- 完全抹掉 ER 的画布感

正确方向是：

- 保留“图形视图”的独特性
- 但让它在 token 层看起来属于 Gridix，而不是外来子系统

## Current Test Surface

当前测试证据：

- [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs) 里主要有快捷键绑定测试
- 没看到覆盖：
  - 顶层显隐语义
  - 打开/关闭/刷新路径
  - 状态恢复
  - 主题一致性

结论：

- ER 当前测试覆盖明显偏薄
- 这也是为什么下一步必须先做设计和边界审计，而不是直接改视觉

## Current Remaining Order

当前前置材料与前 7 刀都已落地，剩余 open work 应收窄到：

### 1. `l` semantics decision is now closed

- bare `l` 现在打开当前选中表
- `Shift+L` 接管 relayout

### 2. Finalize/FK fallback is now unified

- `finalize_er_diagram_load_if_ready()` 的 ready-stage 决策已经有 focused coverage
- FK 成功但空结果与 FK error 现在都统一延迟到 finalize 再决定是否推断关系

### 3. Add narrow regression coverage

- `ToggleErDiagram` 命令注册/可用性已有 focused coverage
- ER 聚焦状态栏已有 focused coverage
- Help -> Relationships 学习入口保留“自动打开学习示例 ER 图”已有 focused coverage
- close -> focus restore helper 已有 focused coverage
- open / refresh shared load plan 已有 focused coverage
- `ERDiagramResponse -> app-level surface dispatch` 已有 focused coverage
- 打开 / refresh / theme switch / central split 的端到端 UI 组合链路仍需继续补
- fit view
- theme switch
- central split / focus restore

## Long-Term Anchors

已收口到长期账本的 ER 文档现在是：

- [47-er-workspace-and-keyboard-contract.md](./47-er-workspace-and-keyboard-contract.md)
- [48-er-visibility-entry-matrix-and-state-ledger.md](./48-er-visibility-entry-matrix-and-state-ledger.md)
- [49-er-keyboard-flow-graph.md](./49-er-keyboard-flow-graph.md)
- [50-er-token-map.md](./50-er-token-map.md)
- [51-er-visual-layout-readability-standards.md](./51-er-visual-layout-readability-standards.md)

关系说明：

- `44` 负责审计和问题边界
- `47` 负责进入实现前的 workspace / state / keyboard contract
- `48` 负责显隐入口矩阵与字段级状态账本
- `49` 负责键盘流图、焦点进出和命令分层
- `50` 负责视觉 token 对照表
- `51` 负责默认完成态的构图与可读性标准
