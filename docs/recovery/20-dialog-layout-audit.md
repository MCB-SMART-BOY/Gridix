# Dialog And Overlay Recovery Ledger

## Scope

这份文档现在是 `dialogs / modal / floating window / utility overlay` 主线的长期维护账本。  
原先分散的 shell 约束、owner authority、blocking modal、workspace picker、长表单 shell、toolbar chooser 和 live regression 记录，现都已并入这里。

依赖基线仍以仓库当前实际版本为准：

| 依赖 | 版本 |
|---|---:|
| `egui` | `0.34.1` |
| `eframe` | `0.34.1` |
| `egui_extras` | `0.34.1` |

## Current Shell Contracts

当前 dialog/overlay 已经收成 4 类契约：

### 1. Blocking Modal

适用对象：

- 删除确认
- grid save confirm

约束：

- 明确阻断背景输入
- `Esc / Enter / y / n` 只作用于当前确认流
- 由 `egui::Modal` 驱动，而不是普通 `Window`

### 2. Form Dialog Shell

适用对象：

- `ConnectionDialog`
- `ImportDialog`
- `ExportDialog`
- `CreateDbDialog`
- `CreateUserDialog`
- `DdlDialog`

约束：

- 固定 `header / body / footer`
- 主体只有一个纵向滚动所有者
- 局部代码预览 / 列表继续在 body 内部单独滚动
- footer 不能随正文滚走

### 3. Workspace Dialog Shell

适用对象：

- `KeyBindingsDialog`
- `HelpDialog`
- toolbar `action/create/theme` chooser dialogs

约束：

- 固定 `header / subheader / body / footer`
- body 吃剩余空间
- picker 使用 `Full / Compact / Hidden` 的 layered contract
- pane 自己滚动，不再由外层 window 兜底纵向滚动

### 4. Utility Overlay

适用对象：

- `CommandPalette`
- `HistoryPanel`
- `DataGrid` goto

约束：

- 明确视口约束
- 明确尺寸边界
- 不承担业务 owner 之外的第二套输入语义

## Landed Fix Clusters

### A. 共享 shell 已接管视口边界

已完成：

- `DialogWindow::*` 统一补上 `constrain_to(content_rect)`
- `DialogWindow::workspace()` 去掉外层 `vscroll(true)`

结果：

- shell 重新成为尺寸和视口边界的唯一权责层
- workspace dialog 不再出现“外层 window 和内层 pane 同轴双滚动”

### B. 原先绕过共享壳层的 utility window 已做最小收口

已完成：

- `WelcomeSetup` 迁入共享 shell，并把正文与底部动作区分离
- `CommandPalette`、`HistoryPanel` 增加响应式宽高与视口钳制
- `DataGrid` goto / save confirm 纳入共享 shell；save confirm 进一步升级成 blocking modal

结果：

- 小视口下不再主要靠“内容自然撑开窗口”
- utility window 的越界问题从高频回归降为局部样式一致性问题

### C. dialog owner 已经收口到显式 app-level authority

已完成：

- `active_dialog_owner` 成为当前权威 owner
- `DialogHostSnapshot` 退化为兼容可见性层
- input router 不再按 legacy 布尔位二次推导 dialog scope

结果：

- 同一帧只允许一个 dialog owner 主导输入
- toolbar `action/create` chooser 也已进入显式 owner 主线

### D. blocking confirm contract 已统一

已完成：

- destructive confirm 使用 `egui::Modal`
- grid save confirm 既阻断 app 级输入，也阻断同帧 grid 键盘链

结果：

- 危险确认不再依赖“碰巧没有串路”
- `has_modal_dialog_open()` 和实际确认窗口语义一致

### E. workspace dialogs 已迁到固定 shell + layered picker

已完成：

- `KeyBindingsDialog` 迁到 `WorkspaceDialogShell`
- `HelpDialog` 迁到 `WorkspaceDialogShell + LayeredPickerLayout`
- toolbar `action/create` chooser 改为显式 overlay dialog

结果：

