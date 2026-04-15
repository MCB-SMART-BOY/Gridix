# Gridix 5.0.0 Bug Ledger

## Scope

这份账本只盘点 **当前 v5.0.0 发版收口时仍值得观察的根因问题**。  
来源限定为：

- `docs/recovery/10-master-recovery-plan.md`
- `docs/recovery/11-core-flows-and-invariants.md`
- `docs/README.md`
- `docs/TESTING.md`
- `src/app/runtime/*`、`src/app/surfaces/*`、`src/app/dialogs/*`
- `src/ui/dialogs/*`、`src/ui/components/er_diagram/*`、`src/ui/components/grid/*`
- `tests/*`
- `docs/CHANGELOG.md`

原则：

- 先按根因聚类，不把同一根因拆成很多表面症状。
- “症状” 与 “根因” 分开写。
- 不确定的地方明确标 `推测`。

## Priority Summary

| 优先级 | Bug ID | 标题 | 状态 | 建议 |
|---|---|---|---|---|
| 观察 | `G41-B007` | 剩余 dialog 横向失控来自固定宽度横排内容行 | 已完成主要 live 验证（转观察） | 除非出现新的窄视口回归，否则不再作为 active implementation workstream |
| 已关闭 | `G41-B005` | ER 图 workspace contract 已收口，`l` 语义已切到 TUI 主线 | 已修复（剩余为顶层回归覆盖薄弱） | 转入回归观察 |

## Open Issues

当前总复审结论：

- 这份账本里已没有“未阻塞且应立即继续实现”的 active bug。
- `G41-B007` 当前保留为 observation 条目；除非重新 live 复现，否则不再继续主动补丁。
- 后续若没有新的 confirmed bug，主线应转入阶段收口和发布准备。

### G41-B007

- Bug ID: `G41-B007`
- 标题: 剩余 dialog 横向失控来自固定宽度横排内容行
- 症状:
  - 一些 dialog 在窄窗口下仍会显得向右延伸、被内容顶宽，或只能靠裁剪勉强显示。
  - 问题已经不再主要出在外层 shell，也不再主要出在 toolbar popup，而是内容层横排 contract 缺失。
- 最小复现方式:
  1. 缩小窗口宽度。
  2. 依次打开 `CreateDbDialog`、`CreateUserDialog`、`ExportDialog`，必要时回看 `DdlDialog`。
  3. 观察路径行、浏览按钮行和列定义行是否继续向右挤压。
- 影响的核心功能:
  - `dialogs 打开 / 浏览 / 填表`
  - `窄视口可用性`
- 相关文件:
  - `src/ui/dialogs/import_dialog/mod.rs`
  - `src/ui/dialogs/create_db_dialog.rs`
  - `src/ui/dialogs/create_user_dialog.rs`
  - `src/ui/dialogs/export_dialog.rs`
  - `src/ui/dialogs/common.rs`
  - `docs/recovery/20-dialog-layout-audit.md`
  - `docs/recovery/43-dialog-responsive-row-design.md`
- 相关函数 / 类型 / 状态字段:
  - `DialogWindow::*`
  - 多处 `TextEdit::singleline(...).desired_width(...)`
  - 多处 `ui.horizontal(...)` 表单行
- 根因假设:
  - 多个 dialog 内容层仍要求“一行容纳多个固定宽度控件”，缺少窄窗口退化规则。
- 证据:
  - toolbar theme chooser 已在 [20](./20-dialog-layout-audit.md) 中收口为显式 overlay dialog，因此 `G41-B007` 当前剩余部分已经缩窄到表单内容层。
  - `ConnectionDialog` 的第一阶段已落地：footer 已迁到固定 shell，核心连接字段与路径/证书/私钥行已进入响应式退化 contract；当前 open cluster 已不再以它为主。
  - `ImportDialog` 的第一阶段已落地：footer 已迁到固定 shell，文件路径行、格式/模式区与 CSV / JSON 选项区已进入响应式退化 contract；当前 open cluster 已不再以它为主。
  - `DdlDialog` 的表信息行与列定义 dense row 已落地；本轮又把首帧外窗从通用 `WORKSPACE` 默认高收紧为 DDL 专用紧凑 profile，并将列区 / SQL 预览高度改成更保守的自适应值。当前 Wayland 会话下沿 `OpenLearningSample -> WelcomeSetup -> 5 -> Ctrl+Shift+N` 的 live 复核已通过，约 `760x560` 级窄视口中 footer 重新保持可见。
  - [43](./43-dialog-responsive-row-design.md) 已把剩余问题归并成 `FieldWithActionRow / FieldPairRow / DenseConfigRow / ValueHintRow` 四类契约，并明确了先后顺序。
  - `ExportDialog` 的稳定结果集 seed 已建立，且在 `960x620`、`860x620`、`760x560` 的第一轮 live 中未再现 footer 漂移；此前 `680x520` 的异常已通过单独的 render clamp 修复收口，不再继续混算为 dialog 行布局问题。
- 状态: `已完成主要 live 验证（转观察）`
- 优先级: `观察`
- 建议先“观察”还是先“修复”: `高频与低频长表单都已完成至少一轮 live；除非再次复现，否则先观察`
- 当前执行顺序:
  1. `CreateDbDialog` 已完成最小修复，并在当前 Wayland 会话下通过等价 live 复核；精确 `960x620` 点仍受窗口管理器高度钳制影响
  2. `ExportDialog` 的稳定结果集 seed 已建立：`OpenLearningSample -> WelcomeSetup -> Enter/5`
  3. `ExportDialog` 已完成第一轮窄视口 live：`960x620` 到 `760x560` 未再现 footer 漂移；此前更小视口触发的 render panic 已独立修复
  4. `DdlDialog` 已完成 live 桌面回归：当前 Wayland 会话下沿 `OpenLearningSample -> WelcomeSetup -> 5 -> Ctrl+Shift+N` 重新打开“创建表”，窄窗口里 footer 可见，问题收敛到 DDL 专用紧凑窗口 profile + 自适应局部高度
  5. `CreateUserDialog` 已完成第一轮 non-SQLite live：本轮先在用户态初始化临时 MariaDB 实例（`127.0.0.1:33306`），再通过临时 `XDG_CONFIG_HOME` 注入一条只供本轮验证使用的 MySQL 连接；沿 `Ctrl+B -> Ctrl+1 -> Enter -> Ctrl+Shift+U` 成功拉起 `CreateUserDialog`。
  6. 在约 `900x650` 的浮动窄窗下，`CreateUserDialog` 的 footer 保持可见，主体滚动与局部 SQL 预览滚动没有新冲突，`q` 关闭合同也正常；当前已不再属于“环境阻塞”，而是转入后续 observation。

