# Changelog

All notable changes to this project are documented in this file.  
本文件记录项目的重要变更。

## [Unreleased]

## [6.1.0]

### Changed
- Added a first-error auto-reveal contract to `FormDialogShell` and wired `CreateUserDialog` onto it, so validation failures now scroll the first invalid field or privileges block back into view instead of leaving only a generic error line below the fold.
  为 `FormDialogShell` 增加了首个错误字段的自动显露 contract，并让 `CreateUserDialog` 先接入这条路径，因此校验失败时现在会把第一个出错字段或权限块滚回可见区，而不再只是在折叠线下方留一条通用错误文本。
- Added an explicit `DenseGraph` ER completed-layout strategy for high-density connected schemas, so dense single-cluster diagrams no longer reuse the ordinary `Relation` path and now dispatch through a stronger high-density refine contract instead.
  为高密度单主簇 schema 新增了显式的 `DenseGraph` ER 完成态策略：这类高密度关系图不再继续复用普通 `Relation` 路径，而是通过单独的高密度 refine contract 完成布局分发。
- Tightened the new `DenseGraph` path so it no longer starts from the same generic grid skeleton: dense schemas now seed a compact `root/core/leaf` band from `ERGraph` node roles before the refine pass, keeping high-density diagrams visually distinct from ordinary relation layouts.
  收紧了新的 `DenseGraph` 路径，使其不再从同一套通用 grid 骨架起步：高密度 schema 现在会先基于 `ERGraph` 的节点角色生成紧凑的 `root/core/leaf` 带状 seed，再进入 refine，因此高密度关系图终于在视觉上区别于普通关系布局。
- Refined DenseGraph intra-band ordering so high-density ER diagrams no longer keep `root/core/leaf` rows in arbitrary name/degree order: adjacent bands now use neighbor barycenter sweeps before refine, reducing avoidable crossings inside dense single-cluster diagrams.
  继续收紧了 DenseGraph 的带内排序：高密度 ER 图的 `root/core/leaf` 三层带不再继续停留在按名称/度数的粗排上；相邻带在进入 refine 前会先做邻居重心排序，从而减少高密度单主簇图里的可避免交叉。
- Refined DenseGraph again so bridge-heavy dense schemas no longer collapse every `Bridge / Hub` node into a single horizontal core strip: the dense seed now uses `ERGraph.layer_hint` to split the middle mass into multiple core bands before barycenter ordering and refine.
  继续收紧了 DenseGraph：bridge-heavy 的高密度 schema 不再把所有 `Bridge / Hub` 节点继续压成单一横向 core strip；DenseGraph seed 现在会在 barycenter 排序和 refine 之前，先利用 `ERGraph.layer_hint` 把中间层拆成多层 core band。
- Refined ER edge routing so stacked tables no longer default to left/right anchors: geometry-aware connectors now prefer top/bottom anchors and a vertical orthogonal route when vertical separation dominates, which reduces long horizontal doglegs in dense layered diagrams.
  继续收紧了 ER 边路由：上下堆叠的表不再默认继续走左右锚点；当垂直分离占优时，geometry-aware connector 现在会优先走 top/bottom 锚点和纵向正交路径，从而减少高密度分层图里的长横折线噪声。
- Refined ER edge routing again so parallel orthogonal relationships no longer collapse onto a single shared corridor: when multiple visible edges overlap on the same `mid_x / mid_y` route, Gridix now assigns stable lane offsets instead of drawing them directly on top of each other.
  继续收紧了 ER 边路由：平行的正交关系线不再继续压到同一条共享走廊上；当多条可见边重叠在同一条 `mid_x / mid_y` 路径附近时，Gridix 现在会为它们分配稳定的 lane offset，而不是直接叠画成一条线。
- Refined ER edge routing for mixed orthogonal connectors too: routes that share the same L-shaped elbow column now receive stable lane offsets instead of collapsing onto one vertical elbow, improving dense edge readability when several mixed relationships converge on the same target column.
  继续收紧了 mixed 正交边的 ER 路由：当多条边共享同一根 L 形肘点列时，Gridix 现在也会为这些 mixed route 分配稳定的 lane offset，而不再继续压成同一根竖肘线，从而提升多条关系汇入同一目标列时的可读性。
- Refined ordinary relation-first ER layouts so they now consume `ERGraph.layer_hint` directly and center narrower parent rows over wider child bands, instead of rebuilding a separate longest-path tower that tends to compress learning-sample-style main clusters into a thin left-aligned column.
  收紧了普通关系优先的 ER 布局：它现在会直接消费 `ERGraph.layer_hint`，并把更窄的父层带围绕更宽的子层带居中，而不再继续重算一套独立的最长路径高塔，避免学习样例风格的主关系簇继续被压成左对齐的细长竖带。
- Refined ordinary relation/component packing again so pure isolated tables no longer just sit “somewhere away from” the main cluster: after related components are packed, isolated components now move into an explicit right-edge zone, preserving the primary cluster as the main visual anchor instead of sharing the same open whitespace.
  继续收紧了普通 `Relation / Component` 的组件 pack：纯孤立表不再只是“离主簇远一点”；在 pack 完相关组件之后，孤立组件现在会进入显式的右侧边缘区，从而让主关系簇继续保住主视觉锚点，而不是继续和孤立表共享同一片大片留白。

## [6.0.0]

### Changed
- Preserve ER layout stability across reloads: exact same-table reloads still restore the full previous layout directly, and partial-overlap reloads now keep matching tables in place after the new `Grid / Relation / Component` completed layout places any newly added tables, while also nudging those new tables away from restored old ones when needed.
  保持 ER 在 reload 期间的布局稳定性：完全相同表集的 reload 仍会直接恢复整张旧布局，而表集只部分重合的 reload 现在也会在新的 `Grid / Relation / Component` 完成态布局放置新增表之后，把交集表恢复回原位置；若这些新增表与已恢复的旧表重叠，还会继续做局部避让。若新增表本身和这些已恢复旧表存在关系，则它们会优先贴近关系邻居，再做最后一层局部避让；若这些关系方向明确，则新增子表会优先落在父表下方，而新增父表会优先落在子表上方；若新增表同时桥接已恢复父/子层，则现在也会优先留在两层之间，而不是继续被挤到整个子层下面。
- Rebuilt the ER diagram generation and rendering pipeline around an explicit graph-summary and strategy-selection stage: finalize and manual relayout now choose between `Grid / Relation / Component` completed layouts, each relationship now carries explicit-vs-inferred origin, and the ER surface now exposes `All / Focus / ExplicitOnly` edge modes plus `Standard / KeysOnly` card density while redrawing cards, toolbar, and routed edges to match Gridix's workspace design language.
  围绕显式的“语义图摘要 + 策略选择”阶段重写了 ER 图的生成与渲染管线：默认完成态与手动 relayout 现在都会在 `Grid / Relation / Component` 三种布局之间做选择，每条关系也都显式区分 `Explicit / Inferred` 来源；ER 界面同时新增了 `All / Focus / ExplicitOnly` 边模式与 `Standard / KeysOnly` 表卡密度，并重画了表卡、工具条和边路由，使其更贴合 Gridix 的工作台式设计语言。
- Let ER's automatic open-time fit-to-view zoom below the old `25%` floor when the default completed layout still does not fit inside the pane, so opening the learning-sample diagram now keeps the whole cluster visible instead of clipping the bottom tables; manual zooming now stays monotonic from those lower fit-driven zoom levels.
  让 ER 在默认打开后的自动 `fit_to_view()` 可以突破旧的 `25%` 下限：当默认完成态仍装不进面板时，学习样例 ER 现在会继续缩小直到整簇保持可见，而不再把底部表裁到视口外；同时从这类更低缩放继续“缩小视图”时，也不会再反向跳回更大的缩放值。
- Fixed ER viewport-mode toggling so `v` now switches `Navigation / Viewport` exactly once per key press when the diagram owns focus; the render pass no longer re-consumes the same shortcut after the input router has already handled it.
  修复了 ER 视口模式切换：当 ER 持有焦点时，`v` 现在每次按键只会在 `Navigation / Viewport` 之间切换一次；render 阶段不再在 input router 已经处理之后重复消费同一个快捷键。