- footer 固定在底部，不再随正文滚走
- 层级前进后，前一级可以 `Compact` 或 `Hidden`
- 不再继续维持“永远三栏一起占宽”
- toolbar `action/create` chooser 的外窗现在改走 `DialogWindow::workspace(...)`，但仍保留 chooser 尺寸级默认宽高，而不是继续锁死在 `fixed_style(...)`
- toolbar `action/create` chooser 的顶部信息区已压缩为单个紧凑 header：左侧标题/副标题，右侧键盘/鼠标提示；`toolbar.menu.dismiss` 现在统一支持 `Esc / Q`
- `KeyBindingsDialog` 里的作用域树现已把 `toolbar.menu.*` / `toolbar.theme.*` 局部命令归到 `dialog.toolbar_menu` / `dialog.toolbar_theme`，不再继续和真正的 `toolbar` 焦点导航命令混在同一个节点里

### F. 长表单 dialog 已迁到固定 footer contract

已完成：

- `CreateUserDialog`
- `CreateDbDialog`
- `DdlDialog`
- `ConnectionDialog`（第一阶段：固定 footer + 核心表单行响应式退化）
- `ImportDialog`（固定 footer + 文件行 / 格式区 / 选项行响应式退化）

结果：

- footer 固定
- 主体滚动所有权明确
- SQL preview / 权限列表 / 列定义局部滚动不再顶走底部动作区
- `DdlDialog` 现已改用 DDL 自己的紧凑 workspace window profile，而不是继续直接吃通用 `DialogStyle::WORKSPACE` 默认高；窄视口首帧不再把 footer 裁到窗口外
- 连接配置中的路径/证书/私钥行不再依赖 `Grid + fixed width + browse` 组合硬撑宽度
- 导入文件路径、格式/模式区与 CSV / JSON 选项行不再继续假设“宽窗口永远成立”

### G. toolbar theme selector 已迁到显式 chooser overlay

已完成：

- toolbar 当前主题 trigger 只负责发起打开
- `Action::OpenThemeSelector` 与 toolbar 鼠标 trigger 现在都汇入显式 dialog owner
- 旧 `Area::fixed_pos(...) + ctx.temp` popup 生命周期已移除

结果：

- 主题选择器不再是 toolbar 内部局部 popup
- open / close / confirm / dismiss 进入 app-level dialog owner 主线
- 视口约束与 footer 布局由 dialog shell 接管

### H. live regression 已完成第一轮

已证实：

- `KeyBindingsDialog` 在小视口下 footer 可见
- `HelpDialog` 在小视口下 layered picker 不再把正文或窗口整体顶爆

### I. `WelcomeSetup` 已补上独立键盘 contract，并成为稳定的 `ExportDialog` seed

已完成：

- `show_welcome_setup_dialog_window()` 现在在 dialog scope 内维护显式动作列表与选中索引
- `Tab / Shift+Tab` 在 guide 内循环动作，不再把焦点泄漏到背景 toolbar
- `Enter` 触发当前选中动作；`1..5` 触发命名动作
- `OpenLearningSample` 打开 guide 后，当前 live 路径已能稳定执行“首条查询”，再进入 `ExportDialog`

结果：

- `WelcomeSetup` 不再只是“可点按钮集合”，而是带作用域命令的 dialog owner
- `OpenLearningSample -> WelcomeSetup -> 首条查询 -> ExportDialog` 已形成可重复的 live 结果集 seed
- 低频 `ExportDialog` live 回归不再受“没有稳定结果集上下文”阻塞

### J. `AboutDialog` 保持 standard shell，但内容层已回摆到更轻的旧版主线

已完成：

- `AboutDialog` 继续保留 `DialogWindow::standard(...)`，没有被抬成 workspace dialog 或可缩放窗口
- 内部结构已从“厚重 hero + manifesto 卡 + facts strip + footer”回摆为“居中品牌头 + 单 manifesto 卡 + 轻量项目速览 + footer”
- 旧版更受欢迎的锚点已恢复：中心化品牌头、更少层级、更轻的社区提示，以及略带轻松感的 about 文案
- 项目信息区仍会根据可用宽度在双列和纵向之间切换，但不再用整块 facts strip 把页面压得笨重