## Resolved During Recovery

### G41-B006

- Bug ID: `G41-B006`
- 标题: Toolbar action/create 菜单仍是 raw popup：焦点不可见、owner 缺失、键盘流割裂
- 症状:
  - 顶部栏 `"打开操作菜单"` / `"打开新建菜单"` 从用户视角看不能稳定被 `hjkl` 选中。
  - 打开后它们不像独立 overlay，而更像匿名 popup；键盘模式切换不清晰。
  - 这两个菜单不服从现有 dialog owner / keyboard-first contract。
- 最小复现方式:
  1. 将焦点切到 toolbar。
  2. 使用 `h/l` 在顶部图标之间移动到 `"⚡"` 或 `"+"`。
  3. 观察这两个触发器是否有稳定焦点高亮。
  4. 按 `Enter` 打开菜单，再尝试继续按键盘进行层级操作。
- 影响的核心功能:
  - `toolbar 键盘流`
  - `dialogs / overlays 打开与选择`
  - `keyboard-first + mouse trigger` 一致性
- 相关文件:
  - `src/ui/components/toolbar/mod.rs`
  - `src/ui/dialogs/toolbar_menu_dialog.rs`
  - `src/app/dialogs/host.rs`
  - `src/app/surfaces/dialogs.rs`
  - `src/app/input/input_router.rs`
  - `docs/recovery/20-dialog-layout-audit.md`
- 相关函数 / 类型 / 状态字段:
  - `Toolbar::show_action_buttons()`
  - `Toolbar::handle_keyboard()`
  - `ToolbarMenuDialog`
  - `ToolbarMenuDialogState`
  - `toolbar_actions_menu_state`
  - `toolbar_create_menu_state`
  - `active_dialog_owner`
- 根因假设:
  - trigger 按钮虽然在 `toolbar_index` 里，但没有焦点可见性。
  - popup 只存在于 `egui::Area + ctx.data_temp`，没有 app 级 owner。
  - toolbar 导航和 popup 导航是两套局部输入，没有显式模式切换状态。
- 证据:
  - [20](./20-dialog-layout-audit.md) 已合并保存 toolbar `action/create` 从 raw popup 迁到显式 overlay dialog 的根因与修复结果。
  - 旧 `src/ui/components/toolbar/dropdowns.rs` 已移除，菜单状态不再放在匿名 `ctx.data_temp(...)`。
  - 后续补丁已把 `Action::OpenToolbarActionsMenu` / `Action::OpenToolbarCreateMenu` 接入 `AppAction` 和 workspace fallback 快捷键路径；toolbar trigger 的 tooltip 也不再继续误报共享 `ToolbarActivate`，而是显示独立的真实绑定。
- 状态: `已修复`
- 优先级: `已关闭`
- 建议先“观察”还是先“修复”: `已在 [20](./20-dialog-layout-audit.md) 收口`
- 后续收口说明:
  - `src/ui/components/toolbar/mod.rs` 里的 `"⚡"` / `"+"` trigger 现在分别由 `Action::OpenToolbarActionsMenu` / `Action::OpenToolbarCreateMenu` 暴露独立入口，默认绑定为 `Alt+A` / `Alt+N`。
  - `src/ui/components/toolbar/utils.rs`、`src/ui/components/toolbar/theme_combo.rs` 与 `src/ui/components/er_diagram/render.rs` 现已把 toolbar / ER toolbar trigger 收口为“默认透明，仅在 hover / focus / selected 时显示 chrome”的交互式按钮；此前 follow-up 回归中出现的常态灰底已收口。
  - `src/ui/dialogs/toolbar_menu_dialog.rs` 里的 chooser 外窗现在走 `DialogWindow::workspace(...)`，保留 chooser 级默认尺寸但允许用户拖拽缩放；顶部信息区也从双整宽堆叠块压缩为紧凑双区块。
  - `toolbar.menu.dismiss` 默认绑定已从仅 `Esc` 扩展为 `Esc / Q`，因此 action/create chooser 现在和 `KeyBindingsDialog`、`HelpDialog` 一样共享显式 dialog 关闭语法。
  - `src/ui/dialogs/keybindings_dialog.rs` 的 scope 树与局部命令标签现已把 `toolbar.menu.*` / `toolbar.theme.*` 归到 `dialog.toolbar_menu` / `dialog.toolbar_theme`，与运行时显式 dialog owner 一致；真正的 `toolbar` 节点只再承载 toolbar 焦点导航与 trigger 激活语义。

### G41-B008

- Bug ID: `G41-B008`
- 标题: `WelcomeSetup` 没有独立键盘 contract，`Tab / Enter` 会泄漏到背景 toolbar
- 症状:
  - `OpenLearningSample` 打开安装/初始化引导后，`Tab / Enter` 不能稳定驱动 guide 底部动作。
  - live 使用中，按键可能泄漏到背景 toolbar，甚至打开 `⚡` action chooser，而不是执行 guide 中的“首条查询”。
  - 这会直接阻塞 `ExportDialog` 的 live seed，因为学习示例连接虽然建立了，但无法稳定用纯键盘触发示例查询。
- 最小复现方式:
  1. 用 `Ctrl+P` 打开命令面板。
  2. 运行 `打开学习示例库`。
  3. 在 `WelcomeSetup` 里按 `Tab`、`Enter`。
  4. 观察输入是否仍留在 guide 内，还是泄漏到背景 toolbar / chooser。
- 影响的核心功能:
  - `dialogs 打开 / 关闭 / 确认 / 取消`
  - `keyboard-first onboarding`
  - `ExportDialog` live seed
- 相关文件:
  - `src/app/workflow/welcome.rs`
  - `src/core/commands.rs`
  - `src/app/action/action_system.rs`
  - `src/app/runtime/database.rs`
  - `src/app/mod.rs`
  - `docs/recovery/20-dialog-layout-audit.md`
