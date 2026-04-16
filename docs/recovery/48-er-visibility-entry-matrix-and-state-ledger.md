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
| `tables` | [src/ui/components/er_diagram/state.rs](../../src/ui/components/er_diagram/state.rs) | loaded graph | `load_er_diagram_data()` 建立空表骨架 | `clear()`, `set_tables()`, `handle_er_table_columns_fetched()`, `grid_layout()`, `relationship_seeded_layout()`, `force_directed_layout()`, 拖拽 | `render.rs` 绘制表卡片与关系 | 高 | 同时承载数据和位置；默认完成态布局现已走“先 grid skeleton，再按关系决定是否 relationship-seeded refine”，且关系种子现在会先把断开的关系簇和孤立表拆成独立组件，并优先让较大的关系主簇占据左上锚点；普通关系布局里的层级种子也已开始直接吃 `ERGraph.layer_hint`，窄父层带会围绕更宽的子层带居中，而不再继续把主簇压成左对齐细竖带；在 pack 完相关组件后，纯孤立组件还会进入显式的右侧边缘区，而不再继续漂在主簇周围的大块空白里 |
| `relationships` | 同上 | loaded graph | `handle_foreign_keys_fetched()` 或 `infer_relationships_from_columns()` | `clear()`, `set_relationships()`, `handle_foreign_keys_fetched()`, `handle_er_table_columns_fetched()` | `render.rs` 绘制关系线 | 中高 | 可能先由 FK 结果写入，也可能由推断回填；每条关系现在都显式带 `origin = Explicit / Inferred`，供布局与视图密度分层消费 |
| `pan_offset` | 同上 | viewport | `new()` 默认值 | `handle_interaction()`, `reset_view()`, `fit_to_view()` | `render.rs` 计算屏幕坐标 | 中 | `clear()` 不会重置，当前语义是“reload 保留视图” |
| `zoom` | 同上 | viewport | `new()` 设为 `1.0` | `zoom_by()`, `reset_view()`, `fit_to_view()` | 工具栏缩放显示、绘制坐标计算 | 中 | `clear()` 不会重置；当前最小缩放已放宽到 `0.1`，因此默认打开 ER 的一次性 fit 可以低于旧 `25%` 下限 |
| `dragging_table` / `drag_start` | 同上 | interaction | `start_drag()` | `update_drag()`, `end_drag()`, `clear()` | `handle_interaction()` | 低 | 局部交互态 |
| `selected_table` | 同上 | selection | `start_drag()`、点击/拖拽交互 | `start_drag()`, `select_table()`, `ensure_selection()`, `clear()` | `render.rs` 选中态 | 高 | 与 app 层 `selected_table` 同名异义 |
| `pending_selection_reveal` | 同上 | selection/viewport bridge | `select_table()`, `ensure_selection()` | `select_table()`, `ensure_selection()`, `reveal_selected_table_in_view()`, `clear()` | `render.rs` 下一帧视口修正 | 中 | 只桥接“选中项需要回到视口内”，不改变 app 层当前表 |
| `pending_fit_to_view` | 同上 | lifecycle/viewport bridge | `set_er_diagram_visible_with_notice()` 打开分支 | `request_fit_to_view()`, `consume_pending_fit_to_view()`, `clear()` | `render.rs` 下一帧按真实画布尺寸执行一次 `fit_to_view()` | 中 | 只用于“从隐藏态打开 ER 后的一次性自动 fit”，不改变 reload/refresh 的保留视图合同 |
| `pending_layout_restore` | 同上 | lifecycle/layout bridge | `load_er_diagram_data()` 在 reload 前抓取旧 `tables` 位置快照 | `set_pending_layout_restore()`, `begin_loading()`, `restore_layout_snapshot_if_exact_match()`, `restore_layout_snapshot_for_matching_tables()`, `clear()` | `finalize_er_diagram_load_if_ready()`、`select_ready_state_er_layout_strategy()`、`apply_ready_state_er_diagram_layout()` | 中 | 只服务于“同表名集合 reload 恢复旧布局”与“表集变化时的交集保位”；当前已被 runtime 显式提升成 `StableIncremental` ready-state 策略入口，但 ownership 仍留在 state/runtime，不进入 `ERGraph`；新增表必须继续使用本轮策略布局位置，并在必要时局部避让已恢复旧表；若新增表和这些旧表存在关系，则还应先贴近关系邻居；若这些关系还具备明确父/子方向，则局部插入还应优先遵守上下层级语义，避免把子表甩到父表左右两侧；若新增表同时连到已恢复父/子层，则局部插入还应优先把桥接表留在两层之间，而不是退化成“直接掉到子层下面” |
| `ERGraph`（transient） | [src/ui/components/er_diagram/graph.rs](../../src/ui/components/er_diagram/graph.rs) | semantic graph | `build_er_graph()` | `build_er_graph()` 内部一次性构建 | `analyze_er_graph()`、`select_er_layout_strategy()`、ready-state structural strategy 选择链 | 中 | 不进入 `DbManagerApp` 或 `ERDiagramState` 的长期字段；当前只作为 finalize / relayout 的语义输入和 `summary` 兼容层，不能反向承担 viewport、selection、snapshot 或 app-level owner 语义；`Phase 2B` 后，默认完成态布局与 render-side strategy label 都应先经过 `ERGraph -> selector`，而不是继续依赖隐式 summary 内联判断；当前纯语义 selector 已能显式分流 `DenseGraph`，因此高密度单主簇图不再继续复用普通 `Relation` 完成态；而 `layout.rs` 的 dense path 现在也开始直接消费 `ERGraph` 的 node role 与 `layer_hint` 语义，把高密度图先排成 `root/core/leaf` seed band，并在 bridge-heavy 图里进一步拆成多层 core band，再在相邻带之间先做一轮 barycenter 排序后再进入 refine；`render.rs` 的 edge routing 现在也继续吃这套 dense-layout 几何结果：不仅上下堆叠表会优先走 top/bottom 锚点，共享同一走廊的 horizontal/vertical 正交边与共享同一 L 形肘点的 mixed edge 也都会参与 lane 分配；`StableIncremental` 这类 snapshot-aware 决策仍应留在 runtime selector，而不是回塞进纯语义图层 |
| `interaction_mode` | 同上 | local interaction mode | `new()` / `clear()` | `toggle_interaction_mode()`, `exit_viewport_mode()`, `clear()` | `input_router.rs` 的 `FocusScope::ErDiagram(Navigation/Viewport)`；`render.rs` 的键盘解释 | 中 | 只决定 ER 局部键盘语义，不承担显隐或业务同步；`clear()` 会重置回 `Navigation` |
| `card_display_mode` | 同上 | local presentation mode | `new()` | `toggle_card_display_mode()`, `clear()` | `render.rs` 表卡尺寸与列可见性 | 中 | 只决定 `Standard / KeysOnly` 两档密度，不改 loaded graph |
| `edge_display_mode` | 同上 | local presentation mode | `new()` | `cycle_edge_display_mode()`, `clear()` | `render.rs` 边可见性 / 降噪策略 | 中 | 只决定 `All / Focus / ExplicitOnly` 三档视图密度，不改 loaded graph |
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
   - `pending_layout_restore` 默认也会被清空；只有 `begin_loading()` 明确带着 staged snapshot 进入新一轮加载时才会保留