- Tightened ER relation-first layout to respect real card sizes: hierarchical seeding now spaces levels and siblings by actual table dimensions, and the force-directed refine now separates tables by center distance plus card-size clearance instead of treating every table like the same fixed box.
  收紧了 ER 的关系优先布局，让它开始尊重真实表卡尺寸：层级种子现在会按实际表卡宽高安排层间和同层间距，而 force-directed refine 也改为基于表卡中心和尺寸留白做分离，不再把所有表都当成同一个固定盒子。
- Tightened the ER relation-first layout so same-level sibling tables no longer inherit raw input order: relationship-seeded layout now initializes levels deterministically and reorders siblings by connected-neighbor barycenter, falling back to name order only when the relation signal ties.
  收紧了 ER 的关系优先布局：同层兄弟表不再直接继承原始输入顺序；关系种子布局现在会先做稳定初始化，再按已知关系邻居的重心调整兄弟顺序，只有在关系信号打平时才回退到名称顺序。
- Split ER relation-first seeding by disconnected graph components before the global force-directed refine, so unrelated clusters and isolated tables no longer start packed into the same seeded horizontal band.
  在进入全局 force-directed refine 之前，先按断开的图组件拆分 ER 的关系优先种子，因此互不相关的关系簇和孤立表不再从同一条横向种子带挤在一起起步。
- Made multi-component ER relation seeding wrap across rows instead of only stretching further right, so schemas with many disconnected clusters use vertical space earlier instead of forming a single extra-wide seeded strip.
  让多组件 ER 关系种子会按行换排，而不是继续只向右拉长；因此包含许多断开关系簇的 schema 会更早利用垂直空间，而不是形成一条过宽的单行种子带。
- Made component-aware ER relation seeding anchor larger relationship clusters before tiny isolated tables, so the main cluster now claims the top-left seed region instead of being displaced merely by lexicographic component order.
  调整了组件化 ER 关系种子：较大的关系主簇现在会先占据左上种子区域，小型孤立表不再仅因组件名称排序更靠前就把主簇挤到后排。

## [5.0.0]

### Changed
- Made ER's default completed layout relation-first: loading still starts from a stable grid skeleton, but once relationships are finalized the diagram now keeps the grid only for empty graphs and otherwise switches to a relationship-seeded force-directed layout; manual relayout now follows the same relationship-first path instead of using a pure spring pass from the current grid positions.
  将 ER 的默认完成态布局改成了关系优先：加载中仍先从稳定的网格骨架起步，但在关系 finalize 之后，空关系图仍保持网格，而存在关系时则会切到“关系层级种子 + force-directed refine”的布局；手动 relayout 现在也会复用同一条关系优先路径，而不是继续只从当前网格位置做一次纯弹簧微调。
- Rebound focused ER browsing so bare `h / l` no longer leave the diagram or immediately query the selected table: they now stay inside ER as left/right geometric navigation, while `Enter / Right` remain the explicit “open selected table” action and `Esc / Left` remain the explicit return-to-workspace path.
  重新绑定了聚焦 ER 时的默认浏览语义：bare `h / l` 不再离开 ER 或立即查询当前表，而是留在 ER 内部承担左右几何邻接导航；`Enter / Right` 继续作为显式“打开当前表”动作，`Esc / Left` 则继续作为显式返回主工作区路径。
- Made DataGrid result-table headers use explicit theme-aware text colors for unfocused columns, so dark themes no longer make most column names fade out while only the active header remains legible.
  让 DataGrid 结果表格的非焦点列头也显式使用主题感知文字色，因此暗色主题下不再出现“多数列名淡到几乎看不见，只剩当前焦点列清晰”的退化现象。
- `Ctrl+R` now focuses ER immediately when opening the diagram, so `h / j / k / l` no longer stay captured by `DataGrid` in result-table workflows; `Alt+R` remains available as an explicit re-focus action when ER is already visible but focus has moved away.
  `Ctrl+R` 现在在打开 ER 时会直接把焦点切入 ER，因此结果表格工作流里 `h / j / k / l` 不会再继续被 `DataGrid` 抢走；`Alt+R` 仍保留为 ER 已可见但焦点已离开时的显式回焦动作。
- Tightened the `DdlDialog` create-table window to use a compact workspace profile and more conservative column-list / SQL-preview heights, so the footer stays visible again in narrow viewports instead of being clipped below the window.
  收紧了 `DdlDialog` 创建表窗口的默认 profile，并把列定义区 / SQL 预览区的高度改成更保守的自适应值，因此窄视口下 footer 重新保持可见，不再被裁到窗口外。
- Restored reliable mouse interaction for sidebar connection headers by separating the name/toggle click surface from the right-side destructive buttons and keeping those mouse entrypoints on the same `SidebarDeleteTarget -> DeleteConfirm` chain as keyboard `d`.
  恢复了侧边栏连接头部的鼠标交互稳定性：连接名/折叠点击面现在与右侧危险按钮分离，而这些鼠标入口也继续和键盘 `d` 共享同一条 `SidebarDeleteTarget -> DeleteConfirm` 删除链。
- Restored interaction-only chrome for the main toolbar and ER toolbar buttons: inactive triggers are transparent again, while hover/focus still reveal button feedback and selected ER viewport mode remains visibly active.
  恢复主 toolbar 和 ER toolbar 按钮仅在交互态显示 chrome：未激活的 trigger 重新回到透明默认态，而 hover / focus 仍会显示按钮反馈，ER 的视口模式按钮也继续保留可见的选中态。
- Added additive ER relationship-adjacency navigation on `Shift+J / Shift+K`: focused ER browsing still keeps `j / k` as the stable linear table order, while the new commands now move only within the current table’s related-table set using deterministic global table ordering instead of layout geometry.
  为 ER 新增了 additive 的关系邻接导航 `Shift+J / Shift+K`：聚焦 ER 时，`j / k` 仍保持稳定线性选表，而这组新命令现在只会在当前表的关联表集合内移动，并按确定性的全局表顺序解释，而不依赖当前布局几何。
- Added additive ER geometry-adjacency navigation on `Shift+Left / Shift+Down / Shift+Up / Shift+Right`: focused ER browsing now keeps `j / k` as linear order and `Shift+J / Shift+K` as relationship order, while the new directional commands choose the nearest geometric neighbor from current card positions without syncing the main workspace table until the user explicitly opens it.
  为 ER 新增了 additive 的几何邻接导航 `Shift+Left / Shift+Down / Shift+Up / Shift+Right`：聚焦 ER 时，`j / k` 继续保持线性选表，`Shift+J / Shift+K` 继续保持关系邻接，而这组新的方向命令会按当前表卡位置选择最近的几何邻居；在用户显式打开之前，它不会直接同步主工作区当前表。
- Split the ER keyboard flow into explicit navigation and viewport sub-scopes, added `v` to toggle a local viewport mode, and moved `h/j/k/l` panning behind that mode so the default focused ER grammar remains relationship browsing while `Esc` in viewport mode now returns to navigation instead of leaving ER.
  将 ER 键盘流拆成显式的浏览与视口两个子作用域，并新增 `v` 切换本地视口模式；`h/j/k/l` 的平移也被收回到该模式下，因此默认聚焦 ER 时的语法继续保持为关系浏览，而视口模式里的 `Esc` 现在只会退出回浏览态，不再直接离开 ER。
- Rolled the “About Gridix” dialog back toward the lighter older style: it now uses a centered brand header, one manifesto card, and a lighter project overview instead of the heavier hero-plus-facts-strip composition, while keeping the same standard dialog shell and close shortcuts.
  将“关于 Gridix”界面回摆到更轻的旧版主线：现在使用居中的品牌头、一张 manifesto 卡和更轻量的项目速览，而不再是之前那种偏厚重的 hero 加 facts strip 组合；同时保持现有 standard dialog shell 和关闭快捷键不变。
- Compressed the stacked workspace dialog headers in KeyBindings and Help into a shared compact two-block layout: at normal widths they now render side by side, and only fall back to vertical stacking when the window is genuinely narrow.
  将快捷键设置和帮助页里原先纵向堆叠的 workspace dialog 顶部区域压缩为共享的紧凑双区块布局：在正常宽度下它们现在会并排显示，只有窗口真的变窄时才回退到纵向堆叠。