- 相关函数 / 类型 / 状态字段:
  - `DbManagerApp::open_welcome_setup_dialog()`
  - `DbManagerApp::detect_welcome_setup_key_action()`
  - `DbManagerApp::run_welcome_setup_action()`
  - `DbManagerApp::show_welcome_setup_dialog_window()`
  - `WelcomeSetupAction`
  - `welcome_setup_action_index`
- 根因假设:
  - `DialogScope::WelcomeSetup` 存在，但 guide 自身没有局部动作索引和 scoped command 消费链。
  - 底部按钮只是普通 `ui.button(...)`，没有把 `Tab / Enter / 数字键` 绑定到当前 dialog owner。
- 证据:
  - live 复现时，旧行为会把 `Tab / Enter` 泄漏到背景 toolbar。
  - 当前修复后，`WelcomeSetup` 已有显式动作列表、选中索引和 `dialog.welcome_setup.*` 命令。
  - live 复核已确认：`Enter` 和 `5` 会在 guide 内消费，并稳定触发 `SELECT 1 AS hello;`；随后 `Ctrl+E` 可打开 `ExportDialog`。
  - 针对 `welcome_setup_*` scoped command 的单元测试已落地。
- 状态: `已修复`
- 优先级: `已关闭`
- 建议先“观察”还是先“修复”: `已在 [20](./20-dialog-layout-audit.md) 收口`

### G41-B009

- Bug ID: `G41-B009`
- 标题: 小视口下 dialog live 回归会触发全局 SQL editor 高度 clamp 崩溃
- 症状:
  - 在足够小的 app 视口里打开 dialog 时，app 会直接 panic，而不是只出现 dialog 排版问题。
  - 已证实路径是：`OpenLearningSample -> WelcomeSetup -> Enter/5 -> Ctrl+E -> ExportDialog`，然后把 app 视口压到极小尺寸。
- 相关文件:
  - `src/app/surfaces/render.rs`
  - `docs/recovery/20-dialog-layout-audit.md`
- 根因假设:
  - `render.rs` 里两处 SQL editor 高度分配都直接执行 `self.sql_editor_height.clamp(100.0, available_height * 0.6)`；当剩余高度不足时会进入 `min > max`。
- 证据:
  - live 复现时 panic 文本为：`min > max, or either was NaN. min = 100.0, max = 28.406252`
  - focused 测试现已锁住 tiny viewport 下的安全收缩边界
- 状态: `已修复`
- 优先级: `已关闭`
- 建议先“观察”还是先“修复”: `已在 render 主线中收口`

### G41-B010

- Bug ID: `G41-B010`
- 标题: Sidebar connection-row destructive entrypoints drift after the custom header regression
- 症状:
  - 连接项右键菜单从用户视角看像是“消失了”，实际很难稳定触发。
  - 连接头部的 `删连 / 删库` 鼠标入口与键盘 `d` 不再形成同一条显式 destructive flow，用户感知为“按钮没作用但键盘能删”。
  - “删除数据库”运行时链路仍在，但 connection-row 上的鼠标入口退化后，功能看起来像被移除了。
- 最小复现方式:
  1. 在侧边栏连接列表中定位一个连接头部。
  2. 右键连接头部，观察菜单是否稳定出现。
  3. 点击 `删连` 或 `删库`，确认是否进入统一删除确认。
  4. 再切到 `Connections / Databases` section 用 `d` 比较目标是否一致。
- 影响的核心功能:
  - `destructive actions`
  - `sidebar 鼠标 / 键盘一致性`
- 相关文件:
  - `src/ui/panels/sidebar/actions.rs`
  - `src/ui/panels/sidebar/connection_list.rs`
  - `src/ui/panels/sidebar/table_list.rs`
  - `src/ui/panels/sidebar/mod.rs`
  - `src/app/surfaces/render.rs`
  - `src/app/surfaces/dialogs.rs`
- 相关函数 / 类型 / 状态字段:
  - `SidebarDeleteTarget`
  - `ConnectionList::delete_targets_for_context()`
  - `ConnectionList::request_connection_delete()`
  - `ConnectionList::request_database_delete()`
  - `ConnectionList::request_table_delete()`
  - `handle_delete_action()`
  - `pending_delete_target`
  - `confirm_pending_delete()`
- 根因假设:
  - custom header 改造后，connection-row 右键菜单绑定在不稳定的 header layout response 上，而不是显式可交互的 header row response。
  - 连接头部按钮、右键菜单、键盘 `d` 各自内联构造删除目标，导致 destructive entrypoint 漂移且缺少 focused coverage。
- 证据:
  - connection-row 右键菜单内容本身没有被删除，`show_delete_targets_menu()` 仍在；问题集中在 trigger surface 与入口漂移，而不是 confirm/runtime 链丢失。
  - `SidebarDeleteTarget` 现已新增统一构造 helper，connection-row 右键菜单、头部删连/删库按钮、keyboard `d` 现都通过同一组 helper 生成目标 payload。
  - connection-row 头部不再给整个 `ui.horizontal(... )` 容器额外叠一层父级 `Sense::click()` 响应；右键菜单现在只挂在 label/toggle 组合交互面上，右侧 action buttons 则保留独立 pointer surface。
  - `handle_sidebar_actions()`、`DeleteConfirm` 和 `confirm_pending_delete()` 的 app-level destructive authority 没有改变，恢复后仍继续走同一确认链。
  - focused coverage 现已向 app-level 补齐：`src/app/surfaces/render.rs` 锁住 `actions.delete -> pending_delete_target -> DeleteConfirm`，`src/app/surfaces/dialogs.rs` 锁住 `confirm_pending_delete()` 对已保存 target 的分发与清理，不再只停留在 sidebar helper 层。
  - focused tests 现已补上 connection header 交互面与右侧按钮组分离的结构覆盖，以及 connection-row payload helper 与 keyboard `d` 的收敛关系。
- 状态: `已修复`
- 优先级: `已关闭`
- 建议先“观察”还是先“修复”: `已在 sidebar destructive flow 中收口`

### G41-B011

- Bug ID: `G41-B011`
- 标题: `AboutDialog` 退化成通用 section 堆叠页，丢失旧版品牌锚点与信息密度
- 症状:
  - “关于 Gridix”从用户视角看更像普通说明页，而不是有完成度的产品 about 页面。
  - 当前版的 hero 很弱，正文被两张等权 section 卡分散，品牌感和项目锚点都不够清晰。
  - 旧版本里更紧凑的 manifesto 卡和元信息区已经消失，视觉上显得笨重。
