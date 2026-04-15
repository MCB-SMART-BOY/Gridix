# ER Visibility Entry Matrix And State Ledger

## Scope

这份文档现在是 ER 主线的显隐入口与状态账本。

目标只有两个：

1. 盘清 `show_er_diagram` 的所有实际入口与关闭路径
2. 盘清 `er_diagram_state` 的字段类别、生产者、修改者与当前风险

## 1. Visibility Entry Matrix

### 1.1 当前实际显隐权威源

当前实际显隐权威源仍然是：

- `DbManagerApp.show_er_diagram`

相关直接消费点：

- [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs)
- [src/app/action/action_system.rs](../../src/app/action/action_system.rs)
- [src/app/input/input_router.rs](../../src/app/input/input_router.rs)

`UiState.show_er_diagram` 与 `SessionState.show_er_diagram` 当前更像镜像/持久化位，而不是运行期 owner。

### 1.2 入口矩阵

| 入口 | 文件 / 函数 | 类型 | 当前写法 | 副作用 | 判断 |
|---|---|---|---|---|---|
| `AppAction::ToggleErDiagram` | [src/app/action/action_system.rs](../../src/app/action/action_system.rs) `ToggleErDiagram` | open/close | `toggle_er_diagram_visibility()` | 进入 `set_er_diagram_visible()`；打开时触发 `load_er_diagram_data()`，并直接切入 `FocusArea::ErDiagram` | 当前最干净的入口；`Ctrl+R` 在 ER 导航态/视口态下也必须仍可路由到这里 |
| `AppAction::FocusErDiagram` | [src/app/action/action_system.rs](../../src/app/action/action_system.rs) `FocusErDiagram` | focus-only | `set_focus_area(FocusArea::ErDiagram)` | 只在 `show_er_diagram == true` 时切入 ER focus；不改显隐，不触发 `load_er_diagram_data()` | 当前最小的显式入焦入口 |
| toolbar / keybinding / workspace fallback | 推测最终都汇到 `AppAction::ToggleErDiagram` | open/close | action 驱动 | 同上 | 方向正确 |
| Help 学习路径：`RunLearningQueryDemo { open_er_diagram: true }` | [src/app/workflow/help.rs](../../src/app/workflow/help.rs) | open | `set_er_diagram_visible_with_notice(true, Silent)` | 打开时仍触发 `load_er_diagram_data()`；保持原有静默语义 | 已收口 |
| Help 学习路径：`ShowLearningErDiagram` | [src/app/workflow/help.rs](../../src/app/workflow/help.rs) | open | `set_er_diagram_visible_with_notice(true, Custom(...))` | 打开时仍触发 `load_er_diagram_data()`；保留学习路径自定义提示 | 已收口 |
| `RouterLocalAction::CloseWorkspaceOverlay` | [src/app/input/input_router.rs](../../src/app/input/input_router.rs) | close | `set_er_diagram_visible_with_notice(false, Silent)` | 进入统一 helper，同时保持原有静默关闭语义 | 已收口 |
| ER 工具栏刷新按钮 | [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs) -> [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs) | reload | `refresh_requested -> load_er_diagram_data()` | 只 reload，不改显隐 | 语义正确 |

### 1.3 当前结论

当前已固定：

- `set_er_diagram_visible_with_notice()` 是运行期唯一显隐 helper
- `FocusErDiagram` 不是显隐入口；它只负责显式聚焦
- 允许 `load_er_diagram_data()` 独立承担 reload
- Help 与 overlay close 的直接布尔写入已经移除
- `ToggleErDiagram` 必须在 ER 自己持有 focus 时继续可用；但 `ShowHistory` 等其他 workspace overlay shortcut 仍保持原有 gate

### 1.4 已落地的第一刀

当前第一刀只解决“显隐入口收口”，没有扩到键盘 owner、lifecycle 或 token：

1. 运行期显隐路径统一经过 `set_er_diagram_visible_with_notice()`
2. 默认 toggle 仍保留通用提示：
   - `ER 关系图已打开`
   - `ER 关系图已关闭`
3. Help 学习查询路径保持静默打开
4. Help 显式“打开学习 ER 图”路径仍保留自定义提示
5. overlay fallback 关闭仍保持静默

## 2. ER State Ledger

### 2.1 字段分类表