- Reclassified the toolbar chooser/theme local shortcuts under dedicated dialog scopes in the keybindings UI, so `toolbar.menu.*` and `toolbar.theme.*` no longer appear mixed into the base `toolbar` node.
  在快捷键设置界面中，将 toolbar chooser/theme 的局部快捷键重新归类到专用 dialog 作用域，因此 `toolbar.menu.*` 与 `toolbar.theme.*` 不再继续混在基础 `toolbar` 节点里显示。
- Added dedicated `Alt+A` / `Alt+N` action-backed shortcuts for the toolbar action/create menus, updated their tooltips to show those real bindings, and restored visible hover/selected chrome so the two toolbar triggers read as actual buttons instead of anonymous icons.
  为 toolbar 的“操作菜单 / 新建菜单”新增了基于 `Action` 的独立快捷键 `Alt+A` / `Alt+N`，并让 tooltip 改为显示这两个真实绑定；同时恢复这两个 trigger 的 hover / selected 可见态，使它们不再像匿名图标而是明确可点击按钮。
- Moved the toolbar action/create chooser windows onto resizable workspace-style shells, compressed their stacked header hints into a single compact two-block strip, and added `Q` alongside `Esc` for dismiss so they now behave more like the keybindings dialog instead of fixed anonymous popups.
  将 toolbar 的“操作菜单 / 新建菜单”chooser 外窗迁移到可缩放的 workspace 风格壳层，把原先纵向堆叠的顶部提示压缩成单个紧凑双区块，并为关闭补上 `Q` 与 `Esc` 共同生效，因此它们现在更接近快捷键设置对话框的使用方式，而不再像固定尺寸的匿名弹层。
- Upgraded `GridWorkspaceStore` to use active `tab_id` in its workspace identity, so the same table now keeps independent grid drafts and view state in different query tabs instead of sharing one table-scoped workspace.
  将 `GridWorkspaceStore` 升级为把活动 `tab_id` 纳入工作区标识，因此同一张表在不同查询标签页中现在会保留各自独立的表格草稿和视图状态，而不再共享同一个按表建模的工作区。
- Stopped successful grid-save callbacks from clearing whichever table workspace happens to be active when the response returns, and also clear the saved draft state for the original save target so switching back no longer resurrects stale unsaved edits.
  修复表格保存成功回包会误清当前活动工作区的问题，并同步清理原保存目标的持久化草稿状态，因此保存期间切到别的表或标签页后再切回时，不会再复活旧的未保存编辑。
- Moved the welcome setup guide window onto the shared resizable dialog shell and added a single body scroll region with viewport constraints, so long onboarding content no longer pushes the window outside smaller viewports.
  将欢迎页安装引导弹窗迁移到共享的可缩放对话框壳层，并加入单一主体滚动区和视口约束，因此较长的引导内容不再把窗口顶出较小视口。
- Clamped the command palette to the current viewport and replaced its hardcoded content minimum width with responsive shell sizing, so narrow windows no longer let the palette force itself wider than the visible area.
  为命令面板补上当前视口约束，并将原先写死的内容最小宽度改为响应式壳层宽度，因此窄窗口下命令面板不再把自己横向撑出可视区域。
- Added viewport-aware size clamps to the query history panel, so the floating history window now stays within smaller viewports instead of keeping only an unconstrained default size.
  为查询历史面板补上视口感知的尺寸约束，因此这个浮层窗口在较小视口中会保持在可见范围内，而不再只依赖一个无约束的默认大小。
- Cleared the active tab's stored query result when dispatching a new SQL request, so switching away and back during execution no longer rehydrates stale rows from the previous successful run.
  在派发新的 SQL 请求时清理活动标签页上一次保存的查询结果，因此执行期间切换标签再切回时不再把上一次成功结果重新刷回界面。
- Synced query elapsed time back from the active tab when switching tabs, and cleared the tab-local timing snapshot when a new query starts so the SQL editor status bar no longer shows another tab's old duration.
  切换标签时改为从活动标签页回填查询耗时，并在新查询开始时清理标签页本地耗时快照，因此 SQL 编辑器状态栏不再显示其他标签页的旧耗时。
- Made the SQL editor status bar prefer the active tab's own last query message, and clear that tab-local message when a new query starts so unrelated global notifications no longer override the current tab's query feedback.
  让 SQL 编辑器状态栏优先显示活动标签页自己的最近查询消息，并在新查询开始时清理该标签页本地消息，因此无关的全局通知不再覆盖当前标签页的查询反馈。
- Split explicit query cancellation from silent stale-request cleanup in the app runtime, so a true user-cancelled `QueryDone` can now bypass the stale gate while superseded-query, tab-close, and disconnect cancellations remain silent.
  在应用运行时中拆分了显式取消查询与静默 stale 请求清理路径，因此真正的用户取消 `QueryDone` 回包现在可以通过 stale gate，而被新查询覆盖、关闭标签页和断开连接触发的取消仍然保持静默。
- Stopped inactive or stale query callbacks from overwriting the active SQL editor timing badge, and now store elapsed time on the target tab before mirroring it only when that tab is still active.
  修复非活动或 stale 查询回包会覆盖当前 SQL 编辑器耗时徽标的问题，并改为先把耗时写入目标标签页，再仅在该标签页仍为活动页时同步到全局镜像。
- Split SQL draft syncing from grid workspace persistence during tab navigation, so editor text changes no longer trigger unrelated workspace-saving side effects while switching tabs still preserves the full active-tab surface state.
  将 tab 导航时的 SQL 草稿同步与 grid workspace 持久化拆分开来，因此编辑器文本变化不再触发无关的工作区保存副作用，而切换标签页时仍会完整保存当前活动页的 surface 状态。
- Added shared viewport constraints to the reusable dialog shells and removed outer vertical scrolling from workspace dialogs, so shell-based windows now stay inside the visible area and multi-pane dialogs no longer depend on competing outer-window scrolling.
  为可复用 dialog shell 统一补上视口约束，并移除了 workspace dialog 的外层纵向滚动，因此使用共享壳层的窗口现在会保持在可视区域内，多栏 dialog 也不再依赖相互竞争的外层窗口滚动。
- Removed hard minimum widths from Help detail cards and picker pane content, so narrow multi-pane help layouts now fill the available pane width instead of forcing cards to claim at least `220/260px`.
  去除了 Help 详情卡片与 picker pane 内容里的硬最小宽度，因此窄视口下的多栏帮助布局现在会贴合当前 pane 宽度，而不再强行要求至少 `220/260px`。
- Moved the DataGrid goto and save-confirm utility windows onto the shared fixed dialog shell, so these local overlays now inherit the same viewport constraints and shell sizing as the rest of the dialog system.
  将 DataGrid 的“跳转到行”和“确认保存”小窗迁移到共享的固定尺寸 dialog shell，因此这些局部浮窗现在会继承与其他 dialog 一致的视口约束和壳层尺寸策略。
- Added an explicit app-level dialog owner and stopped the input router from re-deriving dialog scope from legacy visibility booleans, so dialog input ownership now follows one primary authority path with compatibility fallback only for old visibility toggles.
  新增 app 级显式 dialog owner，并移除了输入路由按 legacy 可见性布尔位二次推导 dialog scope 的逻辑，因此 dialog 输入所有权现在先走单一主权威路径，旧可见性字段只保留兼容回退作用。
- Moved destructive delete confirmation and the DataGrid save-confirm flow onto explicit blocking modal contracts, so dangerous confirms now stop background input consistently instead of behaving like ordinary floating windows.
  将 destructive delete confirm 与 DataGrid 保存确认统一迁移到显式 blocking modal contract，因此危险确认现在会一致地阻断背景输入，而不再表现为普通浮动窗口。
- Moved the keybinding settings dialog onto a fixed workspace shell with a layered picker body, so the footer now stays pinned while navigator/items panes compact or hide instead of permanently competing with the detail editor for height and width.
  将快捷键设置对话框迁移到固定 workspace shell 和 layered picker 主体，因此底部 footer 现在会固定可见，导航/列表 pane 也会按层级收缩或隐藏，而不再长期和详情编辑区争抢高度与宽度。