结果：

- `AboutDialog` 重新回到“轻量、好读、有记忆点”的 centered about 页面，而不是厚重的说明卡片堆叠
- 标准 dialog shell 契约保持不变；变化只发生在 about 自身的内容层次、间距、品牌锚点和文案语气
- 旧版本里“更少卡片、更强中心锚点、更轻松的第一印象”已经恢复，但没有把整套历史戏谑文案原样搬回当前主线

仍未 live 全覆盖：

- `CreateUserDialog`

当前 dialog overflow 主线里，高频与低频长表单都已经完成至少一轮桌面复核；`CreateUserDialog` 不再是唯一缺 seed 的阻塞点。

### K. `KeyBindingsDialog` / `HelpDialog` 顶部 header 已压缩为共享紧凑双区块

已完成：

- `KeyBindingsDialog` 与 `HelpDialog` 的顶部不再连续堆两段整宽 `DialogContent::toolbar(...)`
- 两者现在都通过 `PickerDialogShell::header_blocks(...)` 共享同一套 header 组合逻辑
- 宽度足够时，顶部会以内联双区块展示：左侧主控件或快捷键提示，右侧 breadcrumb / 鼠标提示
- 宽度不足时，才回退到纵向堆叠；pane body / layered picker 宽度算法未被修改

结果：

- `快捷键设置` 和 `帮助与学习` 的第一屏高度浪费明显下降
- 顶部视觉语言开始统一到同一套 workspace picker header，而不是“两个整宽块硬堆”
- 这次改动只触及 header 组合层，不改变 body、footer、pane focus 或 dialog owner

### L. `FormDialogShell` 已补上首个错误字段 auto-reveal contract

已完成：

- `FormDialogShell::show(...)` 的 body 现在接收一个最小 `FormDialogBodyContext`
- shell 会登记字段 rect，并在同一滚动壳层内处理 `FirstError` reveal
- `CreateUserDialog` 已先作为代表路径接入：
  - 用户名
  - 密码
  - 确认密码
  - 权限块
- 其余 `FormDialogShell` 调用点当前只同步更新了 body 签名，没有顺手改变表单行为

结果：

- `CreateUserDialog` 在校验失败时，不再只是在 body 底部显示一条通用错误文本
- 首个错误字段现在会被滚回主滚动区的可见位置，避免“错误看到了，但出错字段仍埋在上方/下方”的长表单断裂感
- 这次改动仍保持 `FormDialogShell` 的单主体滚动所有权，不把 reveal 责任重新塞回具体 dialog

当前边界：

- 这条 contract 目前只在代表路径 `CreateUserDialog` 上启用，尚未扩到 `CreateDbDialog` / `DdlDialog`
- 当前还没有 live 复核，因此下一步应先验证代表路径在真实桌面视口下的观感，再决定是否继续推广

## Current Remaining Issues

### 1. 低频长表单仍需要 live 视口回归确认

相关文件：

- [src/ui/dialogs/create_db_dialog.rs](../../src/ui/dialogs/create_db_dialog.rs)
- [src/ui/dialogs/create_user_dialog.rs](../../src/ui/dialogs/create_user_dialog.rs)
- [src/ui/dialogs/export_dialog.rs](../../src/ui/dialogs/export_dialog.rs)

当前问题：

- `ConnectionDialog`、`ImportDialog`、`DdlDialog` 的高风险内容层行布局已经进入响应式退化 contract
- `CreateDbDialog` 已补上固定 footer + 响应式内容行修复，但仍需要在真实桌面视口下复核 footer 是否恢复可见
- `CreateUserDialog`、`ExportDialog` 仍属于低频长表单，但现在都已完成至少一轮真实桌面视口复核；后续主要转入观察而不是继续猜测性补丁
- 当前 open risk 更偏向“live 回归缺口”，而不是已确认的 shell/root cause

