# FAQ | 常见问题

> Shortcut names below use the default keymap unless otherwise stated.  
> 下文提到的快捷键名称默认基于内置默认键位。
> If your `keymap.toml` differs, trust the in-app tooltip and Help panel first.  
> 如果你的 `keymap.toml` 已修改，请优先以应用内悬停提示和帮助面板为准。

## 1. Is Gridix suitable for complete beginners? | 完全零基础能用吗？
Yes. Start from welcome page and press `F1` for guided learning topics.  
可以。先看欢迎页，再按 `F1` 进入学习指南。

Recommended first path:
推荐顺序：
1. SQLite sample
2. `数据库、表、行、列`
3. `SELECT 基础`

## 2. Why does `F5` behave differently? | 为什么 `F5` 有时执行 SQL，有时像刷新？
`F5` is focus-aware:
- In SQL editor: execute SQL.
- Outside SQL editor: refresh connection/table state.

`F5` 是焦点感知按键：
- SQL 编辑器焦点下执行 SQL。
- 编辑器外焦点下用于刷新连接或表状态。

## 3. Why does `Tab` not switch focus in SQL editor? | 为什么 SQL 编辑器里 `Tab` 不切焦点？
In editor input mode, `Tab` is prioritized for autocomplete.  
在编辑器输入模式下，`Tab` 优先用于自动补全。

To switch major focus, use `Tab` when editor is not actively consuming completion.
若要切换主焦点，请在编辑器未消费补全时使用 `Tab`。

## 4. I don't know if PostgreSQL/MySQL is installed. | 我不知道本机有没有 PostgreSQL/MySQL。
Use welcome page status cards and click `重新检测本机数据库环境`.  
看欢迎页状态卡片并点击 `重新检测本机数据库环境`。

If not detected, use `安装与初始化` guide button.  
若未检测到，使用 `安装与初始化` 引导按钮。

## 5. Should I use SQLite or server DB first? | 先学 SQLite 还是先学服务端数据库？
For first-time users, start with SQLite.  
第一次使用建议先从 SQLite 开始。

Then move to PostgreSQL/MySQL after understanding:
- table/row/column
- basic SELECT/WHERE
- safe update workflow

再迁移到 PostgreSQL/MySQL，先掌握表行列、基础查询与安全修改流程。

## 6. Is "expert mode" necessary in connection dialog? | 连接页“专家模式”有必要吗？
The default path is simplified; advanced options are optional.  
默认路径已简化，高级选项按需展开。

Use advanced options only when you need SSH/SSL or special connection tuning.
只有在需要 SSH/SSL 或特殊连接参数时再展开高级项。

## 7. Where can I report bugs? | 去哪里反馈问题？
GitHub Issues: <https://github.com/MCB-SMART-BOY/Gridix/issues>

Please include:
- version
- OS
- database type
- reproducible steps
- screenshot/error text