- 最小复现方式:
  1. 打开 “关于 Gridix”。
  2. 观察当前版是否仍是 “hero + 产品定位卡 + 项目信息卡 + footer” 的纵向堆叠。
  3. 对比旧版 about 的单 manifesto 卡和紧凑元信息区，评估第一屏是否还能先看到品牌锚点。
- 影响的核心功能:
  - `dialog 设计语言一致性`
  - `about / 品牌信息呈现`
- 相关文件:
  - `src/ui/dialogs/about_dialog.rs`
  - `src/ui/dialogs/common.rs`
  - `src/app/surfaces/dialogs.rs`
  - `docs/recovery/20-dialog-layout-audit.md`
- 相关函数 / 类型 / 状态字段:
  - `AboutDialog::show()`
  - `DialogWindow::standard(...)`
  - `DialogFooter::show_close_only(...)`
- 根因假设:
  - shell 统一后，about 内容层直接套用了通用 section 语言，没有保留旧版更强的品牌 hero、单 manifesto 卡和紧凑项目事实条带。
- 证据:
  - `v4.0.0` 的 `about_dialog.rs` 与当前实现几乎一致，问题不是最近一刀引入，而是更早的内容层退化一直保留到了 4.1.0 主线。
  - `3.6.0` 与更早版本的 `about_dialog.rs` 都保留居中的品牌头、轻量信息层级和更有记忆点的语气；用户反馈也明确更偏好这条旧主线，而不是更“规整”的厚重品牌页。
  - 当前修复后，`AboutDialog` 仍保持 `DialogWindow::standard(...)` 的简单壳层，但内容层已经回摆成“居中品牌头 + 单 manifesto 卡 + 轻量项目速览 + footer”，不再继续使用厚重的 facts strip。
- 状态: `已修复`
- 优先级: `已关闭`
- 建议先“观察”还是先“修复”: `已在 about dialog 内容层回摆并收口`

### G41-B012

- Bug ID: `G41-B012`
- 标题: `KeyBindingsDialog` 与 `HelpDialog` 顶部连续堆叠双 toolbar，浪费首屏高度
- 症状:
  - `快捷键设置` 顶部的搜索/重置区和 breadcrumb/鼠标提示区连续纵向堆叠，占掉了过多首屏空间。
  - `帮助与学习` 顶部的快捷键提示区和 breadcrumb/鼠标提示区也沿用了同样的双整宽堆叠。
  - 在默认 workspace dialog 宽度下，这两块本可以并排展示，却仍被硬拆成上下两行。
- 最小复现方式:
  1. 打开 `快捷键设置`。
  2. 观察搜索/重置区和 breadcrumb/鼠标提示区是否仍然上下堆叠。
  3. 再打开 `帮助与学习`，观察快捷键提示区和 breadcrumb/鼠标提示区是否同样纵向堆叠。
- 影响的核心功能:
  - `workspace dialog header 信息密度`
  - `帮助 / 快捷键设置的设计语言一致性`
- 相关文件:
  - `src/ui/dialogs/keybindings_dialog.rs`
  - `src/ui/dialogs/help_dialog.rs`
  - `src/ui/dialogs/picker_shell.rs`
  - `docs/recovery/20-dialog-layout-audit.md`
- 相关函数 / 类型 / 状态字段:
  - `PickerDialogShell::header_blocks_layout()`
  - `PickerDialogShell::header_blocks()`
  - `WorkspaceDialogShell::show()`
  - `KeyBindingsDialog::show()`
  - `HelpDialog::show()`
- 根因假设:
  - 两个 dialog 都在 `WorkspaceDialogShell` 顶部连续渲染两段整宽 toolbar，但没有共享一个“宽度允许时并排展示”的 header 组合 helper。
- 证据:
  - `KeyBindingsDialog` 与 `HelpDialog` 原来都在 `header` / `subheader` 两个 shell 槽位里连续放置整宽 `DialogContent::toolbar(...)`。
  - 现已新增 `PickerDialogShell::header_blocks_layout()` 与 `header_blocks()`，并让两个 dialog 在默认宽度下优先以内联双区块展示 header。
  - focused tests 已锁住共享阈值，以及 `KeyBindingsDialog` / `HelpDialog` 默认宽度下都会选择 inline header blocks。
- 状态: `已修复`
- 优先级: `已关闭`
- 建议先“观察”还是先“修复”: `已在共享 header helper 中收口`

### G41-B013

- Bug ID: `G41-B013`
- 标题: DataGrid 结果表格列头在暗色主题下退化成“只剩焦点列可见”
- 症状:
  - 表格上方的列头一行里，看起来只有当前焦点列名清晰可见。
  - 其余非焦点列不是没数据，而是列头文字在当前主题下退化成近乎不可见。
  - 用户会误以为列结构丢失，或把 `.` / 筛选小标记误当成唯一可见信息。
- 最小复现方式:
  1. 打开任意带较多列的查询结果表格。
  2. 保持暗色主题。
  3. 观察未聚焦、未筛选的列头文字是否接近消失，而只有焦点列仍清晰。
- 影响的核心功能:
  - `查询结果展示`
  - `结果表格可读性`
- 相关文件:
  - `src/ui/components/grid/render.rs`
  - `src/ui/styles.rs`
  - `docs/recovery/11-core-flows-and-invariants.md`
- 相关函数 / 类型 / 状态字段:
  - `render_column_header()`
  - `column_header_text_color()`
  - `DataGridState.cursor`
  - `DataGridState.filters`
- 根因假设:
  - `render_column_header()` 之前只给焦点列和筛选列设置了显式颜色，非焦点、非筛选列继续依赖隐式前景色；在当前暗色主题组合下，这条隐式颜色链会退化成近乎不可见。
- 证据:
  - 焦点列原本显式使用 `state.mode.color()`，筛选列显式使用筛选高亮，而未聚焦列仅 `RichText::new(col_name).strong()`，没有显式颜色。
  - 当前修复已新增 `column_header_text_color()`，将列头颜色收口为 `焦点列 = mode accent`、`筛选列 = 筛选高亮`、其余列 = `theme_text(visuals)`。
  - focused tests 已锁住三条分支：`unfocused_unfiltered_column_header_uses_theme_text_color`、`focused_column_header_keeps_mode_accent_color`、`filtered_column_header_keeps_filter_highlight_color`。
  - 当前 Wayland live smoke 也已通过：现有运行中的学习样例结果表 `customer_addresses` 截图中，非焦点列头已经恢复可读，不再出现“只剩焦点列名清晰、其余列头近乎消失”的现场症状。
