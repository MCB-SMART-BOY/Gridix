# Changelog

All notable changes to this project are documented in this file.  
本文件记录项目的重要变更。

## [Unreleased]

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