| 字段 | 所在文件 | 类别 | 生产者 | 修改者 | 消费者 | 风险 | 备注 |
|---|---|---|---|---|---|---|---|
| `tables` | [src/ui/components/er_diagram/state.rs](../../src/ui/components/er_diagram/state.rs) | loaded graph | `load_er_diagram_data()` 建立空表骨架 | `clear()`, `set_tables()`, `handle_er_table_columns_fetched()`, `grid_layout()`, `force_directed_layout()`, 拖拽 | `render.rs` 绘制表卡片与关系 | 高 | 同时承载数据和位置 |
| `relationships` | 同上 | loaded graph | `handle_foreign_keys_fetched()` 或 `infer_relationships_from_columns()` | `clear()`, `set_relationships()`, `handle_foreign_keys_fetched()`, `handle_er_table_columns_fetched()` | `render.rs` 绘制关系线 | 中高 | 可能先由 FK 结果写入，也可能由推断回填 |
| `pan_offset` | 同上 | viewport | `new()` 默认值 | `handle_interaction()`, `reset_view()`, `fit_to_view()` | `render.rs` 计算屏幕坐标 | 中 | `clear()` 不会重置，当前语义是“reload 保留视图” |
| `zoom` | 同上 | viewport | `new()` 设为 `1.0` | `zoom_by()`, `reset_view()`, `fit_to_view()` | 工具栏缩放显示、绘制坐标计算 | 中 | `clear()` 不会重置 |
| `dragging_table` / `drag_start` | 同上 | interaction | `start_drag()` | `update_drag()`, `end_drag()`, `clear()` | `handle_interaction()` | 低 | 局部交互态 |
| `selected_table` | 同上 | selection | `start_drag()`、点击/拖拽交互 | `start_drag()`, `select_table()`, `ensure_selection()`, `clear()` | `render.rs` 选中态 | 高 | 与 app 层 `selected_table` 同名异义 |
| `pending_selection_reveal` | 同上 | selection/viewport bridge | `select_table()`, `ensure_selection()` | `select_table()`, `ensure_selection()`, `reveal_selected_table_in_view()`, `clear()` | `render.rs` 下一帧视口修正 | 中 | 只桥接“选中项需要回到视口内”，不改变 app 层当前表 |
| `interaction_mode` | 同上 | local interaction mode | `new()` / `clear()` | `toggle_interaction_mode()`, `exit_viewport_mode()`, `clear()` | `input_router.rs` 的 `FocusScope::ErDiagram(Navigation/Viewport)`；`render.rs` 的键盘解释 | 中 | 只决定 ER 局部键盘语义，不承担显隐或业务同步；`clear()` 会重置回 `Navigation` |
| `loading` | 同上 | lifecycle | `begin_loading()` 置 `true` | `begin_loading()`, `mark_foreign_keys_resolved()`, `mark_table_request_resolved()` | `render.rs` loading 文案 | 中 | 现在等待“FK 请求结束 + 所有表列请求结束” |
| `needs_layout` | 同上 | lifecycle/layout flag | `new()` / `clear()` | `clear()`, `set_tables()`, `load_er_diagram_data()` | 未来布局路径 | 中 | 当前责任不清晰，`load_er_diagram_data()` 末尾直接置 `false` |
| `pending_column_tables` | 同上 | lifecycle | `begin_loading()` | `begin_loading()`, `mark_table_request_resolved()`, `clear()` | `loading` 计算 | 中 | 内部追踪，避免再拿“列是否为空”猜请求是否完成 |
| `foreign_key_columns` | 同上 | merge cache | `set_foreign_key_columns()` | `set_foreign_key_columns()`, `clear()` | 现有列回填、晚到列投影 | 中 | 内部缓存，避免 FK/列回包顺序决定最终徽标 |
| `foreign_keys_resolved` | 同上 | lifecycle | `begin_loading()` | `set_foreign_key_columns()`, `mark_foreign_keys_resolved()`, `clear()` | `loading` 计算 | 中 | 表示 FK 请求已结束（成功或失败） |

### 2.2 隐式语义

当前从代码可以直接推导出的隐式语义：

1. `clear()` 只清图数据和选择，不清 `pan_offset / zoom / loading`
   - 当前等价语义是：reload 会清内容，但尽量保留视图位置
   - `pending_selection_reveal` 也会清空，避免旧选中项把新一轮加载后的视口再拉回去

2. `load_er_diagram_data()` 会先清空 `tables / relationships / selection`，再立即设置 `loading = true`

3. `load_er_diagram_data()` 结束前会把 `needs_layout = false`
   - 这说明 `needs_layout` 当前并不是真正的“待布局请求队列”

4. `selected_table` 与 `pan_offset` 当前不直接耦合
   - 只有 `pending_selection_reveal == true` 时，render 才会在拿到 canvas 尺寸后调用 `reveal_selected_table_in_view()` 修正视口

5. `interaction_mode` 是 ER 的局部键盘解释状态
   - 当前只有 `Navigation` 与 `Viewport` 两种
   - `v` 在两者间切换
   - 视口模式内的 `Esc` 只退出到浏览模式，不改变 `show_er_diagram`
   - reload / `clear()` 会保守重置回 `Navigation`