- 状态: `已修复`
- 优先级: `已关闭`
- 建议先“观察”还是先“修复”: `已在 grid header render 层收口；后续只保留 live 观感观察`

### G41-B004

- Bug ID: `G41-B004`
- 标题: 少量 utility overlay 与 blocking confirm contract 仍未统一
- 症状:
  - utility overlay 若继续绕过 shared shell contract，仍可能在小视口下出现约束不一致。
  - destructive confirm 若继续使用普通 `Window` 而不是明确的 blocking contract，键盘流和视觉语义仍会不够清晰。
- 最小复现方式:
  1. 缩小窗口尺寸。
  2. 依次打开帮助、快捷键设置、import、create user、DDL 等 dialog。
  3. 再打开命令面板、历史面板，并触发删除确认 / grid 保存确认，观察这些 overlay 的壳层约束和阻塞语义是否仍不一致。
- 影响的核心功能:
  - `dialogs 打开 / 关闭 / 确认 / 取消`
  - `键盘输入所有权`
- 相关文件:
  - `src/app/dialogs/host.rs`
  - `src/app/surfaces/dialogs.rs`
  - `src/ui/dialogs/common.rs`
  - `src/ui/dialogs/confirm_dialog.rs`
  - `src/app/action/command_palette.rs`
  - `src/ui/components/grid/mod.rs`
  - `src/ui/panels/history_panel.rs`
  - `tests/ui_dialogs_tests.rs`
  - `docs/CHANGELOG.md`
  - `docs/recovery/20-dialog-layout-audit.md`
- 相关函数 / 类型 / 状态字段:
  - `render_dialogs()`
  - `DialogWindow::standard / resizable / workspace / fixed`
  - `ConfirmDialog::show()`
  - `HistoryPanel::show()`
  - `render_command_palette()`
  - `DataGrid::show_save_confirm_dialog()`
  - `DataGrid::show_goto_dialog()`
- 根因假设:
  - dialog owner、blocking modal、workspace shell 与长表单 shell 虽已收口，但少量 overlay 一致性曾长期分散在多份修复记录中。
  - destructive confirm 和 utility overlay 还没有收敛成明确的交互类别。
- 证据:
  - [20](./20-dialog-layout-audit.md) 已合并保存共享 shell 视口约束、owner authority、blocking confirm、workspace layered picker、长表单 shell、toolbar chooser overlay 等收口结果。
  - `tests/ui_dialogs_tests.rs` 仍主要覆盖通用结构，不覆盖真实 viewport overflow / pane content 反向撑窗。
  - `docs/CHANGELOG.md` 在 `4.0.0` 和 `4.1.0` 都多次记录 dialog 宽度、fixed-size、resizable、picker shell overflow 的修复，说明这是重复回归簇。
- 状态: `已修复（剩余仅为低优先级 utility overlay 一致性）`
- 优先级: `已关闭`
- 建议先“观察”还是先“修复”: `已在 [20](./20-dialog-layout-audit.md) 分步收口`

### G41-B005

- Bug ID: `G41-B005`
- 标题: ER 图 workspace contract 已收口，`l` 语义已切到 TUI 主线
- 症状:
  - ER 图的视觉语言可能继续和主工具栏 / dialog / grid 不一致，尤其在主题切换下更明显。
  - 关闭、重开、刷新 ER 图时，workspace ownership 与内部状态边界仍需继续冻结，后续改动容易引入串状态。
  - ER 当前已经进入显式键盘工作区主线；关系邻接与几何邻接都已进入 additive 浏览层。结合现有 `OpenSelectedTable -> QuerySelectedTable -> DataGrid` 主链，内部独立 detail mode 当前不进入实现；若后续直接扩展，很容易把浏览语义和视口语义重新搅回同一层。
- 最小复现方式:
  1. 在浅色主题下打开 ER 图。
  2. 切换关闭 / 打开 / 刷新路径，并从帮助页和普通入口分别进入。
  3. 观察工具栏、空态、loading、选中/拖拽/缩放反馈是否与主 UI 风格一致，以及关闭后是否还残留 ER input owner。
- 影响的核心功能:
  - `ER 图打开与交互`
  - `主题一致性`
- 相关文件:
  - `src/app/input/input_router.rs`
  - `src/app/workflow/help.rs`
  - `src/app/runtime/er_diagram.rs`
  - `src/app/runtime/handler.rs`
  - `src/ui/components/er_diagram/state.rs`
  - `src/ui/components/er_diagram/render.rs`
  - `src/ui/mod.rs`
  - `docs/recovery/10-master-recovery-plan.md`
  - `docs/recovery/11-core-flows-and-invariants.md`
  - `docs/recovery/44-er-ownership-and-design-audit.md`
  - `docs/recovery/47-er-workspace-and-keyboard-contract.md`
  - `docs/recovery/48-er-visibility-entry-matrix-and-state-ledger.md`
  - `docs/recovery/49-er-keyboard-flow-graph.md`
  - `docs/recovery/50-er-token-map.md`
- 相关函数 / 类型 / 状态字段:
  - `show_er_diagram`
  - `ERDiagramState.loading / pan_offset / zoom / selected_table`
  - `set_er_diagram_visible()`
  - `load_er_diagram_data()`
  - `handle_foreign_keys_fetched()`
  - `handle_er_table_columns_fetched()`
  - `consume_er_diagram_key_action()`
  - `FocusArea`
  - `RenderColors::from_theme(theme, visuals)`
- 根因假设:
  - ER 图的显隐权威已经收口到 app 层 `show_er_diagram`，历史上的最后一个 open contract 是 `l` 的最终语义。
  - ER 的 lifecycle 一直缺少“FK 请求是否完成”和“哪些表列请求仍在等待”的显式状态，所以 `loading` 与 ready 通知容易受异步顺序影响。
  - ER 原先既缺显式键盘 owner，也缺关系浏览器式的本地导航 contract。
  - ER 图的视觉 token 体系虽然已经完成两波收口，但仍保留“从主主题派生的 ER 私有语义”；这已不再构成当前 open bug，只留下顶层回归覆盖薄弱。