结论：

- dialog 横向延伸的主根因已经基本从高频路径收口，剩余工作转为低频长表单的 live 验证与必要时的小补丁。

已完成的最新 live 观察：

- `CreateDbDialog`
  - 在真实 Wayland 桌面会话中，`Ctrl+Shift+D` 可直接拉起“新建 SQLite 数据库”路径。
  - 在约 `960x620` 的 app 视口下，footer 曾在实机截图中不可见，这条回归已从“推测”升级为“已证实”。
  - 当前代码已经补上 `FormDialogShell + resizable shell + 响应式行布局` 的最小修复；自动化验证已通过，但 live 复核尚未补齐。
  - 修复后已在当前 Hyprland/Wayland live 会话中补做一次实机复核：窗口管理器会把浮动窗口高度钳到约 `720px`，在可达到的 `960x720` 视口下 footer 已恢复可见。
  - 原先的精确 `960x620` 点在当前桌面环境中仍未原样复演，因此这条记录现在应理解为“已通过等价 live 复核，但保留窗口管理器高度钳制这一环境注记”。
  - 在更窄视口继续回归时，同一快捷键会根据当前欢迎/初始化状态切到 `SQLite 安装与初始化引导` 路径，而不是稳定进入同一个 create-db form。
  - 这说明后续 live 回归必须先固定 session precondition，不能把“新建数据库”与“欢迎引导”混成同一条观察路径。
- `CreateUserDialog`
  - 本轮先在用户态初始化临时 MariaDB 实例（`127.0.0.1:33306`），并通过临时 `XDG_CONFIG_HOME` 写入一条只供本轮验证使用的 MySQL 连接，绕开了系统级凭据缺失。
  - 沿 `Ctrl+B -> Ctrl+1 -> Enter -> Ctrl+Shift+U` 可以稳定拉起 `CreateUserDialog`，因此 non-SQLite live seed 已不再阻塞。
  - 在约 `900x650` 的浮动窄窗里，footer 保持可见，用户名/密码/确认密码行、MySQL 主机选择行、授权数据库行和 SQL 预览区没有出现新的横向挤压。
  - `q` 能正常关闭该 dialog，说明关闭合同与固定 footer 没有在 non-SQLite live 路径下漂移。
  - 当前 `CreateUserDialog` 状态应从“环境前提阻塞”更新为“已完成第一轮 non-SQLite live，转入观察”。
- `ExportDialog`
  - 当前已确认稳定 seed：`OpenLearningSample -> WelcomeSetup -> Enter`，或在 guide 中直接按 `5`。
  - live 观察显示 `Enter` 与 `5` 都会留在 `WelcomeSetup` 内部消费，随后稳定得到 `SELECT 1 AS hello;` 结果集，并可继续打开 `ExportDialog`。
  - 在 `960x620`、`860x620`、`760x560` 的浮动 app 视口下，footer 仍可见，暂未再现“底部按钮被内容顶走”。
  - `680x520` live 曾暴露一个与 `ExportDialog` 同时出现的 render panic；这条根因现已独立修复为 `render.rs` 中 SQL editor 高度的安全 clamp，不再继续归类为 `ExportDialog` 内容层问题。
  - 修复后新增 focused 测试锁住 tiny viewport 下“SQL editor 可收缩而不 panic”的边界；当前桌面环境会把浮窗重新钳回 `960x720`，因此没有再原样复演精确 `680x520` 点。

#### 下一步不要直接改代码

先按固定观察点做 live 回归，再决定是否继续补丁：

- `CreateDbDialog`
  - `show()` 里的数据库名行
  - `show_mysql_options()` 的字符集 / 排序规则
  - `show_postgres_options()` 的编码 / 模板 / 所有者
  - `show_sqlite_options()` 的文件路径行
- `CreateUserDialog`
  - `show()` 里的用户名 / 密码 / 确认密码行
  - MySQL 主机选择行
  - 授权数据库选择行
  - 权限区域在小视口下是否仍由主体滚动和局部滚动正确分层
