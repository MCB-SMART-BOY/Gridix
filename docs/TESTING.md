# Testing Guide | 测试指南

## 1. Quick Commands | 常用命令

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
nix --extra-experimental-features 'nix-command flakes' flake check --no-write-lock-file
python scripts/check_doc_links.py
```

Recommended local order / 建议本地执行顺序：
1. `cargo fmt`
2. `cargo clippy`
3. `cargo test`

## 2. Test Layout | 测试结构

Current test files in `tests/` include:
- `autocomplete_tests.rs`
- `core_tests.rs`
- `database_tests.rs`
- `ddl_dialog_tests.rs`
- `ddl_tests.rs`
- `edge_regression_tests.rs`
- `export_tests.rs`
- `formatter_tests.rs`
- `grid_tests.rs`
- `mysql_cancel_integration.rs`
- `ssh_tests.rs`
- `syntax_tests.rs`
- `ui_dialogs_tests.rs`

## 3. Integration Test (MySQL) | MySQL 集成测试

This test is ignored by default and requires MySQL service.  
该测试默认忽略，需要本机或 CI 提供 MySQL 服务。

```bash
GRIDIX_IT_MYSQL_HOST=127.0.0.1 \
GRIDIX_IT_MYSQL_PORT=3306 \
GRIDIX_IT_MYSQL_USER=gridix \
GRIDIX_IT_MYSQL_PASSWORD=gridix \
GRIDIX_IT_MYSQL_DB=gridix_test \
cargo test --test mysql_cancel_integration -- --ignored --nocapture
```

## 4. CI Coverage | CI 覆盖范围

- `.github/workflows/docs.yml`
  - Markdown local-link validation (`scripts/check_doc_links.py`).
- `.github/workflows/build.yml`
  - Cross-platform release build checks (Linux/Windows/macOS ARM).
- `.github/workflows/mysql-integration.yml`
  - Scheduled MySQL cancellation integration validation.

## 4.1 Edge Regression Suite | 边缘回归测试

The `edge_regression_tests.rs` suite focuses on:
`edge_regression_tests.rs` 重点覆盖：

- Autocomplete boundary behavior (unicode cursor, out-of-range cursor, dedup, result cap).
  自动补全边界行为（Unicode 光标、越界光标、去重、结果数量上限）。
- Session/tab boundary behavior (invalid index remove/switch).
  会话与标签页边界行为（越界索引删除/切换）。
- Welcome onboarding state machine transitions.
  欢迎页新手引导状态机流转。

Run only edge regressions:
仅运行边缘回归测试：
```bash
cargo test --test edge_regression_tests
```

## 5. High-Risk Areas To Verify | 高风险回归区域

Before merging changes touching these modules, run focused checks:
涉及以下模块时建议重点回归：

- `src/app/input/input_router.rs`, `src/app/input/owner.rs`, `src/app/dialogs/host.rs`, `src/app/surfaces/dialogs.rs`
  - Input owner, dialog owner, modal priority, and scoped dispatch must stay aligned.
  - 输入 owner、dialog owner、模态优先级与作用域分发必须继续一致。
- `src/ui/components/toolbar/mod.rs`, `src/ui/dialogs/toolbar_menu_dialog.rs`, `src/ui/components/toolbar/theme_combo.rs`
  - Toolbar focus visibility, overlay chooser behavior, and the remaining raw theme popup must keep keyboard and viewport behavior stable.
  - 工具栏焦点可见性、overlay 选择器行为，以及剩余的主题 popup 必须继续保持稳定的键盘流和视口边界。
- `src/ui/dialogs/common.rs`, `src/ui/dialogs/picker_shell.rs`, `src/ui/dialogs/help_dialog.rs`, `src/ui/dialogs/keybindings_dialog.rs`
  - Workspace dialog shells must keep fixed header/footer, pane-owned scrolling, and layered `Full / Compact / Hidden` behavior.
  - workspace dialog shell 必须继续保持固定 header/footer、pane 自有滚动，以及 `Full / Compact / Hidden` 的层级退场行为。
- `src/ui/dialogs/connection_dialog.rs`, `src/ui/dialogs/import_dialog/mod.rs`, `src/ui/dialogs/export_dialog.rs`, `src/ui/dialogs/create_db_dialog.rs`, `src/ui/dialogs/create_user_dialog.rs`, `src/ui/dialogs/ddl_dialog.rs`
  - Form dialogs must keep one main scroll owner, fixed footer actions, and text-entry-safe shortcut behavior.
  - 表单类 dialog 必须保持单一主体滚动、固定 footer 动作区，以及不会抢走文本输入的快捷键行为。
  - Manual smoke for `CreateUserDialog`: once a non-SQLite connection is active, verify `Ctrl+Shift+U` can open the dialog, `q` can close it, and a narrow floating viewport still keeps the footer visible while the main form and SQL preview remain scroll-safe.
  - `CreateUserDialog` 手工冒烟：在 active non-SQLite 连接下验证 `Ctrl+Shift+U` 能拉起 dialog、`q` 能关闭，且窄浮窗里 footer 仍可见，主体表单与 SQL 预览区继续保持安全滚动。
- `src/app/runtime/database.rs`, `src/app/runtime/request_lifecycle.rs`, `src/app/runtime/handler.rs`, `src/app/surfaces/render.rs`
  - Query execution must preserve request-id isolation, user-cancel vs stale split, active-tab mirrors, and explicit error rendering.
  - 查询执行链必须继续保持请求 ID 隔离、用户取消与 stale 分流、active tab 镜像一致，以及显式错误渲染。
- `src/ui/components/grid/actions.rs`, `src/ui/components/grid/keyboard.rs`, `src/app/runtime/handler.rs`
  - Grid saves must keep context isolation, failure-state retention, keyboard edit semantics, and same-table refresh on success.
  - grid 保存必须继续保持上下文隔离、失败保留编辑、键盘编辑语义，以及成功后回到同一表视图。
  - Query-result headers must remain readable across focused, filtered, and unfocused columns under the active theme; do not let non-focused headers fall back to effectively invisible implicit colors.
  - 查询结果表格列头在当前主题下必须继续保持可读：焦点列、筛选列和普通列都要有明确可见的前景色，不能再次退化成“只有焦点列可见”。
  - Manual smoke: open any wide result table and confirm column names such as `id / customer_id / label / recipient_name / ...` remain readable together instead of only the focused header standing out.
  - 手工冒烟：打开任意多列表结果表，确认 `id / customer_id / label / recipient_name / ...` 这类列名会一起保持可读，而不是只剩当前焦点列显眼。
- `src/ui/panels/sidebar/*`, `src/app/runtime/database.rs`, `src/app/surfaces/dialogs.rs`
  - Sidebar traversal and destructive actions must keep explicit targets, connection context, and a stable focus graph.
  - 侧边栏遍历与危险操作必须继续保持显式目标、连接上下文和稳定焦点图。
  - Focused anchors should keep both the sidebar entrypoints and the app-level confirmation chain covered: `SidebarDeleteTarget` construction, `handle_sidebar_actions()` saving `pending_delete_target` and opening `DeleteConfirm`, and `confirm_pending_delete()` clearing saved state after dispatch.
  - focused 锚点除了覆盖 sidebar 入口外，还应继续锁住 app 级确认链：`SidebarDeleteTarget` 构造、`handle_sidebar_actions()` 保存 `pending_delete_target` 并打开 `DeleteConfirm`，以及 `confirm_pending_delete()` 在分发后清理已保存状态。
- `src/core/keybindings.rs`, `src/core/commands.rs`, `src/ui/shortcut_tooltip.rs`
  - Registry ids, inherited bindings, diagnostics, and tooltip metadata must stay in sync with runtime keymap behavior.
  - 注册表 id、继承绑定、诊断信息与 tooltip 元数据必须继续与运行时 keymap 行为一致。
- `src/core/theme.rs`, `src/ui/styles.rs`, `src/ui/components/er_diagram/*`, `src/ui/components/toolbar/*`
  - Theme changes must keep light/dark readability and avoid reintroducing dark-only foreground assumptions.
  - 主题调整必须继续保证明暗主题可读性，避免重新引入只适配暗色背景的前景色假设。
- `src/app/input/input_router.rs`, `src/app/runtime/er_diagram.rs`, `src/ui/components/er_diagram/state.rs`, `src/ui/components/er_diagram/render.rs`
  - ER top-level combos must keep `navigation / viewport` scope boundaries stable: opening with `Ctrl+R` must now enter ER focus directly, bare `h / l` in navigation must stay local to ER geometry navigation rather than returning/opening immediately, viewport-mode `q` still closes ER, `r / Shift+L / f` still work there, `Ctrl+R` must still route `ToggleErDiagram` even when ER itself owns focus, open/refresh/reload must reset `interaction_mode` back to navigation, theme-switch renders must not wipe the current ER viewport/selection state, `FocusErDiagram -> ToggleErDiagram` must restore a legal non-ER workspace focus (or fall back to `DataGrid`) without leaving hidden ER scope behind, `ShowHistory` must remain blocked in ER scope, `Shift+J / Shift+K` must keep relation-adjacent navigation deterministic and layout-independent, `Shift+Arrow` must keep geometry-adjacent navigation directional, local-only, and deterministic for a fixed layout, and finalize must now default to force-directed layout whenever explicit or inferred relationships exist.
  - ER 顶层组合链必须继续保持 `navigation / viewport` 作用域边界稳定：`Ctrl+R` 打开 ER 时现在必须直接切入 ER 焦点，导航态里的 bare `h / l` 也必须继续留在 ER 内部承担几何邻接，而不是立刻返回主工作区或打开查询；视口模式内的 `q` 仍要关闭 ER，`r / Shift+L / f` 仍要可用，`Ctrl+R` 在 ER 自己持有焦点时仍必须继续路由到 `ToggleErDiagram`，open/refresh/reload 必须把 `interaction_mode` 重置回浏览态，theme switch render 也不能把当前 ER 的视口/选中状态洗掉，而 `FocusErDiagram -> ToggleErDiagram` 还必须恢复一个合法的非 ER workspace 焦点（目标不可用时回退到 `DataGrid`），且不能残留隐藏 ER scope；`ShowHistory` 仍必须在 ER scope 下保持被阻止；新增的 `Shift+J / Shift+K` 关系邻接导航也必须继续保持确定性，不能受当前几何布局影响；新增的 `Shift+Arrow` 几何邻接则必须继续保持方向性、局部性，并且在固定布局下具有确定性；同时 finalize 在存在显式或推断关系时，默认完成态布局也必须自动落到 force-directed，而不是继续停在纯 grid。
  - Focused anchors should also keep the new geometry-adjacency boundaries explicit: `Shift+Arrow` must remain `NoOp` in `er_diagram.viewport`, and geometry navigation must not pre-sync app-level `selected_table` before `OpenSelectedTable`.
  - focused 锚点还应继续显式锁住新的几何邻接边界：`Shift+Arrow` 在 `er_diagram.viewport` 中必须保持 `NoOp`，且几何邻接在 `OpenSelectedTable` 之前不得提前同步 app 层 `selected_table`。
  - Focused layout anchors should also prove relation-seeded ordering is no longer raw-input driven: same-level siblings should follow their referenced-neighbor barycenter when one exists, and fall back to a deterministic name order when barycenters tie.
  - focused 布局锚点还应继续证明关系种子排序不再受原始输入顺序直接驱动：同层兄弟表在存在已知引用邻居时应按邻居重心排序，而在重心相同的情况下则必须稳定回退到名称顺序。
  - Focused ER keyboard anchors should also prove `v` only toggles viewport mode once per frame after the router/render split, instead of flipping into viewport and immediately back to navigation.
  - focused ER 键盘锚点还应继续证明，在 router/render 拆分之后，`v` 每帧只切换一次视口模式，不会再出现“切进视口又立刻切回浏览”的双消费。
  - Focused layout anchors should also prove relation-first layout now respects actual card sizes: large related tables should not overlap after hierarchical seeding or after relation-seeded finalize.
  - focused 布局锚点还应继续证明关系优先布局现在会尊重真实表卡尺寸：大尺寸相关表在层级种子阶段和关系完成态之后都不应继续互相重叠。
  - Focused layout anchors should also prove relation-first seeding separates disconnected graph components before the global refine pass: unrelated relationship clusters should not start inside the same seeded band, and isolated tables should not be packed into a related cluster.
  - focused 布局锚点还应继续证明关系优先种子会在全局 refine 之前先拆开断开的图组件：互不相关的关系簇不应从同一条种子带起步，孤立表也不应被挤进一个已有关系的主簇里。
  - Focused layout anchors should also prove multi-component relation seeding no longer expands only as one long horizontal strip: once enough disconnected components exist, the seed layout should wrap them into multiple rows before the global refine pass.
  - focused 布局锚点还应继续证明多组件关系种子不再只沿单一横带无限展开：当断开组件足够多时，种子布局应在进入全局 refine 之前先把它们换排到多行。
  - Focused viewport anchors should also prove open-time fit-to-view can lower ER zoom below the previous quarter-scale floor when the default completed layout still does not fit, and that continuing to zoom out from such a state remains monotonic instead of snapping back up.
  - focused 视口锚点还应继续证明：当默认完成态在旧 `25%` 下限下仍装不进视口时，打开 ER 后的一次性 `fit_to_view()` 必须允许更低缩放；并且在这种状态下继续执行“缩小视图”时，不得反向跳回更大的缩放值。
  - Focused ER generation anchors should also prove the new semantic layer stays explicit: `analyze_er_graph()` must keep `Grid / Relation / Component` strategy selection deterministic for the same graph, and card/edge display modes must stay local presentation toggles rather than mutating the loaded graph.
  - focused ER 生成锚点还应继续证明新的语义层保持显式：`analyze_er_graph()` 对同一关系图必须稳定选择 `Grid / Relation / Component`，而表卡/边显示模式也必须始终只作为本地视图切换，而不是改写 loaded graph。
  - Focused incremental-stability anchors should also prove same-table reload keeps the current ER layout and partial-overlap reload keeps matching tables stable without freezing new ones: `begin_loading()` may preserve a staged snapshot across `clear()`, `finalize_er_diagram_load_if_ready()` must still restore exact matches directly, and when the table set drifts it should restore only overlapping table names after the new strategy-selected completed layout has already placed the new entries.
  - focused 增量稳定性锚点还应继续证明：同表集 reload 会保住当前 ER 布局，而表集部分变化时也会让交集表保持稳定、但不会冻结新表位置；`begin_loading()` 可以带着 staged snapshot 穿过 `clear()`，`finalize_er_diagram_load_if_ready()` 对完全一致的表集仍应直接恢复旧位置，而在表集漂移时则必须先跑新的策略完成态，再只恢复交集表的旧位置。
  - Focused incremental-stability anchors should also prove newly added tables do not immediately collide with restored old tables after a partial reload: once matching table names snap back to their old positions, local insertion/avoidance must still move the new entries off those restored rectangles.
  - focused 增量稳定性锚点还应继续证明：partial reload 之后新增表不会立刻撞回这些已恢复的旧表；当交集表回到旧位置后，本地插入/避让逻辑仍必须把新表从这些旧矩形上挪开。
  - Focused incremental-stability anchors should also prove relation-aware insertion remains local: if a newly added table is connected to restored old tables, it should re-anchor near those restored neighbors before the final avoidance pass, without rewriting app-level state or exact-match restore semantics.
  - focused 增量稳定性锚点还应继续证明：当 partial reload 里的新增表和已恢复旧表存在关系时，它不会只做纯几何避让，而会先贴近这些关系邻居，再做最后一层局部避让；同时这条逻辑必须继续保持本地布局层语义，不能顺手改写 app-level 状态或 exact-match 直恢复合同。
  - Focused incremental-stability anchors should also prove directional relation semantics survive incremental insertion: a newly added child table should re-anchor below restored parent tables, and a newly added parent table should prefer the area above restored child tables instead of drifting sideways.
  - focused 增量稳定性锚点还应继续证明：方向语义在增量插入里仍然有效；新增子表应优先落在已恢复父表下方，而新增父表则应优先落在已恢复子表上方，而不是只做无方向的侧向靠近。
  - Focused incremental-stability anchors should also prove bridge tables do not collapse below the whole child layer when both restored parents and restored children exist: the insertion resolver should keep the new bridge table inside the parent/child band whenever a non-overlapping local placement exists there.
  - focused 增量稳定性锚点还应继续证明：当已恢复父表和子表同时存在时，桥接表不会再退化成“直接掉到整个子层下面”；只要在父/子层之间存在可行的局部非重叠位置，插入解析器就应优先把新桥接表留在这条上下层带内。

## 5.1 UX / Input Recovery Live Smoke | UX / 输入恢复主线 live 冒烟

当 `toolbar / sidebar connection-row / AboutDialog / workspace dialog header / ER top-level combo` 这批条目进入“已关闭 / 观察”状态后，建议至少补一轮 live smoke，而不是只依赖 unit tests：

1. `Alt+A` 打开操作菜单，确认 chooser 正常打开、顶部 header 为紧凑双区块、`q` 可关闭。
2. `Alt+N` 打开新建菜单，确认 chooser 正常打开、顶部 header 为紧凑双区块、`q` 可关闭。
3. `Alt+K` 打开快捷键设置，确认顶部搜索/重置区与 breadcrumb/提示区在默认宽度下并排展示。
4. `F1` 打开帮助与学习，确认顶部快捷键提示区与 breadcrumb/提示区在默认宽度下并排展示。
5. `Ctrl+P -> about -> Enter` 打开 About，确认首屏不再回到旧的厚重双 section 堆叠。
6. 右键 connection-row，确认菜单稳定弹出；点击 `删连 / 删库` 后必须进入 `DeleteConfirm`，而不是只构造 helper target。
7. 在 ER 已打开时验证 `Ctrl+R` 的关/开，以及 `Alt+R` 只负责聚焦 ER。
8. 沿 `OpenLearningSample -> WelcomeSetup -> 5 -> Ctrl+Shift+N` 打开 `DdlDialog`，在约 `760x560` 级窄窗口下确认 footer 仍可见，且“列定义 / 预览 SQL” 不会再把整窗顶出视口。
9. 沿 `Ctrl+P -> sample -> Enter -> Ctrl+R -> Alt+R -> Shift+Right/Down/Left/Up` 进入学习样例 ER，确认 `Shift+Arrow` 的几何邻接没有明显误跳，且选中项仍会随视口进入可见区域。
10. 在同一学习样例 ER 链路下继续执行 `Shift+L -> Shift+Down -> f -> Ctrl+Shift+T -> j -> Enter`，确认 relayout / fit view / theme switch 不会把 ER 当前布局、缩放和几何邻接稳定性一起冲掉；若 theme chooser 关闭后要继续走 ER 局部键盘浏览，可显式再按一次 `Alt+R` 重新聚焦。

注意：
- 当前 Wayland live smoke 已确认 `Alt+A / Alt+N / Alt+K / F1 / Ctrl+P -> about` 这些显式路径通过。
- `DdlDialog` 的 live 窄视口复核也已通过：沿 `OpenLearningSample -> WelcomeSetup -> 5 -> Ctrl+Shift+N` 打开“创建表”后，footer 已重新保持可见。
- `Ctrl+R` 的自动注入验证目前仍不可靠：`wtype` 与 `hyprctl dispatch sendshortcut` 在当前 grid 场景下会落成 bare `r` 行为，因此这条必须优先用实体键手动复核；在拿到实体键复核前，不要仅凭自动注入失败就重新打开 ER bug。
- 命令面板链 `Ctrl+P -> toggle_er_diagram -> Enter` 现在已有 app-level focused regression 覆盖；如果 live 自动注入在这条链上失败，先检查注入是否选中了正确命令，而不要直接把它升级成新的 ER 打开回归。
- ER 几何邻接已完成第一轮 live 观察：当前 Wayland 会话下，沿 `Ctrl+P -> sample -> Enter -> Ctrl+R -> Alt+R -> Shift+Right/Down/Left/Up` 进入学习样例 ER 后，方向跳转与 `selection follows viewport` 没有复现明显误跳；当前先保持观察，不继续猜测性调算法。
- ER 几何邻接已完成第二轮组合观察：继续执行 `Shift+L -> Shift+Down -> f -> Ctrl+Shift+T -> j -> Enter` 后，relayout / fit view / theme switch 也没有把当前 ER 布局、缩放和几何邻接稳定性冲掉；当前仍先保持观察，不继续猜测性调算法。
- ER 关系优先布局已完成第三轮 live 观察：学习样例默认完成态下，关系种子 + 同层重心排序目前没有再出现明显“同层兄弟表直接继承输入顺序”的观感问题；若要继续复核 `Shift+L / f` 之后的 live 观感，优先用实体键或显式 `Alt+R` 回焦后的链路，不要把自动注入导致的焦点漂移误记成新的布局 bug。
- ER 默认打开后的可见范围第四轮 live 观察已完成：沿 `Ctrl+P -> sample -> Enter -> 5 -> Ctrl+R` 进入学习样例后，ER 现在会在默认完成态自动降到约 `12%`，整簇重新完整落回右侧可见区，底部表不再继续被旧 `25%` 下限卡在视口外。
- focused 布局锚点还应继续显式证明：多组件关系种子不仅会拆组件、换行，还会优先把较大的关系主簇锚定到左上区域，而不是让小型孤立表仅因名字更靠前就先占据主锚点。

## 6. Issue Reproduction Template | 问题复现模板

When filing or validating a bug:
提交或验证缺陷时建议记录：

- Version / 版本
- OS / 系统环境
- Steps / 复现步骤
- Expected vs Actual / 预期与实际
- Logs or screenshot / 日志与截图