- 证据:
  - `src/ui/components/er_diagram/state.rs` 已删除 `ERDiagramState.show`，运行期显隐继续只由 `show_er_diagram` 驱动。
  - `src/app/workflow/help.rs` 与 `src/app/input/input_router.rs` 先前的直接布尔写入，现已统一收口到 `set_er_diagram_visible_with_notice(...)`；这说明显隐权威入口已开始变成明确 contract，而不是散落兼容路径。
  - `ERDiagramState` 现已新增 `pending_column_tables / foreign_key_columns / foreign_keys_resolved`，`loading` 不再由单个 FK 回包直接结束。
  - `handle_er_table_columns_fetched()` 现在会按缓存的 `foreign_key_columns` 投影 `is_foreign_key`，因此 FK 结果先到、列结果后到时不会再把外键徽标冲掉。
  - ER 的 ready 提示已从 FK 回包阶段移到 `finalize_er_diagram_load_if_ready()`，因此关系通知、推断关系和最终 layout 现在在同一阶段决定。
  - `src/ui/mod.rs` 现已新增 `FocusArea::ErDiagram`，并进入 workspace 主循环。
  - `src/ui/components/er_diagram/render.rs` 现在只在 ER 显式聚焦时消费局部快捷键，鼠标 hover 不再抢键。
  - `src/app/input/input_router.rs` 现已把 `j/k/Enter/Right/h/Left/Esc/q` 收口为 ER scope 的显式本地语义：线性选表、打开当前表、返回最近一个合法的非 ER workspace 区域、关闭 ER。
  - `InputContextSnapshot::focus_scope()` 现已在 `show_er_diagram == false` 时拒绝把 `FocusArea::ErDiagram` 继续解释为 `FocusScope::ErDiagram`，隐藏 ER 不再保留局部 input owner。
  - `src/ui/components/er_diagram/state.rs` 现已为 ER 增加稳定线性选表 helper，并优先按 app 当前 `selected_table` 恢复首次选中项。
  - `src/ui/components/er_diagram/state.rs` 与 `render.rs` 现已补上“selection follows viewport”：键盘切换选中项或重新进入 ER scope 后，当前选中表会在下一帧自动滚回可见区域，不再出现“选中了但画布没跟上”。
  - `src/core/keybindings.rs`、`src/app/action/action_system.rs` 与 `src/app/input/input_router.rs` 现已补上显式 `FocusErDiagram` / `focus_er_diagram`；默认 `Alt+R` 只在 ER 已打开时可用，只切入 `FocusArea::ErDiagram`，不再把“显式入焦”继续混在 `ToggleErDiagram` 的 open/close 语义里。
  - `src/ui/components/er_diagram/state.rs` 现已新增局部 `interaction_mode`，默认 `Navigation`；`clear()` 会保守重置回浏览模式，避免 reload 之后继续残留在旧视口键盘状态。
  - `src/app/input/input_router.rs` 现已把 ER scope 继续细分为 `FocusScope::ErDiagram(Navigation)` 与 `FocusScope::ErDiagram(Viewport)`；`v` 负责在两者间切换，视口模式内的 `Esc` 只退出到浏览模式，不再直接离开 ER。
  - `src/core/commands.rs`、`src/ui/shortcut_tooltip.rs` 与 `src/ui/dialogs/keybindings_dialog.rs` 现已补上 `er_diagram.viewport.*` scoped commands，因此视口模式下的 `h/j/k/l` 平移与 `Esc` 退出不再继续借用匿名 raw-key 路径。
  - `src/ui/components/er_diagram/render.rs` 现已把 `h/j/k/l` 平移限制在视口模式内；浏览模式仍保留关系浏览语义，`q / r / Shift+L / f / +/-` 在两种局部模式间保持稳定。
  - `src/app/input/input_router.rs` 现已补上视口模式下 `q` 仍路由到 `CloseDiagram` 的 focused test，避免 `q` 在次级局部模式里意外失效。
  - `src/ui/components/er_diagram/render.rs` 现已补上视口模式下 `r / Shift+L / f` 仍保持可用的 focused test，避免引入“切到视口模式后顶层画布动作被一起关掉”的回归。
  - `src/ui/components/er_diagram/state.rs` 现已补上 `begin_loading()` 会把 `interaction_mode` 保守重置回 `Navigation` 的 focused test，确保 open / refresh / reload 不会把旧的视口模式带进新一轮加载。
  - `src/ui/components/er_diagram/render.rs` 现已补上跨 `ThemePreset` 渲染不会改写 `interaction_mode / selected_table / pan_offset / zoom` 的 focused test，确保 theme switch 当前只影响视觉 token，不会把 ER 的局部浏览/视口状态洗掉。
  - `src/core/commands.rs`、`src/ui/shortcut_tooltip.rs` 与 `src/app/input/input_router.rs` 现已补上 `er_diagram.prev_related / next_related`；浏览态新增 `Shift+K / Shift+J`，会按稳定全局表顺序在当前表的关联集合内前进/后退，不替换原有 `j/k` 的线性选表。
  - `src/ui/components/er_diagram/state.rs` 现已补上关系邻接导航的 focused tests：关联集合会按稳定全局表顺序解释，双向/重复关系会被去重，而没有关联项时命令会保持 no-op。
  - `src/core/commands.rs`、`src/ui/shortcut_tooltip.rs` 与 `src/app/input/input_router.rs` 现已补上 `er_diagram.geometry_left / geometry_down / geometry_up / geometry_right`；浏览态新增 `Shift+Left / Shift+Down / Shift+Up / Shift+Right`，会按当前表卡几何中心和方向选择最近邻，不替换 `j/k` 线性选表或 `Shift+J / Shift+K` 关系邻接。
  - `src/ui/components/er_diagram/state.rs` 现已补上几何邻接导航的 focused tests：优先选择请求方向内的最近候选；该方向无候选时保持 no-op；若仅存在同方向对角候选，则按保守回退选择。
  - `src/app/input/input_router.rs` 现已补上几何邻接的边界 focused tests：`Shift+Arrow` 在 `er_diagram.viewport` 中保持 `NoOp`，且几何邻接在 `OpenSelectedTable` 之前不会提前污染 app 层 `selected_table`。
  - `src/app/input/input_router.rs` 中 `ErDiagramLocalAction::OpenSelectedTable` 当前会先把 ER 局部 `selected_table` 映射到 app 层 `selected_table`，随后直接分发 `AppAction::QuerySelectedTable` 并把焦点回落到 `DataGrid`；而 `src/app/action/action_system.rs` 中 `selected_table_query_effects()` 会继续切 `grid workspace`、执行 `SELECT *` 并拉主键。当前并不存在承接 ER 内部 detail pane 的独立状态或 render 容器，因此 detail 仍由主工作区承载。
  - `src/core/commands.rs` 现已把 bare `l` 改到 `er_diagram.open_selected`，并把 `er_diagram.layout` 改到 `Shift+L`；ER 第一阶段语义现已完成从保守版 `方案 B` 到更强 `h/l` 主线的切换。
  - `src/app/runtime/handler.rs` 现已移除“FK 成功但空结果时提前推断”的旁路；无论 FK 返回空列表还是 FK 请求报错，关系推断现在都统一延迟到 `finalize_er_diagram_load_if_ready()`。
  - `src/app/runtime/handler.rs` 现已为 `explicit / inferred / empty` 三种 ready-state 决策，以及“空 FK 结果保持为空显式关系”的分支补上 focused tests；finalize/FK fallback contract 已收口。
  - `src/ui/components/er_diagram/render.rs` 现已将 card/background、text、border、selection、toolbar chrome，以及 `grid_line / relation_line / pk_icon / fk_icon / table_shadow / text_type` 全部收口到 `ThemePreset::colors()` + `egui::Visuals` 的派生规则。
  - `src/ui/components/er_diagram/render.rs` 现已把 ER 顶部工具栏按钮的 chrome 改成交互态显示：默认透明，hover / focus 时临时显现，视口模式按钮保留显式 selected 态，不再常驻灰底。
  - `src/app/input/input_router.rs` 现已把 `ToggleErDiagram` 从通用 workspace overlay gate 里单独拆出，因此 `Ctrl+R` 在 `FocusScope::ErDiagram(Navigation/Viewport)` 下也仍会路由到 toggle；与此同时，`ShowHistory` 仍保持原有 gate，不会因为进入 ER scope 被一起放开。
  - `src/app/input/input_router.rs` 本轮又把 `ToggleErDiagram` 的打开分支收口为“显隐 + 自动入焦 ER”：ER 从隐藏切到可见时会直接进入 `FocusArea::ErDiagram`，因此结果表格已打开的场景下，`h/j/k/l` 不会继续被 `DataGrid` 抢走；`Alt+R` 继续保留为 ER 已打开但焦点丢失时的显式回焦入口。
  - `src/app/action/action_system.rs` 现已补上 `ToggleErDiagram` 命令注册/可用性与 ER 聚焦状态栏的 focused tests；`src/ui/dialogs/help_dialog/topic_content.rs` 也已锁住 Help->Relationships 学习入口仍保留“自动打开学习示例 ER 图”动作。
  - `src/app/input/input_router.rs` 现已把“关闭 ER 后是否、以及回到哪个 workspace focus”的判断抽成纯 helper，并补上 focused tests；close -> focus restore 不再只靠运行期路径旁证。
  - `src/app/runtime/er_diagram.rs` 现已把 open / refresh 共享的 load planning 抽成纯 helper，并补上“无 active connection / 空表 / 正常加载”三条 focused tests；顶层 open/refresh 不再只靠 UI 路径旁证。
  - `src/app/surfaces/render.rs` 现已把 `ERDiagramResponse -> app-level focus/refresh/layout/fit_view` 的 surface dispatch 抽成纯 helper，并补上 focused tests；ER render 侧的 open/refresh UI 组合分发不再只靠内联分支旁证。
  - `docs/recovery/10` 和 `11` 都把 ER 列为单独 workstream，并明确指出顶层测试覆盖薄弱。
  - [44](./44-er-ownership-and-design-audit.md) 已收口审计边界；[47](./47-er-workspace-and-keyboard-contract.md)、[48](./48-er-visibility-entry-matrix-and-state-ledger.md)、[49](./49-er-keyboard-flow-graph.md) 和 [50](./50-er-token-map.md) 已把进入实现前的 workspace / state / keyboard / token contract 冻结到文档。