- `ExportDialog`
  - `show_row_range()` 的自定义行数输入
  - `show_csv_options()` / `show_tsv_options()` / `show_sql_options()` / `show_json_options()`
  - 格式选择与信息栏在窄视口下是否只换行而不继续撑宽

建议固定观察视口：

- `960x620`
- `860x620`
- `760x560`
- `680x520`

基于当前代码与 live 结果，下一步已经可以进一步收窄：

1. `CreateUserDialog` 已完成第一轮 non-SQLite live；后续只在复现新回归时再回到这里补丁。
2. 如有需要，再在更可控的窗口环境中补 `CreateDbDialog` 的精确 `960x620` 复核点。
3. dialog 主线现在可以继续停留在 observation，或把主线切回别的 confirmed bug/workstream。

### 2. `CommandPalette` / `HistoryPanel` 仍是低优先级 raw utility window

当前状态：

- 视口越界风险已收口
- 但还没有完全统一到共享 utility shell contract

结论：

- 这是剩余一致性问题，不是当前最高优先级 root cause。

### 3. 小视口 dialog live 回归暴露过全局 render clamp 崩溃

相关文件：

- [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs)
- [src/ui/dialogs/export_dialog.rs](../../src/ui/dialogs/export_dialog.rs)

当前问题：

- 在 `680x520` 的浮动 app 视口下，沿着 `OpenLearningSample -> WelcomeSetup -> Enter/5 -> Ctrl+E` 打开 `ExportDialog`，app 会直接 panic。
- panic 信息为：`min > max, or either was NaN. min = 100.0, max = 28.406252`
- 当前 panic 数值更贴近主工作区剩余高度过小时的非法 `clamp`，而不是 `ExportDialog` 内容区继续横向撑爆。

处理结果：

- 这条根因已通过最小补丁收口：SQL editor 高度现在会在 tiny viewport 下收缩，而不是继续用 `100px` 最小值进入非法 `clamp`。
- focused 单测已覆盖“正常视口保留 `100px` 最小高度”和“极小视口不会 panic”两条边界。
- 当前问题已从 open root cause 降为历史回归注记；后续如要补 live，只需要确认不会再在更小视口下崩溃。

## Verification Checklist

继续改这条主线时，至少要检查：

1. 窗口缩小时，footer 是否仍固定可见。
2. 同一轴是否只有一个主滚动所有者。
3. `CreateDb / CreateUser / Export` 的内容行是否仍在窄窗口下继续横向挤压。
4. `Esc / Enter / 确认 / 取消` 是否只命中当前 owner。
5. layered picker 进入下一层后，前一级是否 `Compact / Hidden`，而不是继续常驻占宽。
6. toolbar chooser / utility overlay 是否仍受当前视口约束。

自动化建议：

- `cargo test ui::dialogs::common --lib`
- `cargo test ui::dialogs::picker_shell --lib`
- `cargo test ui::dialogs::help_dialog --lib`
- `cargo test ui::dialogs::keybindings_dialog --lib`
- `cargo test ui::dialogs::toolbar_menu_dialog --lib`
- `cargo test ui_dialogs_tests --test ui_dialogs_tests`
- `cargo test --lib`
- `cargo test`

## Current Priority

dialog workstream 当前最高优先级不再是 owner、blocking modal 或 workspace shell。  
当前第一优先问题已经回到 [12-bug-ledger-4.1.0.md](./12-bug-ledger-4.1.0.md) 中的 `G41-B007`：

- 剩余低频长表单的 live seed 与最终验证；其中 `CreateUserDialog` 当前已确认被环境前提阻塞

具体的下一阶段设计包见：

- [43-dialog-responsive-row-design.md](./43-dialog-responsive-row-design.md)

## Related Docs

- [10-master-recovery-plan.md](./10-master-recovery-plan.md)
- [11-core-flows-and-invariants.md](./11-core-flows-and-invariants.md)
- [12-bug-ledger-4.1.0.md](./12-bug-ledger-4.1.0.md)
