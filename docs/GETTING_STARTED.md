# Gridix Getting Started | 新手上手指南

> Target users: first-time database tool users.  
> 适用对象：第一次使用数据库工具的新手。

## 1. First Launch Checklist | 首次启动检查
1. Open Gridix and stay on the welcome page.
   打开 Gridix，先停留在欢迎页。
2. Check the three database cards status:
   查看三张数据库卡片状态：
   - `SQLite`: built-in, no external install needed.
     `SQLite`：内置支持，无需外部安装。
   - `PostgreSQL` / `MySQL`: local service detection state.
     `PostgreSQL` / `MySQL`：显示本机服务检测状态。
3. Click `重新检测本机数据库环境` if status looks wrong.
   若状态异常，点击 `重新检测本机数据库环境`。

## 2. Fastest Path (Recommended) | 最快上手路径（推荐）
1. Click `一键打开 SQLite 学习示例库`.
   点击 `一键打开 SQLite 学习示例库`。
2. Press `F1` and open learning guide.
   按 `F1` 打开帮助与学习。
3. Start from: `数据库、表、行、列` -> `SELECT 基础`.
   从 `数据库、表、行、列` -> `SELECT 基础` 开始学习。
4. Return to main UI and run your first SQL:
   回到主界面执行第一条 SQL：
   ```sql
   SELECT 1 AS hello_gridix;
   ```

## 3. Create First Connection | 创建第一个连接
1. Press `Ctrl+N` (or click database card) to open connection dialog.
   按 `Ctrl+N`（或点击数据库卡片）打开连接窗口。
2. Choose database type:
   选择数据库类型：
   - New user: `SQLite` first.
     新手优先 `SQLite`。
   - Existing server: `PostgreSQL` or `MySQL/MariaDB`.
     有现成服务时使用 `PostgreSQL` 或 `MySQL/MariaDB`。
3. Fill required fields only, save connection.
   先填写必填项并保存连接。

## 4. First Query Workflow | 第一条查询流程
1. In sidebar, select a table or stay on default query tab.
   在侧边栏选表，或保持默认查询标签页。
2. Press `Ctrl+J` to show SQL editor.
   按 `Ctrl+J` 打开 SQL 编辑器。
3. Type SQL and run with `Ctrl+Enter` (or `F5` when editor focused).
   输入 SQL 后按 `Ctrl+Enter` 执行（编辑器焦点下也可 `F5`）。
4. Use DataGrid for edits, then save with `Ctrl+S`.
   在数据表格中编辑后，按 `Ctrl+S` 保存。

## 5. If PostgreSQL/MySQL Is Not Detected | 未检测到 PostgreSQL/MySQL 时
- Click `安装与初始化` on the corresponding card.
  点击对应卡片上的 `安装与初始化`。
- Follow guide to finish:
  按引导完成：
  1. install service / 安装服务  
  2. start service / 启动服务  
  3. initialize database/user / 初始化库与用户  
  4. return to Gridix and recheck / 回到 Gridix 重检

## 6. Essential Keys | 必记按键
- `Ctrl+N`: new connection / 新建连接
- `Ctrl+J`: toggle SQL editor / 显示或隐藏 SQL 编辑器
- `Ctrl+Enter`: run SQL / 执行 SQL
- `Ctrl+S`: save table edits / 保存表格修改
- `F1`: help and learning guide / 帮助与学习指南
- `Tab` + `hjkl`: focus + navigation / 焦点切换与导航

Full keyboard reference: [KEYBINDINGS.md](KEYBINDINGS.md)  
完整键位说明见：[KEYBINDINGS.md](KEYBINDINGS.md)