- Moved the help dialog onto the same shared workspace shell and layered picker contract, so detail-focused help content can now compact or hide earlier levels instead of always keeping three full panes on screen.
  将帮助对话框迁移到同一套共享 workspace shell 与 layered picker 契约，因此聚焦正文时更早层级现在可以收缩或隐藏，而不再始终保留三列占位。
- Added a reusable `FormDialogShell` and migrated the create-user, create-database, and DDL dialogs onto fixed-footers with a single body scroll owner, so long form content and SQL previews no longer need to push action buttons out of reach.
  新增可复用的 `FormDialogShell`，并将创建用户、创建数据库和 DDL 对话框迁移到固定 footer + 单一主体滚动模式，因此较长的表单内容和 SQL 预览不再需要把操作按钮顶出可达区域。
- Moved the toolbar action/create menus out of anonymous raw popups and onto explicit overlay dialogs with visible trigger focus, so `h/l` can clearly select those toolbar slots and `Enter` now opens an owner-aware list/detail chooser instead of a hidden popup path.
  将 toolbar 的 action/create 菜单从匿名 raw popup 迁移到显式 overlay dialog，并补上 trigger 的可见焦点态，因此 `h/l` 现在可以清楚选中这两个 toolbar 槽位，`Enter` 打开的也不再是隐藏 popup，而是 owner-aware 的 list/detail 选择对话框。
- Moved the toolbar theme picker out of the inline raw popup and onto an explicit chooser overlay with dialog ownership, so `Ctrl+Shift+T` and the toolbar theme trigger now open the same viewport-aware theme selector instead of a local `Area::fixed_pos(...)` popup.
  将 toolbar 主题选择器从内联 raw popup 迁移到显式 chooser overlay，并纳入 dialog owner，因此 `Ctrl+Shift+T` 和 toolbar 主题 trigger 现在会打开同一个受视口约束的主题选择器，而不再走局部 `Area::fixed_pos(...)` popup。
- Moved `ConnectionDialog` onto the fixed form-dialog shell and replaced its core path/certificate/private-key rows with responsive layouts, so the footer now stays pinned while narrow viewports no longer rely on `Grid + fixed-width + browse` rows to remain usable.
  将 `ConnectionDialog` 迁移到固定的表单对话框壳层，并把核心路径/证书/私钥行改成响应式布局，因此 footer 现在会固定可见，窄视口也不再依赖 `Grid + 固定宽度 + 浏览` 行来勉强维持可用性。
- Moved `ImportDialog` onto the fixed form-dialog shell and replaced its file selector, format/mode controls, and CSV/JSON option rows with responsive layouts, so the footer stays pinned while narrow viewports no longer rely on subtractive width math or crowded single-line control groups.
  将 `ImportDialog` 迁移到固定的表单对话框壳层，并把文件选择、格式/模式控制以及 CSV/JSON 选项行改成响应式布局，因此 footer 会固定可见，窄视口也不再依赖减法宽度计算或拥挤的单行控件组来勉强维持可用性。
- Reworked `DdlDialog` table-info rows and column dense rows into responsive wide/medium/narrow layouts, so table metadata and column editors now degrade without horizontal stretching while preserving column shortcuts, SQL preview, and the pinned footer contract.
  将 `DdlDialog` 的表信息行与列定义 dense row 重做为宽/中/窄响应式布局，因此表元数据与列编辑器现在可以在不横向继续延伸的情况下完成退化，同时保留列快捷键、SQL 预览和固定 footer 契约。
- Moved `CreateDbDialog` onto the same fixed form-dialog shell behavior as the other recovered long forms, and replaced its database-name / charset / collation / encoding / template / owner / SQLite-path rows with responsive layouts so the create footer no longer depends on fixed-width single-line rows to stay reachable.
  将 `CreateDbDialog` 对齐到其他已恢复长表单的固定表单壳层行为，并把数据库名、字符集、排序规则、编码、模板、所有者和 SQLite 路径这些行改成响应式布局，因此“创建” footer 不再依赖固定宽度单行表单来勉强保持可达。
- Added scoped keyboard actions to the welcome setup guide and made it track its own selected footer action, so `Tab / Shift+Tab / Enter / 1..5` now stay inside the onboarding dialog and can reliably run the sample query that seeds `ExportDialog`.
  为欢迎安装引导补上 scoped 键盘动作，并让引导自行维护底部动作选中状态，因此 `Tab / Shift+Tab / Enter / 1..5` 现在都会留在引导对话框内部消费，并能稳定执行学习示例查询来建立 `ExportDialog` 的结果集 seed。
- Stopped tiny dialog viewports from crashing the main workspace when SQL editor space becomes smaller than its old `100px` minimum, so opening dialogs such as `ExportDialog` in a very small window now shrinks the editor instead of panicking on an invalid height clamp.
  修复极小视口下 dialog 打开会把主工作区带崩的问题：当 SQL 编辑器剩余空间小于旧的 `100px` 最小值时，现在会继续收缩而不是进入非法高度 `clamp`，因此像 `ExportDialog` 这样的对话框在很小窗口中不再因高度分配 panic。
- Audited the core docs set so README, keyboard guide, keymap spec, architecture notes, sidebar workflow notes, and testing guidance now describe the current scope-aware input model, resizable picker dialogs, explicit grid edge transfers, and separated sidebar delete targets more consistently.
  审计并更新了核心文档集合，使 README、键盘指南、keymap 规范、架构说明、侧边栏工作流说明与测试指南现在能更一致地描述当前的 scope-aware 输入模型、可缩放 picker 对话框、显式 grid 边界转移，以及已拆分的侧边栏删除目标。
- Made ER loading wait for both the foreign-key request and all per-table column requests, and cache FK column pairs across async arrival order, so ER no longer marks itself ready too early or lose FK badges when columns arrive after relationships.
  让 ER 加载改为同时等待外键请求和所有按表列请求完成，并缓存 FK 列对以跨越异步回包顺序，因此 ER 不会再过早结束 loading，也不会在“先到关系、后到列”的情况下丢失外键徽标。
- Moved ER layout, empty/inferred relationship resolution, and the final ready notification into a single finalize phase, so foreign-key responses no longer announce completion before the diagram is actually ready.
  将 ER 的 layout、空关系推断和最终 ready 提示统一到单一 finalize 阶段，因此外键回包不再在图真正 ready 之前抢先宣告完成。
- Added first-stage TUI-style local navigation to the ER workspace, so the focused ER pane now uses `j/k` to move between tables, `Enter/Right` to open the selected table back into the main workspace, `h/Left/Esc` to return to `DataGrid`, and `q` to close ER while preserving the existing `r/l/f/+/-` canvas commands.
  为 ER 工作区加入了第一阶段 TUI 风格本地导航，因此当前聚焦的 ER 面板现在会使用 `j/k` 在线性表顺序中移动、使用 `Enter/Right` 打开当前选中表并带回主工作区、使用 `h/Left/Esc` 返回 `DataGrid`、使用 `q` 关闭 ER，同时保留现有的 `r/l/f/+/-` 画布命令。
- Reworked the first wave of ER theme tokens to derive card layers, text, borders, selection, and the compact toolbar chrome from Gridix theme colors and `egui::Visuals`, so ER now follows the active theme more closely instead of relying on a separate hardcoded dark/light palette for those shared surfaces.
  将 ER 第一波主题 token 改为从 Gridix 主题色和 `egui::Visuals` 派生卡片层级、文字、边框、选中态和紧凑工具条 chrome，因此这些共享表面现在会更贴近当前激活主题，而不再依赖另一套硬编码的深浅色调色板。
- Reworked the remaining ER canvas tokens so grid dots, relation lines, PK/FK markers, shadow depth, and type text now derive from the active Gridix theme instead of fixed RGB values, while still preserving ER-specific visual semantics.
  将 ER 画布剩余 token 重做为从当前 Gridix 主题派生，因此网格点、关系线、主外键标记、阴影深度和类型文字现在都不再依赖固定 RGB，同时仍保留 ER 自己的视觉语义。
- Added a minimal ER return-history contract so leaving ER with `h/Left/Esc` or closing it with `q` now restores the most recent legal non-ER workspace area (`Sidebar`, `DataGrid`, or `SqlEditor`) instead of always collapsing back to `DataGrid`.
  为 ER 补上了最小返回历史契约，因此现在用 `h/Left/Esc` 离开 ER 或用 `q` 关闭 ER 时，会恢复最近一个合法的非 ER 工作区区域（`Sidebar`、`DataGrid` 或 `SqlEditor`），而不再一律塌缩回 `DataGrid`。