2. `load_er_diagram_data()` 在真实 reload 分支里会先抓取当前 `tables` 的位置快照，再通过 `begin_loading()` 清空 `tables / relationships / selection` 并立即设置 `loading = true`
   - 加载中的表骨架仍先走 `grid_layout()`
   - 这个快照现在既支持“表名集合完全一致”的直接恢复，也支持“表集变化但存在交集”时的局部保位；新增表仍由当前策略布局生成新位置，并在必要时再做一次局部避让；若它们和已恢复旧表存在关系，则会先按这些关系邻居选择局部锚点；若关系方向明确，则会进一步把“父在上、子在下”的层级语义带进局部插入

3. `load_er_diagram_data()` 结束前会把 `needs_layout = false`
   - 这说明 `needs_layout` 当前并不是真正的“待布局请求队列”
   - 默认完成态布局现在统一由 `finalize_er_diagram_load_if_ready()` 决定：无关系保持 grid，有关系则改走关系层级种子 + force-directed refine
   - 最新 contract 是：关系种子会先按断开的关系簇 / 孤立表拆组件，再分别做层级初始化，并在组件足够多时按目标行宽换排，避免无关组件共用同一条横向种子带或无限向右展开；pack 完相关组件后，纯孤立组件还会被推进显式的右侧边缘区，保证主簇继续占据主视觉锚点

4. `selected_table` 与 `pan_offset` 当前不直接耦合
   - 只有 `pending_selection_reveal == true` 时，render 才会在拿到 canvas 尺寸后调用 `reveal_selected_table_in_view()` 修正视口

5. 从隐藏态打开 ER 时会额外挂起一次 `pending_fit_to_view`
   - 这条 fit 在下一帧拿到真实画布尺寸后才执行
   - 它可以把 `zoom` 降到旧 `25%` 下限以下，以确保默认完成态完整回到可见区
   - reload/refresh 不会自动重置这条标记，因此仍保持“尽量保留当前视图”的现行合同

6. 从第二阶段前两刀开始，reload 还额外具备“位置快照桥接”语义
   - 若 finalize 时当前表名集合与快照完全一致，则优先恢复旧节点位置
   - 若只存在部分名称交集，则会先跑一遍新的策略布局，再把交集表恢复回旧位置
   - 新增表继续保留这轮策略布局位置；若它们与已恢复的旧表重叠，则再做一次局部避让
   - 若新增表与已恢复旧表存在关系，则优先贴近关系邻居，再做最后一层局部避让
   - 若这些关系方向明确，则优先让新子表落在父表下方，让被子表引用的新父表落在子表上方
   - 移除的旧表则随快照一起丢弃
   - 当前它仍不是用户显式可控的 `keep layout` 开关，只是内部的 incremental-stability contract

7. `interaction_mode` 是 ER 的局部键盘解释状态
   - 当前只有 `Navigation` 与 `Viewport` 两种
   - `v` 在两者间切换
   - 视口模式内的 `Esc` 只退出到浏览模式，不改变 `show_er_diagram`

7. `ERGraphSummary` 与 `ERLayoutStrategy` 现在是完成态布局的中间层
   - 它们不持久化在 `ERDiagramState` 内
   - 但默认完成态和手动 relayout 都必须先经过这层语义判断，不能再直接从 `tables + relationships` 跳到单一布局函数
   - reload / `clear()` 会保守重置回 `Navigation`

8. `StableIncremental` 现在是 runtime 级 ready-state 策略，而不是 finalize 里的隐式 if/else
   - `ERGraph -> select_er_layout_strategy()` 仍只回答纯结构问题
   - snapshot overlap / exact restore / partial restore 仍由 state/runtime 决定
   - 这样可以保持语义图层纯粹，同时把增量布局入口从“散落逻辑”收口成可测试 contract

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