- 状态: `已修复（剩余为顶层回归覆盖薄弱）`
- 优先级: `已关闭`
- 建议先“观察”还是先“修复”: `转入 focused / top-level regression 观察`
- 当前执行顺序:
  1. 显隐入口首刀已落地：运行期入口统一经过 `set_er_diagram_visible_with_notice(...)`
  2. lifecycle / merge 第二刀已落地：`loading` 等待 FK 与表列请求都结束，FK 徽标不再受回包顺序污染
  3. finalize 语义第三刀已落地：ready 后 layout / 推断关系 / 通知已集中到单一阶段
  4. keyboard owner 第四刀已落地：ER 现在有显式 `FocusArea::ErDiagram`，点击与主循环可进入，hover 不再拥有键盘
  5. local navigation 第五刀已落地：ER 现在有显式 `j/k/Enter/Right/h/Left/Esc/q` 语义，并作为关系浏览器进入主工作区链路
  6. token 第六刀已落地：ER 两波 token 现已全部收口到主主题系统的派生规则
  7. 返回历史第七刀已落地：ER 现在会恢复最近一个合法的非 ER workspace 区域，而不是统一回 `DataGrid`
  8. finalized 决策测试第八刀已落地：handler 层现在显式锁住 `explicit / inferred / empty` 三种 ready-state 分支
  9. FK fallback 第九刀已落地：空 FK 结果与 FK error 现在统一延迟到 finalize 再决定是否推断关系
  10. `l` contract 第十刀已落地：bare `l` 现在打开当前表，`Shift+L` 承担 relayout，ER 键盘流已切到更强的 TUI 主线
  11. 顶层回归锚点第十一刀已落地：`ToggleErDiagram` 命令注册/可用性、ER 聚焦状态栏，以及 Help->Relationships 学习入口现在都有 focused coverage
  12. top-level close/focus-restore 第十二刀已落地：关闭 ER 时只会从 `FocusArea::ErDiagram` 触发 workspace restore，且 `Sidebar / SqlEditor / DataGrid` 回退分支现在都有 focused coverage
  13. top-level open/refresh 第十三刀已落地：`load_er_diagram_data()` 的共享 load planning 现在显式锁住“无 active connection / 空表 / 正常加载”三条分支
  14. surface dispatch 第十四刀已落地：`ERDiagramResponse` 到 app-level `focus / refresh / layout / fit_view` 的分发顺序与分支现在都有 focused coverage
  15. selection-visible 第十五刀已落地：ER 键盘选中项现在会在下一帧自动回到可见视口，线性选表不再只改索引而不改当前浏览位置
  16. explicit-focus 第十六刀已落地：新增 `FocusErDiagram` / `focus_er_diagram`，默认 `Alt+R`；它只在 ER 已打开时可用，只切入 `FocusArea::ErDiagram`，不改变显隐，也不触发 reload
  17. viewport-mode 第十七刀已落地：ER 键盘流现已拆成 `er_diagram` 浏览子作用域与 `er_diagram.viewport` 视口子作用域；`v` 在两者间切换，浏览态继续承载 `j/k/l/h/q`，视口态将 `h/j/k/l` 收回为平移，`Esc` 只退出视口模式回到浏览态
  18. viewport-mode combo anchors 第十八刀已落地：focused tests 现已锁住视口模式内 `q` 关闭、`r / Shift+L / f` 保持可用，以及 `begin_loading()/reload` 会把 `interaction_mode` 重置回 `Navigation`
  19. theme-switch combo anchor 第十九刀已落地：focused test 现已锁住跨主题 render 不会改写 ER 当前的 `interaction_mode / selected_table / pan_offset / zoom`
  20. focus-restore combo anchor 第二十刀已落地：app-level focused tests 现已锁住 `FocusErDiagram -> ToggleErDiagram` 从视口模式关闭时，会恢复最近一个合法的非 ER workspace 区域；若跟踪目标已不可用，则回退到 `DataGrid`，且隐藏后不会残留 ER input scope
  21. relation-adjacency 第二十一刀已落地：浏览态新增 `Shift+J / Shift+K`，会按稳定全局表顺序在当前表的关联集合内前进/后退；它只增强关系浏览，不替换原有 `j/k` 的线性选表
  22. toggle-gate 第二十二刀已落地：`ToggleErDiagram` 现在在 `FocusScope::ErDiagram(Navigation/Viewport)` 下也保持可用，`Ctrl+R` 不再因为 ER 自己持有 focus 而失效；同时 `ShowHistory` 仍保持原有 workspace overlay gate，没有被顺手放开
  23. geometry-adjacency 第二十三刀已落地：浏览态新增 `Shift+Left / Shift+Down / Shift+Up / Shift+Right`，会按当前表卡中心点和方向锥体优先选择同方向最近邻；若没有轴向候选，则才保守回退到同方向对角候选。它只增强 ER 局部浏览，不替换 `j/k` 线性或 `Shift+J / Shift+K` 关系邻接，也不直接同步主业务当前表
  24. detail-mode decision 第二十四刀已冻结：基于现有 `OpenSelectedTable -> QuerySelectedTable -> DataGrid` 证据链，以及 ER 当前缺少独立 detail pane/render 容器的事实，主线现阶段不进入独立 detail mode；ER 继续保持“浏览在 ER、详情回主工作区”的 companion pane 定位
  25. geometry live observation 第一轮已完成：当前 Wayland 会话下沿 `Ctrl+P -> sample -> Enter -> Ctrl+R -> Alt+R -> Shift+Right/Down/Left/Up` 进入学习样例 ER 后，几何邻接与 `selection follows viewport` 未复现明显误跳；当前状态继续保持“观察 / 启发式调优候选”，而不是重新打开 active bug
  26. geometry live observation 第二轮已完成：在同一学习样例链路下继续执行 `Shift+L -> Shift+Down -> f -> Ctrl+Shift+T -> j -> Enter` 后，当前仍未复现 relayout / fit view / theme switch 之后的几何误跳；主题切换后 ER 的缩放级别、当前布局与显隐状态均保持稳定。继续用 `Shift+Arrow` 浏览前，若 chooser 关闭后需恢复 ER 键盘 owner，可显式再按一次 `Alt+R`；目前更接近现有显式聚焦模型，而不是新 bug。
  27. toggle-open focus 第二维收口已落地：基于用户对“ER 已打开但 `h/j/k/l` 仍被 `DataGrid` 捕获”的直接反馈，`ToggleErDiagram` 现在在打开分支也会切入 `FocusArea::ErDiagram`；关闭时仍保持既有的 focus restore / fallback contract。当前 `Alt+R` 保留为 ER 已打开但焦点已离开后的显式回焦动作。