- Stopped the ER foreign-key handler from inferring relationships early when the FK query returns no explicit edges, so both empty-FK and FK-error paths now defer inference until finalize after all table-column requests settle.
  修复 ER 外键处理链在 FK 查询返回空结果时会提前推断关系的问题，因此“空 FK 结果”和“FK 请求报错”两条路径现在都会统一延迟到 finalize 阶段，并在所有表列请求落定后再决定是否推断关系。
- Promoted ER keyboard navigation to the stronger `h/l` TUI-style contract: bare `l` now opens the selected table into the main workspace, while relayout moves to `Shift+L`.
  将 ER 键盘导航提升到更强的 `h/l` TUI 风格契约：bare `l` 现在会把当前选中表打开回主工作区，而重新布局改由 `Shift+L` 触发。
- Made ER keyboard selection keep the chosen table inside the visible canvas: changing `selected_table` through keyboard navigation or re-entering ER focus now recenters/pans just enough in the next frame so the highlight no longer drifts off-screen.
  让 ER 的键盘选中会继续把当前表保留在可见画布内：通过键盘导航切换 `selected_table` 或重新进入 ER 焦点后，下一帧会自动做最小视口修正，因此“高亮跑到屏幕外但索引还在继续变化”的情况不再出现。
- Added a dedicated `focus_er_diagram` action on `Alt+R`, so ER can now be entered explicitly from workspace command mode without overloading `Ctrl+R`: the new shortcut only focuses an already-open ER pane and never toggles visibility or triggers a reload.
  新增专用的 `focus_er_diagram` 动作，默认绑定 `Alt+R`，因此现在可以在工作区命令模式下显式进入 ER，而不再继续复用 `Ctrl+R`：这个新快捷键只会聚焦已经打开的 ER 面板，不会切换显隐，也不会触发 reload。
- Restored `Ctrl+R` as a reliable ER toggle even while ER itself owns focus: `ToggleErDiagram` now stays routable from both `er_diagram` and `er_diagram.viewport`, but this fix does not broaden unrelated workspace overlays such as `ShowHistory`.
  恢复了 `Ctrl+R` 作为 ER 显隐切换的可靠性：即使当前焦点已经在 `er_diagram` 或 `er_diagram.viewport`，`ToggleErDiagram` 也仍会继续路由；与此同时，这次修复不会顺手放开 `ShowHistory` 之类无关的 workspace overlay。

## [4.1.0]

### Changed
- Introduced a table-scoped `GridWorkspaceStore` and virtual-row model so pending new rows, filters, cursor state, and unsaved edits now stay attached to the current table workspace instead of leaking across table switches.
  引入按表隔离的 `GridWorkspaceStore` 与虚拟行模型，使待保存的新行、筛选、光标状态和未保存编辑现在都绑定到当前表格工作区，不再在切表后泄漏。
- Tightened sidebar layer traversal so `l` / Right now only enters deeper connection layers or filter value input, while cross-panel movement remains on vertical edge transfer.
  收紧侧边栏层级遍历语义：`l` / 右箭头现在只负责进入更深的连接层级或筛选值输入，跨 panel 流转继续只保留给纵向边界移动。
- Moved picker-style dialog navigation onto `dialog.picker.*` scoped commands so help and keybinding dialogs no longer hard-code raw `Tab/h/j/k/l/Enter` handling outside the keymap-aware dialog shortcut path.
  将 picker 风格对话框的导航迁移到 `dialog.picker.*` scoped command，使帮助与快捷键设置不再在 keymap-aware 的对话框快捷键路径之外硬编码处理 `Tab/h/j/k/l/Enter`。
- Moved toolbar, query-tab, and toolbar popup navigation onto scoped local commands, and restored keyboard opening for the toolbar action/create menus instead of leaving those slots outside the keyboard workflow.
  将工具栏、查询标签栏以及工具栏弹层导航迁移到 scoped local command，并恢复工具栏“操作/新建”菜单的键盘打开能力，不再把这两个位置留在键盘工作流之外。
- Moved command-palette and ER-diagram keyboard handling onto scoped local commands as well, so these overlays no longer keep a separate raw-key path outside the scope-aware shortcut system.
  继续将命令面板与 ER 图的键盘处理迁移到 scoped local command，使这些浮层不再维护独立于 scope-aware 快捷键体系之外的 raw-key 路径。
- Moved grid inline-edit finish and `sidebar.filters.input` escape handling onto scoped local commands too, so the remaining local edit-dismiss paths also run through the same keymap-aware shortcut layer.
  继续将表格内联编辑结束键和 `sidebar.filters.input` 的返回键迁移到 scoped local command，使剩余的局部编辑退出路径也经过同一层 keymap-aware 快捷键语义。
- Stopped picker-style dialogs from forcing their windows wider than the current viewport, and unified sidebar delete menus so “delete database” and “delete connection” are shown as separate targets in the same place.
  修复 picker 风格对话框会把窗口自动撑大的问题，并统一侧边栏删除菜单，使“删除数据库”和“删除连接”以两个独立目标在同一位置展示。
- Made connection-level destructive actions explicit again: the active connection strip now exposes separate `删库` / `删连` controls, and MySQL database deletion no longer depends on being connected to the target database itself.
  恢复连接级危险操作的显式展示：活动连接条现在直接显示独立的 `删库` / `删连` 控件，同时 MySQL 删除数据库不再依赖“当前正连着目标数据库”这一脆弱前提。
- Restored workspace-style help and keybinding dialogs as movable, resizable windows, and let help collapse the navigation/item panes once detail content is active so the reading area gets more width.
  恢复帮助与快捷键设置对话框的可拖拽、可缩放窗口行为，并让帮助页在进入详情后自动收窄导航/层级列，把更多宽度让给正文。
- Split the sidebar visibility toolbar into explicit “工作区” and “高级” rows so panel-group labels no longer wrap into a single mixed flow.
  将侧边栏显隐工具条拆成明确的“工作区”和“高级”两行，避免分组标签和按钮继续混在同一条自动换行流里。

### Fixed
- Restored the connection-row context menu after the custom sidebar header regression, and re-aligned connection/database/table delete entrypoints so right-click actions, header delete buttons, and keyboard `d` all flow through the same target-specific delete-confirm chain.
  修复自定义侧边栏连接头部带来的 connection-row 右键菜单回归，并重新收口连接/数据库/表的删除入口：右键动作、头部删连/删库按钮与键盘 `d` 现在都会先进入同一条按目标区分的删除确认链。
- Sidebar delete targets now carry connection context for database/table drops, so connection-header delete actions and table deletes no longer depend on whichever connection happens to be active.
  侧边栏删除目标现在会携带连接上下文，数据库/表删除不再错误依赖“当前恰好处于 active 的连接”；连接头部的删除入口和删表动作因此恢复可靠。
- Restored connection-row expansion and destructive controls after the custom header regression: clicking the connection label now expands/collapses the database/table stack again, and `删库 / 删连` are back as direct header actions instead of a hidden submenu path.
  修复连接行自定义 header 带来的回归：点击连接标签现在会正常展开/折叠数据库与表列表，`删库 / 删连` 也恢复为头部的直接动作，不再藏在不稳定的子菜单路径里。
- Counted grid navigation, insert-mode entry, and row copying now include pending new rows as first-class virtual rows, and `h` / Left at the first grid column transfers focus back to the sidebar again.
  修复表格中的数字计数导航、进入编辑和复制整行逻辑，使未保存新行被视为一等虚拟行；同时恢复在首列按 `h` / 左箭头返回侧边栏。
- Picker-style dialogs now auto-reveal the keyboard-selected entry inside their scroll areas, keeping help and keybinding lists in view during fully keyboard-driven navigation.
  修复 picker 风格对话框在纯键盘导航下不会自动滚动的问题：帮助和快捷键设置中的当前选中项现在会自动滚动到可见区域。
- Export dialog scroll regions and repeated widgets now use stable ids, which fixes the broken egui duplicate-id overlays and keeps large previews/column lists rendering normally.
  为导出对话框的滚动区和重复控件补上稳定 id，修复 egui 重复 id 导致的异常叠层提示，并让大预览与列列表恢复正常渲染。
