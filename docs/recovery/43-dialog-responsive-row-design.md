# Dialog Responsive Row Design

## Scope

这份设计文档只服务当前 open 问题 `G41-B007`：

- 多个 dialog 内容层仍依赖“固定宽度横排”
- 窄视口下仍可能出现横向挤压、向右延伸、按钮把输入框顶爆

本文件先定义契约和任务顺序，不实现代码。

## Confirmed Root Cluster

### 固定宽度横排行仍假设“宽窗口永远成立”

高风险位置：

- [src/ui/dialogs/connection_dialog.rs](../../src/ui/dialogs/connection_dialog.rs)
  - `show_connection_form()`
  - `show_mysql_ssl_config()`
  - `show_postgres_ssl_config()`
  - `show_ssh_tunnel_config()`
- [src/ui/dialogs/import_dialog/mod.rs](../../src/ui/dialogs/import_dialog/mod.rs)
  - `show_file_selector()`
  - `show_csv_options()`
  - `show_json_options()`
- [src/ui/dialogs/ddl_dialog.rs](../../src/ui/dialogs/ddl_dialog.rs)
  - `DdlDialog::show()` 中的“表信息”“列定义”区

共同特征：

- 多个 `desired_width(...)` 并列
- `ui.horizontal(...)` 内同时放输入框、按钮、checkbox、combo
- 缺少窄窗口时的折行 / 纵排规则

## Row Archetypes

后续不应按“每个 dialog 单独修”，而应先把剩余行布局分成 4 类。

### A. `FieldWithActionRow`

典型例子：

- SQLite 文件路径 + `浏览`
- CA 证书路径 + `浏览`
- SSH 私钥路径 + `浏览`
- import 文件路径 + `浏览`

目标规则：

- 宽窗口：输入框与按钮同一行
- 中窗口：输入框仍优先占满，按钮缩到最小触达宽度
- 窄窗口：输入框独占一行，按钮单独下一行右对齐或左对齐

### B. `FieldPairRow`

典型例子：

- host + port
- table name + comment
- username + password（若仍并排）

目标规则：

- 宽窗口：双列并排
- 中窗口：主字段宽、次字段窄
- 窄窗口：自动退化为两行

### C. `DenseConfigRow`

典型例子：

- DDL 列定义：列名 / 类型 / 多个布尔位 / 默认值 / 删除按钮

目标规则：

- 不能继续假设“单行容纳所有控件”
- 必须接受：
  - 宽窗口：单行 dense row
  - 中窗口：拆成两段
  - 窄窗口：主字段行 + 次级 flag 行

### D. `ValueHintRow`

典型例子：

- CSV / JSON 选项区
- skip rows / delimiter / table name

目标规则：

- label 与 hint 可以折行
- 输入控件优先保证可编辑宽度
- hint 不得反向把整行撑宽

## Contract Proposal

### Dialog content rows need a shared responsive contract

建议定义一个统一 helper 层，至少覆盖：

- `field_with_action_row(...)`
- `field_pair_row(...)`
- `dense_config_row(...)`

helper 的职责：

- 输入 `available_width`
- 根据阈值决定 `Wide / Medium / Narrow`
- 输出行内布局策略

不要再让各 dialog 直接手写：

- `desired_width(220.0)`
- `desired_width(ui.available_width() - 80.0)`
- 一串 `ui.horizontal(...)` 中的固定宽度推算

### 3. Suggested width classes

建议只做 3 档，不要把布局逻辑写得太细：

- `Wide`: `>= 720px`
- `Medium`: `560px..720px`
- `Narrow`: `< 560px`

这些阈值不是最终视觉规范，只是实现前的验证边界。

## Task Order

### Phase 1. Connection dialog rows（已完成）

已收：

- SQLite 文件路径行
- SSL 证书路径行
- SSH 私钥路径行
- host/port 等典型双字段行

结果：

- 行布局模式最全
- 最能沉淀 reusable row helper
- `ConnectionDialog` 已迁到 `FormDialogShell`
- 核心连接字段、SSL 浏览行、SSH 私钥浏览行已进入响应式退化 contract

当前剩余观察项：

- 数据库类型选择器卡片在极窄宽度下仍可能需要单独退化（推测）

### Phase 2. Import dialog rows（已完成）

已收：

- 文件路径行
- table name / skip rows / json path 这类选项行

结果：

- 它最能验证“路径展示 + 局部提示 + 表单选项”的混合布局
- `ImportDialog` 已迁到 `FormDialogShell`
- 文件路径行不再依赖 `desired_width(ui.available_width() - 80.0)`
- 格式/模式区与 CSV / JSON 选项区已进入响应式退化 contract

当前剩余观察项：

- 预览区本身的内容密度和列裁切仍主要依赖现有 preview/table 逻辑，不属于这一阶段的行布局收口

### Phase 3. DDL dense rows（已完成）

已收：

- 表信息行
- 列定义 dense row

结果：

- `DdlDialog` 的表名/注释区已进入同一套宽/中/窄退化 contract
- 列定义区不再要求所有控件永远挤在一行；宽窗口保持 dense row，中窗口开始分层，窄窗口按“标题 / 主字段 / flags / 默认值”展开
- `DdlDialog` 的首帧窗口 profile 也已从通用 workspace 默认高收紧到 DDL 专用紧凑尺寸，列区 / SQL 预览进一步改成保守的自适应高度；窄视口下 footer 不再被外窗裁出可见区
- footer 与快捷键语义保持不变，列导航仍优先服从文本输入

当前剩余观察项：

- `CreateUserDialog` 仍缺稳定 non-SQLite live seed；当前不是布局 contract 未落地，而是环境前提阻塞

## Verification Matrix

后续实现时，至少按这张矩阵验证：

| Surface | Wide | Medium | Narrow |
|---|---|---|---|
| ConnectionDialog | 路径行同排 | 次按钮不挤爆输入框 | 按钮折到下一行 |
| ImportDialog | 文件行完整 | 路径截断但不溢出 | 浏览按钮不把输入框顶爆 |
| DdlDialog | dense row 单行可用 | 控件开始分层 | 主字段与 flags 分层，仍可编辑 |

统一验收条件：

- 不出现横向裁切后仍继续扩窗
- footer 不受这些行影响
- 文本输入仍优先于命令键
- `Esc / Enter` 语义不因折行而漂移

## Not In Scope

这份设计不处理：

- dialog shell 本身
- blocking modal 语义
- toolbar theme chooser（该切片已落地并并回 [20-dialog-layout-audit.md](./20-dialog-layout-audit.md)）
- Help / KeyBindings layered picker
- ER 图视觉改造

这些已经在其他账本中有单独边界。