## Notes

- `UX / Input Recovery Batch A` 现已完成阶段性收口：`G41-B006`、`G41-B010`、`G41-B011`、`G41-B012` 以及 `G41-B005` 的顶层 follow-up 已不再作为 active open bug 重复列入 `Priority Summary`。
- 这批条目当前只保留观察项，而不是新的 active implementation 队列：
  - connection-row 右键菜单稳定弹出仍缺真正的 egui/widget live regression；
  - `AboutDialog` 与 `KeyBindingsDialog` / `HelpDialog` 顶部 header compression 仍缺截图级回归锚点；
  - `dialog.toolbar_menu / dialog.toolbar_theme` 是 keybindings UI 的显示 taxonomy，运行时命令 id 继续保持 `toolbar.menu.* / toolbar.theme.*`，这属于分层设计，不应再被误判为未收口冲突。
- 若后续 live smoke 再次复现这批 UX/Input 问题，应新开 follow-up bug 或重新打开对应条目；在没有新复现证据前，这批修复保持 `已关闭 / 观察` 状态。
- 查询执行主线上的已修复问题，现已统一归档到 [02-query-execution-trace.md](./02-query-execution-trace.md)。
- 当前仍未新增显式“取消当前查询”的 UI 入口，但 runtime 已分离“显式取消”和“静默取消”的语义，详见 [02-query-execution-trace.md](./02-query-execution-trace.md)。
- 当前没有把“所有重复状态”都列成独立问题；只收录已经直接影响核心流、或高度可能继续产出回归的根因簇。
- 当前没有把“已在 04/05/07/08/09 中修掉的表层症状”重复列为独立 open bug；这里只保留尚未收敛的底层问题。