## 3. Confirmed Risks

### 3.1 Loading completion is no longer tied to FK-first completion

当前第一刀已经把 `loading` 从“FK 回包先到就结束”改成了双条件：

- FK 请求已结束
- 所有表列请求已结束

证据：

- `ERDiagramState::begin_loading()`
- `ERDiagramState::mark_foreign_keys_resolved()`
- `ERDiagramState::mark_table_request_resolved()`

当前结论：

- `loading` 不再被单个 FK 回包提前关闭
- `ready` 现在只在 finalize 阶段统一处理，而不是在 FK 回包阶段提前发完成提示

### 3.2 Foreign-key badge population is order-sensitive

当前第一刀已经消除了最直接的顺序污染。

证据：

1. FK 回包现在先落入 `foreign_key_columns`
2. 现有列通过 `set_foreign_key_columns()` 回填
3. 晚到列在 `handle_er_table_columns_fetched()` 中通过 `is_foreign_key_column()` 投影

推论：

- FK 结果先到、列结果后到时，不会再把外键徽标冲掉
- 关系线、layout 与最终 ready 通知现在统一受 finalize 语义控制

这里先记为：

- **已修复顺序污染，finalize 语义已开始集中**

本轮已修 FK/列顺序污染，不继续扩到 keyboard 或 token。

### 3.3 Ready notification is now emitted only after ER is actually ready

此前通知路径的问题是：

1. FK 回包阶段会直接提示
2. 列回包阶段可能还没结束
3. 用户先看到“ER图: X 张表, Y 个关系”，但图仍处于 loading

当前 finalize / FK fallback 收口后：

- `handle_foreign_keys_fetched()` 不再直接发 ready 提示
- FK 成功但返回空列表时，不再提前推断关系
- `finalize_er_diagram_load_if_ready()` 统一承担：
  - layout
  - 空关系时的推断
  - 最终 ready 提示

当前结论：

- ready 提示不再早于真正 ready
- FK 显式关系 / 推断关系 / 空关系 3 种提示都在同一阶段决定
- 空 FK 结果与 FK error 现在都统一走 finalize fallback
- 下一步 open 点已经收窄到快捷键 contract，而不是继续修 finalize 时机

### 3.3 Selection naming can mislead future integrations

`ERDiagramState.selected_table` 与 app 层 `selected_table` 名称相同但语义不同。

当前结论：

- 这不是已复现 bug
- 但如果后续把 `Enter`、主表同步或 sidebar 联动接进 ER，这会成为高风险耦合点

## 4. Design Consequences

基于当前矩阵，后续实现前必须先固定这 3 条：

1. `show_er_diagram` 已只通过统一 helper 改写，重复显隐字段也已移除
2. `loading` 已从 FK-first completion 脱钩，finalize / FK fallback contract 也已统一
3. `selected_table` 需要先决定是否与主业务表选择同步，以及同步方向
4. `interaction_mode` 只应继续影响 ER 局部 scope 分层，不应反向驱动 app-level focus、显隐或主业务表选择

## 5. Next Non-Implementation Tasks

接下来先按这个顺序推进：

1. 补顶层 regression coverage
   - `ToggleErDiagram` 命令注册/可用性已锁住
   - Help 路径仍保留“自动打开学习示例 ER 图”已锁住
   - close -> focus restore helper 已锁住
   - open / refresh shared load plan 已锁住
   - `ERDiagramResponse -> app-level surface dispatch` 已锁住
   - `FocusErDiagram` 已锁住
   - `v` 进入视口模式、`Esc` 退出视口模式、视口模式内 `h/j/k/l` 平移已锁住
   - 视口模式内 `q` 关闭、`r / Shift+L / f` 仍保持可用，以及 reload 会把 `interaction_mode` 重置回 `Navigation` 也已锁住
   - theme switch render 不改 ER 局部状态也已锁住
   - `FocusErDiagram -> ToggleErDiagram` 的 app-level focus restore / fallback 组合也已锁住
   - 当前真正剩余的顶层主线已收窄到几何邻接导航的启发式观察；独立 detail mode 当前已冻结，不进入实现

2. 再考虑更强导航
   - 关系邻接第一刀已进入实现：`Shift+J / Shift+K` 现已按稳定全局表顺序在当前表的关联集合内移动
   - 几何邻接也已进入实现：`Shift+Left / Shift+Down / Shift+Up / Shift+Right` 现已按当前布局方向选择最近邻
   - 当前仍没有独立 detail mode，并且该方向已冻结，不进入近端实现
