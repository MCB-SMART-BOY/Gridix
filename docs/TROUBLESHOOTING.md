# Gridix Troubleshooting | 故障排查

## 1. App Fails to Start | 应用无法启动

### Symptom | 现象
- App exits immediately on Linux.
  Linux 下启动后立即退出。

### Checks | 检查项
- Verify runtime dependencies:
  检查运行依赖：
  - Debian/Ubuntu: `libgtk-3-dev` `libxdo-dev`
  - Fedora/RHEL: `gtk3-devel` `libxdo-devel`
  - Arch: `gtk3` `xdotool`

### Action | 处理
- Install dependencies, then relaunch.
  安装依赖后重新启动。

## 2. PostgreSQL/MySQL Shows "Not Detected" | PostgreSQL/MySQL 显示未检测到

### Symptom | 现象
- Welcome card status is `未检测到本机安装`.
  欢迎页卡片显示 `未检测到本机安装`。

### Checks | 检查项
- Service installed?
  是否已安装服务？
- Service running?
  服务是否在运行？
- Default port reachable (`5432` / `3306`)?
  默认端口是否可达（`5432` / `3306`）？

### Action | 处理
- Click `安装与初始化` or `启动服务引导` on the card.
  点击卡片上的 `安装与初始化` 或 `启动服务引导`。
- Click `重新检测本机数据库环境`.
  点击 `重新检测本机数据库环境`。

## 3. Tab Key Moves Focus Instead of Completion | Tab 变成切焦点而不是补全

### Symptom | 现象
- Pressing `Tab` in SQL editor jumps area focus.
  在 SQL 编辑器按 `Tab` 后焦点跳到别的区域。

### Checks | 检查项
- Is SQL editor focused?
  SQL 编辑器是否为当前焦点？
- Is cursor in editor input state (Insert)?
  是否处于编辑输入状态（Insert）？

### Action | 处理
- Click editor once or press `i` to enter input mode.
  单击编辑器或按 `i` 进入输入模式。
- Use `Ctrl+Space` to force completion popup, then `Tab` confirm.
  先 `Ctrl+Space` 触发补全，再用 `Tab` 确认。

## 4. SQL Executes But No Result Appears | SQL 执行后无结果

### Symptom | 现象
- Execution reports success but table is empty/no visible output.
  执行显示成功，但结果为空或看不到输出。

### Checks | 检查项
- Query type is `UPDATE/INSERT/DELETE` (no rows result expected)?
  是否执行了 `UPDATE/INSERT/DELETE`（本身不返回结果行）？
- Result filtered accidentally?
  是否误开了筛选？

### Action | 处理
- Run a select test query:
  执行测试查询：
  ```sql
  SELECT 1 AS ok;
  ```
- Clear filters (`Ctrl+Shift+F`) and rerun.
  清空筛选（`Ctrl+Shift+F`）后重试。

## 5. Connection Works Then Randomly Fails | 连接一会儿可用一会儿失败

### Checks | 检查项
- Network/SSH tunnel stability.
  网络或 SSH 隧道稳定性。
- Database user permission / max connections.
  数据库用户权限与连接数限制。
- SSL/TLS mode mismatch.
  SSL/TLS 配置模式不匹配。

### Action | 处理
- Reopen connection with minimal settings first.
  先用最小配置重新连接验证。
- Then re-enable SSH/SSL options one by one.
  再逐项恢复 SSH/SSL 配置定位问题。

## 6. UI Looks Too Large/Too Small | 界面过大或过小

### Action | 处理
- Use `Ctrl++` / `Ctrl+-` to scale.
  用 `Ctrl++` / `Ctrl+-` 调整缩放。
- Use `Ctrl+0` to reset scale.
  用 `Ctrl+0` 重置缩放。

## 7. Where To Report Bugs | 如何反馈问题
- GitHub Issues: <https://github.com/MCB-SMART-BOY/Gridix/issues>
- Include:
  - OS + version
  - Gridix version (`vX.Y.Z`)
  - database type (SQLite/PostgreSQL/MySQL)
  - reproducible steps
  - screenshot/error message