- Deleting a database is now a first-class workflow separate from deleting a connection, with dedicated sidebar actions, confirmation copy, and MySQL/PostgreSQL runtime handling.
  删除数据库现在成为独立于删除连接的一等工作流：拥有单独的侧边栏动作、确认文案以及 MySQL/PostgreSQL 运行时处理链路。
- Removed the dead global filter-binding path from the scope-aware keymap: filter editing remains available through the sidebar filter workspace and command palette, but `Ctrl+F / Ctrl+Shift+F` are no longer advertised as routed top-level shortcuts that runtime rejects.
  从 scope-aware keymap 中移除了失效的全局筛选绑定路径：筛选编辑仍可通过侧边栏筛选工作区和命令面板完成，但不再把运行时会拒绝的 `Ctrl+F / Ctrl+Shift+F` 宣传为顶层快捷键。

## [4.0.0]

### Changed
- Reframed major area switching as the `next_focus_area` / `prev_focus_area` action pair in the input router, so `Tab / Shift+Tab` are now only default bindings for workspace fallback actions instead of hard-coded global-first keys.
  将主区域切换在输入路由中重构为 `next_focus_area` / `prev_focus_area` 动作对，使 `Tab / Shift+Tab` 现在只是 workspace fallback action 的默认绑定，而不是硬编码的 global-first 按键。
- Tightened the router order so focused-scope keymap actions now always run before workspace fallback shortcuts such as `next_focus_area`, and converted the remaining theme/sidebar fallback keys into action-backed routes instead of direct router hooks.
  收紧输入路由顺序，使当前聚焦作用域的 keymap 动作现在始终先于 `next_focus_area` 这类 workspace fallback 快捷键执行，并将剩余的主题/侧边栏回退按键改为 action-backed 路径，而不是 direct router hook。
- Removed the old global filter shortcut path from the input router so sidebar filter editing now lives only in the sidebar-local workflow instead of leaking through app-level fallback handling.
  从输入路由中移除了旧的全局筛选快捷键路径，使 Filters 编辑现在只存在于 sidebar 局部工作流中，不再通过 app-level fallback 泄漏。
- Added persistent `sidebar.edge_transfer` config and hardened the sidebar focus graph around the explicit `Connections -> Databases -> Tables -> Filters -> Triggers -> Routines` order.
  新增持久化的 `sidebar.edge_transfer` 配置，并围绕显式的 `Connections -> Databases -> Tables -> Filters -> Triggers -> Routines` 顺序收紧侧边栏焦点图。
- Completed the third-round keymap migration by initializing `~/.config/gridix/keymap.toml` from defaults when missing, keeping runtime partial-merge backfill in memory, and surfacing parser/runtime diagnostics instead of silently dropping issues.
  完成第三轮 keymap 迁移：在 `~/.config/gridix/keymap.toml` 缺失时从默认值初始化，只在内存中做补齐合并，并通过 diagnostics 暴露解析期和运行时问题，而不是静默丢弃。
- Reworked the shortcut settings dialog into a scope-aware skeleton with a scope tree, per-scope action list, current binding/source display, and diagnostics placeholder instead of the old flat action table.
  将快捷键设置界面重构为 scope-aware skeleton：包含作用域树、按 scope 展开的动作列表、当前绑定/来源显示以及 diagnostics 占位，不再沿用旧的平铺动作表。
- Added a legacy-import affordance and keymap-path card to the shortcut settings dialog, so users can explicitly pull old `config.toml` bindings into the new editor and copy the active `keymap.toml` location.
  为快捷键设置界面新增 legacy 导入入口和 keymap 路径卡片，使用户可以显式把旧 `config.toml` 键位导入到新编辑器中，并复制当前生效的 `keymap.toml` 路径。
- Exposed scope-action override rows such as `toolbar.refresh` directly inside the shortcut settings dialog, including inherited/local source state and scoped diagnostics instead of limiting the editor to legacy local commands.
  在快捷键设置界面中直接暴露 `toolbar.refresh` 这类 scope-action override 条目，并显示继承/局部来源状态与 scoped diagnostics，而不是把编辑器限制在遗留的局部命令上。
- Extended the shortcut settings dialog to expose text-entry runtime scopes such as `editor.insert` and `sidebar.filters.input` with only text-entry-safe scoped actions, keeping command-mode-only actions out of those lists.
  扩展快捷键设置界面，显式暴露 `editor.insert` 与 `sidebar.filters.input` 这类文本输入运行时作用域，并只展示 text-entry-safe 的 scoped action，避免把仅适用于 command mode 的动作放进这些列表。
- Tightened DataGrid keyboard semantics so `h/j/k/l` stay local to table movement, fixed counted movement to avoid double-applying the numeric prefix, and kept the explicit bottom-edge `j` transfer to SQL editor.
  收紧数据表格键盘语义，使 `h/j/k/l` 保持为表格内移动；修复数字计数移动被重复应用的问题；并保留“在底部再次按 `j` 才进入 SQL 编辑器”的显式转移。
- Removed the SQL editor's remaining hard-coded execute/explain keys from local handling so `F5` / `Ctrl+Enter` / `F6` continue to work only through editor-scoped bindings and current input ownership.
  移除 SQL 编辑器局部处理里残留的硬编码执行/分析按键，使 `F5` / `Ctrl+Enter` / `F6` 继续只通过 editor-scoped 绑定和当前输入所有权生效。
- Added a reusable picker-style dialog shell for layered selection flows, so chooser-style dialogs can use the same fixed-size, keyboard-first, click-to-open structure.
  新增可复用的 picker 风格对话框壳，用于分级选择工作流，使 chooser 类对话框共享固定尺寸、键盘优先、单击打开的统一结构。
- Reworked the help dialog into a layered picker flow with fixed panes for root topics, current items, and detail content while keeping the existing learning actions and reducer path.
  将帮助对话框重构为分级 picker 流程，固定显示主线、当前条目和详情三栏，同时保留现有学习动作与 reducer 路径。
- Rebuilt the keybinding settings dialog around the same layered picker model so scope selection, action browsing, and binding editing now follow a yazi-like open flow instead of an expanding workspace layout.
  将快捷键设置对话框重构为同一套分级 picker 模型，使作用域选择、动作浏览与绑定编辑遵循 yazi 风格的逐级打开流程，而不是继续使用会扩张的 workspace 布局。
- Continued the dialog reducer split by moving help-learning navigation and keybinding-editor mutations onto explicit action paths instead of mutating business state directly from render branches.
  继续推进 dialog reducer 拆分：帮助学习导航和快捷键编辑区的状态变更改为显式 action 路径，不再直接从渲染分支修改业务状态。
- Unified help-content buttons behind a single `HelpUiAction` exit so learning navigation and demo actions now leave the renderer through the same path.
  将帮助内容区按钮统一到单一 `HelpUiAction` 出口，学习导航与示例动作现在通过同一条路径离开渲染层。
- Moved keybinding search text and grid-sequence editor input onto dialog UI actions so the remaining high-frequency controls no longer mutate state directly from render code.
  将快捷键设置中的搜索词与表格命令序列编辑输入迁移到 dialog UI action 路径，使剩余高频控件不再直接从渲染代码修改状态。

### Fixed
- Restored keybinding-recording ownership so recording mode now consumes `Esc` and recorded keys itself instead of leaking back into generic dialog dismiss handling.
  修复快捷键录制态输入所有权：录制模式现在会自行消费 `Esc` 和录制按键，不再错误落回通用对话框关闭逻辑。
- Fixed the picker shell width allocation so help and keybinding dialogs now always fit the actual window width instead of inventing extra horizontal space.
  修复 picker 壳的列宽分配逻辑，帮助与快捷键设置对话框现在始终服从实际窗口宽度，不再虚构额外横向空间。
- Locked the help and keybinding dialogs to fixed-size windows with internal scrolling, preventing both dialogs from auto-extending with content.
  将帮助与快捷键设置对话框锁定为固定尺寸窗口并使用内部滚动，阻止内容驱动窗口继续自动延伸。
