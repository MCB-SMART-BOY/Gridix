# Configuration | 配置说明

## 1. Config File Location | 配置文件位置

Gridix stores user config in `config.toml` under app config directory.  
Gridix 将用户配置保存在应用配置目录下的 `config.toml`。

Typical paths / 常见路径：
- Linux: `~/.config/gridix/config.toml`
- macOS: `~/Library/Application Support/gridix/config.toml`
- Windows: `%APPDATA%\\gridix\\config.toml`

## 2. Main Config Fields | 核心配置字段

| Field | Description |
|---|---|
| `connections` | Saved database connection profiles / 保存的数据库连接配置 |
| `light_theme` | Theme preset used in light mode / 日间模式主题 |
| `dark_theme` | Theme preset used in dark mode (default: Tokyo Night Storm) / 夜间模式主题（默认 Tokyo Night Storm） |
| `is_dark_mode` | Whether dark mode is active / 当前是否启用夜间模式 |
| `ui_scale` | UI zoom ratio (`0.5` to `2.0`) / UI 缩放比例（`0.5` 到 `2.0`） |
| `query_history` | Query history metadata / 查询历史元信息 |
| `command_history` | Per-connection SQL command history / 按连接保存的 SQL 命令历史 |
| `keybindings` | Custom keyboard shortcuts / 自定义快捷键 |
| `onboarding` | Beginner onboarding progress / 新手引导进度 |
| `connection_dialog_show_advanced` | Connection dialog advanced section state / 连接对话框高级配置展开状态 |

## 3. Onboarding Progress Fields | 新手引导字段

`onboarding` includes:
- `environment_checked`
- `connection_created`
- `database_initialized`
- `user_created`
- `first_query_executed`

These fields are used by welcome/help onboarding flow.  
这些字段用于欢迎页与帮助页的新手引导流程。

## 4. Security Notes | 安全说明

- On Unix-like systems, config file is written with `0600` permission.
  在 Unix 类系统上，配置文件写入权限为 `0600`。
- Password/security-sensitive values are handled by app encryption mechanisms.
  密码等敏感信息由应用的加密机制处理。

## 5. Reset Config Safely | 安全重置配置

1. Close Gridix.
   先关闭 Gridix。
2. Backup current config directory.
   备份当前配置目录。
3. Remove or rename `config.toml`.
   删除或重命名 `config.toml`。
4. Relaunch Gridix to regenerate defaults.
   重新启动 Gridix 自动生成默认配置。

## 6. Theme Defaults | 主题默认值

- Default light theme: `TokyoNightLight`
- Default dark theme: `TokyoNightStorm`
- Default mode: dark mode enabled

默认日间主题：`TokyoNightLight`  
默认夜间主题：`TokyoNightStorm`  
默认模式：夜间模式开启