- Fixed the broken keybinding settings rendering by removing the previous expanding workspace layout and giving every pane stable scroll ids and business-key-based entry ids.
  通过移除之前会扩张的 workspace 布局，并为每个 pane 提供稳定的 scroll id 和基于业务 key 的条目 id，修复快捷键设置界面的异常渲染。

### Changed
- Introduced an app-level dialog host so only the active modal dialog owns keyboard input and dialog result handling in a frame.
  新增应用层对话框宿主，使每帧只有当前 active 模态对话框拥有键盘输入与结果处理权。
- Added a frame-level input owner model for modal, text-entry, select, command, recording, and disabled input states.
  新增每帧输入所有者模型，覆盖模态、文本输入、选择、命令、录制与禁用状态。
- Updated dialog rendering orchestration to respect active dialog priority instead of letting all open dialog flags process interaction.
  更新对话框渲染编排，按 active dialog 优先级处理，而不是让所有打开状态的对话框都处理交互。
- Replaced the SQLite create-database string sentinel with an explicit `CreateDatabaseRequest::SqliteFile` workflow request.
  将 SQLite 创建数据库的字符串哨兵替换为显式的 `CreateDatabaseRequest::SqliteFile` 工作流请求。
- Added a scoped command metadata registry and moved legacy local shortcut descriptions, categories, and default bindings behind that registry.
  新增作用域命令元数据注册表，并将遗留局部快捷键的说明、分类与默认键位迁移到该注册表之后。
- Added command-id dialog shortcut helpers and migrated the import/export dialog keyboard handlers to resolve scoped command ids directly.
  新增 command-id 对话框快捷键 helper，并将导入/导出对话框键盘处理迁移为直接解析作用域 command id。
- Reworked grid save execution into a batched workflow so multi-row edits are not cancelled by later statements, and successful saves refresh back into the same table view instead of falling through to Welcome.
  将表格保存重构为批量执行工作流，避免多行修改被后续语句取消，并在保存成功后刷新回同一张表视图，而不是掉回 Welcome。
- Adjusted DataGrid bottom-edge navigation so reaching the last row keeps focus in the grid, and only a subsequent `j` / Down opens the SQL editor while scrolling the table to the bottom.
  调整数据表格底边导航行为：到达最后一行时仍保持表格焦点，只有后续再次按 `j` / 下箭头时才打开 SQL 编辑器，并同时将表格滚动到底部。
- Replaced a batch of hard-coded dark-only label colors with theme-driven text colors across the toolbar, query tabs, SQL editor, help/about dialogs, sidebar menus, ER diagram controls, and grid actions so light themes remain readable.
  将工具栏、查询标签、SQL 编辑器、帮助/关于对话框、侧边栏菜单、ER 图控制条与表格动作中的一批仅适配暗色模式的硬编码文字颜色改为主题驱动颜色，确保日间主题下仍可读。
- Refactored `ConnectionDialog` so keyboard shortcuts resolve scoped command ids directly and all file-picker side effects are dispatched from a single action path instead of being embedded inside render branches.
  重构 `ConnectionDialog`：键盘快捷键改为直接解析作用域 command id，并将所有文件选择副作用收口到统一动作分发路径，不再散落在渲染分支中。
- Migrated create-database, create-user, and DDL dialog shortcut parsing to scoped command ids and added regression coverage for text-entry priority on DDL column navigation.
  将创建数据库、创建用户和 DDL 对话框的快捷键解析迁移到 scoped command id，并补充 DDL 列导航在文本输入优先级下的回归测试。

### Documentation
- Updated architecture, keyboard focus, keymap, testing, release, distribution, and install docs for the `v4.0.0` scoped-input foundation.
  更新架构、键盘焦点、keymap、测试、发布、分发与安装文档，以反映 `v4.0.0` 的作用域化输入基础。

## [3.8.0]

### Added
- Added DataGrid command-sequence editing to the keybinding settings dialog, so `yy` / `dd` / `:w` / `gg` and related table commands can now be customized from the UI instead of only through `keymap.toml`.
  为快捷键设置界面新增数据表格命令序列编辑能力，使 `yy` / `dd` / `:w` / `gg` 等表格命令可以直接在 UI 中自定义，而不再只能手改 `keymap.toml`。

### Changed
- Extended the keybinding scope tree with a dedicated DataGrid section and sequence-management workflow for grid commands.
  扩展快捷键作用域树，新增独立的数据表格分区和命令序列管理流程。

### Fixed
- Added DataGrid prefix-conflict diagnostics so exact collisions and prefix-shadowing cases like `g` vs `gg` or `:` vs `:w` are surfaced before they break command chains.
  新增数据表格前缀冲突诊断，可在 `g` 与 `gg`、`:` 与 `:w` 这类命令链被吞掉之前，提前识别完全冲突和前缀遮蔽问题。

## [3.7.1]

### Changed
- Expanded the in-app learning sample into a versioned large relational dataset with 8 main tables, 100+ rows per table, and richer multi-hop relationships.
  将内置学习示例扩展为版本化的大型关系型数据集，包含 8 张主表、每表 100+ 行，并提供更丰富的多跳关系。
- Updated learning-guide overview and onboarding copy so the sample database is described as a real teaching dataset instead of a tiny demo.
  更新学习指南总览与新手引导文案，使示例数据库被明确描述为真实教学数据集，而不是小型演示库。

### Fixed
- Fixed focus-routing regressions where sidebar-to-grid transfer, SQL editor cancel behavior, and DataGrid horizontal movement could stop responding consistently.
  修复焦点路由回归问题，解决侧边栏到表格的切换、SQL 编辑器取消行为以及数据表格横向移动不能稳定响应的问题。
- Fixed legacy learning-sample databases so older files are detected and rebuilt instead of failing to open after the dataset upgrade.
  修复旧版学习示例数据库兼容性问题，使旧文件会被识别并自动重建，而不是在数据集升级后无法打开。
- Fixed a query-learning edge case where sample mutation demos could clash with the new seeded dataset and constraints.
  修复查询学习示例中的边界问题，避免示例更新/删除演示与新的种子数据和约束发生冲突。

## [3.7.0]

### Added
- Added scope-tree navigation, issue-only filtering, and conflict-summary jumping to the keybinding settings dialog.
  为快捷键设置对话框新增作用域树导航、仅看问题项过滤以及冲突摘要跳转。
- Added structured `grid.normal.*` command-sequence support to `keymap.toml` for DataGrid command chains.
  为 `keymap.toml` 新增结构化 `grid.normal.*` 命令序列支持，用于配置数据表格命令链。
- Added regression coverage for scope-tree filtering, issue summaries, and custom Grid command-sequence overrides.
  新增作用域树筛选、冲突摘要以及数据表格自定义命令序列覆盖的回归测试。

### Changed
- Reworked the keybinding settings dialog from flat filters into a scope-tree driven workflow with richer issue analysis.
  将快捷键设置界面从平铺筛选重构为作用域树驱动流程，并增强问题分析能力。
- DataGrid mode help and high-frequency action tooltips now reflect the current runtime command sequences instead of fixed literals.
  数据表格模式帮助和高频操作提示改为反映当前运行时命令序列，不再写死固定字面量。

### Fixed
- Fixed a regression where configurable Grid command prefixes could break counted `gg` jumps.
  修复数据表格可配置命令前缀引入后可能破坏带计数 `gg` 跳转的回归问题。
- Fixed another gap between configurable shortcut infrastructure and DataGrid’s hard-coded command chains.
  修复快捷键可配置基础设施与数据表格硬编码命令链之间的又一处断层。

## [3.6.0]

### Added
- Added original Gridix brand assets under `assets/branding`, including a square app icon and a horizontal wordmark.
  新增原创 Gridix 品牌资产，统一放入 `assets/branding`，包括方形应用图标和横版字标。
- Added native window icon loading from the packaged branding icon.
  新增原生窗口图标加载，直接使用正式品牌图标。
- Added structured local keymap sections and runtime local-shortcut overrides on top of external `keymap.toml`.
  在外置 `keymap.toml` 之上新增结构化局部键位 section 与运行时局部快捷键覆盖能力。
- Added high-level Grid keyboard regression tests covering prefixes, counts, selection, save/quit commands, and filter entry.
  新增表格键盘高层回归测试，覆盖前缀命令、计数、选择模式、保存/退出命令以及筛选入口。

### Changed
- Moved packaging and runtime icon references from the repository root into `assets/branding`.
  将打包与运行时图标引用从仓库根目录迁移到 `assets/branding`。
- Updated README branding display to use the dedicated logo asset instead of the old root image path.
  README 的品牌展示改为使用专门的 logo 资产，不再使用旧的根目录图片路径。
- Updated desktop metadata to better reflect Gridix as a database tool.
  更新桌面文件元数据，使其更准确反映 Gridix 的数据库工具定位。
- Reworked dialogs, help/history panels, sidebar, and DataGrid around a shared local action/shortcut layer.
  对话框、帮助/历史面板、侧边栏与数据表格进一步重构为共享的局部动作/快捷键层。
- Expanded shortcut discoverability so hover hints and learning/help content increasingly reflect the current runtime keymap.
  扩展快捷键可发现性，悬停提示与学习/帮助内容开始更多地反映当前运行时真实键位。

### Fixed
- Fixed a branding/resource inconsistency where the root `gridix.png` had become an ad-hoc distribution asset.
  修复品牌资源不一致问题，根目录 `gridix.png` 不再作为临时发行资产继续扩散。
- Fixed several legacy keyboard paths that still relied on deprecated dialog-level handlers.
  修复多处仍依赖旧式对话框级键盘处理器的遗留路径。
- Fixed DataGrid command-buffer edge cases, including stuck prefixes, leaked counts, and incorrect `2gg` / `2G` row jumps.
  修复数据表格命令缓冲区边界问题，包括前缀卡死、计数泄漏以及 `2gg` / `2G` 跳行错误。
- Fixed sidebar command-prefix behavior so `gg` and `gs` work reliably inside the workflow list.
  修复侧边栏命令前缀行为，使 `gg` 与 `gs` 能在工作流列表中可靠工作。

## [3.4.0]

### Added
- Added focus-scoped input routing foundation and keyboard architecture RFC/spec docs.
  新增按焦点作用域分发输入的基础设施，并补充键盘架构 RFC/规范文档。
- Added external `keymap.toml` loading, generation, merge-backfill, and validation path.
  新增外置 `keymap.toml` 的加载、初始化生成、补齐合并与校验链路。
- Added unified local-shortcut tooltip/label helpers and wired them into toolbar, dialogs, sidebar, grid, help, and welcome UI.
  新增统一的局部快捷键提示/标签工具，并接入工具栏、对话框、侧边栏、表格、帮助和欢迎页。
- Added dedicated TSV import/export support with tests.
  新增正式的 TSV 导入/导出支持，并补充相应测试。

### Changed
- Reworked sidebar defaults and workflow toward beginner-friendly connections + filters layout.
  侧边栏默认布局和工作流重构为更适合新手的“连接 + 筛选”优先模式。
- Moved more workspace actions behind scoped helpers instead of global shortcut grabs.
  更多工作区动作改为走作用域化 helper，而不是被全局快捷键直接抢占。
- Help, welcome, learning guide, and configuration docs now reflect real runtime key bindings instead of hard-coded shortcuts.
  帮助、欢迎页、学习指南和配置文档现在会反映运行时真实键位，而不是硬编码快捷键。
- CSV/TSV/JSON import preview now also prepares generated SQL, enabling copy-to-editor flow before execution.
  CSV/TSV/JSON 导入预览现在会同步生成 SQL，可先复制到编辑器检查后再执行。

### Fixed
- Fixed several text-input vs global-shortcut conflicts in editor/sidebar-related paths.
  修复编辑器与侧边栏多处“文本输入被全局快捷键抢占”的冲突。
- Fixed import/export format model inconsistency where TSV existed only as hidden CSV behavior.
  修复导入导出格式模型不一致的问题，TSV 不再只是隐藏在 CSV 行为里的别名。

## [3.3.1]

### Added
- Added dedicated Nix Flake installation guide with run/install/build/overlay usage.
  新增专门的 Nix Flake 安装文档，覆盖运行、安装、构建与 overlay 用法。
- Added edge regression test suite for autocomplete/session/welcome-onboarding boundaries.
  新增边缘回归测试套件，覆盖自动补全/会话状态/欢迎页引导状态机边界。

### Changed
- Extended flake outputs with standard `apps` and `overlay` entries for more reliable Nix integration.
  Flake 输出增强，补充标准 `apps` 与 `overlay`，提升 Nix 集成稳定性。
- Updated README and getting-started docs with explicit `nix profile install` path.
  README 与新手上手文档补充明确的 `nix profile install` 安装路径。

## [3.3.0]

### Changed
- Updated full dependency lock graph to the latest Rust 1.94.1-compatible versions via `cargo update`.
  使用 `cargo update` 将依赖锁文件整体升级到 Rust 1.94.1 可兼容的最新版本。
- Upgraded direct dependencies to latest compatible versions:
  `eframe/egui/egui_extras/rfd/rusqlite/russh/toml`.
  直接依赖升级到当前兼容最新版本：
  `eframe/egui/egui_extras/rfd/rusqlite/russh/toml`。
- Refactored app architecture by splitting request lifecycle, preferences/config, and metadata loading logic into dedicated modules.
  应用架构重构：将请求生命周期、偏好/配置、元数据加载逻辑拆分为独立模块。
- Slimmed `app/mod.rs` and delegated per-frame orchestration to render flow entry.
  精简 `app/mod.rs`，每帧编排流程下沉到渲染入口方法。
- Updated frame rendering entry to current `eframe` API (`App::ui` + `CentralPanel::show_inside`) and resolved related input/style API changes.
  渲染入口迁移到当前 `eframe` API（`App::ui` + `CentralPanel::show_inside`），并完成输入/样式相关 API 适配。

### Documentation
- Updated keyboard guide baseline to `v3.3.x`.
  键盘文档基线更新为 `v3.3.x`。
- Added platform distribution guide for AUR/Homebrew/nixpkgs.
  新增 AUR/Homebrew/nixpkgs 分发指南文档。
- Refreshed release/process docs and roadmap baseline to `v3.3.0`.
  发布流程与优化路线图基线同步更新至 `v3.3.0`。

## [3.2.1]

### Documentation
- Rebuilt the full documentation set into a bilingual, indexed structure.
  将全套文档重构为中英同页、可索引结构。
- Added beginner and user-operational docs:
  `GETTING_STARTED`, `FAQ`, `TROUBLESHOOTING`, `CONFIGURATION`.
  新增新手与用户操作文档：
  `GETTING_STARTED`、`FAQ`、`TROUBLESHOOTING`、`CONFIGURATION`。
- Added engineering/maintenance docs:
  `ARCHITECTURE`, `TESTING`, `SECURITY`, `RELEASE_PROCESS`,
  `ENVIRONMENT_VARIABLES`, `LEARNING_CURRICULUM`, `DOCS_STYLE`, `CONTRIBUTING`.
  新增工程与维护文档：
  `ARCHITECTURE`、`TESTING`、`SECURITY`、`RELEASE_PROCESS`、
  `ENVIRONMENT_VARIABLES`、`LEARNING_CURRICULUM`、`DOCS_STYLE`、`CONTRIBUTING`。
- Added automated markdown local-link validation:
  script `scripts/check_doc_links.py` and CI workflow `.github/workflows/docs.yml`.
  新增 Markdown 本地链接自动校验：
  脚本 `scripts/check_doc_links.py` 与 CI 工作流 `.github/workflows/docs.yml`。

## [3.2.0]

### Added
- Beginner onboarding loop on welcome/help flows.
  欢迎页与帮助页新增新手上手闭环。
- Structured learning guide split from tool usage guide.
  帮助系统拆分为工具指南与数据库知识学习指南。

### Changed
- SQL editor completion and focus behavior stabilized.
  SQL 编辑器补全与焦点行为稳定性提升。
- Default dark theme set to Tokyo Night Storm.
  默认深色主题为 Tokyo Night Storm。
- New connection dialog defaults to simple mode (advanced collapsed).
  新建连接默认简化模式（高级选项折叠）。

### Fixed
- `Tab` completion and focus transfer conflicts in SQL editor.
  修复 SQL 编辑器 `Tab` 补全与焦点转移冲突。
- Multiple welcome/help layout and alignment issues.
  修复欢迎页与帮助页多处布局对齐问题。
