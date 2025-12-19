# Gridix

> 当 Vim 用户遇到数据库，会发生什么？

简洁、快速、安全的跨平台数据库管理工具。**不用鼠标，照样起飞。**

![Version](https://img.shields.io/badge/version-0.5.1-green.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)
[![AUR](https://img.shields.io/aur/version/gridix-bin?label=AUR)](https://aur.archlinux.org/packages/gridix-bin)

```
Gridix = Grid + Helix
       = 表格数据 + 键盘流操作
       = 你的新生产力工具
```

## 为什么选择 Gridix？

| 其他工具 | Gridix |
|---------|--------|
| 点点点点点... | `hjkl` 走天下 |
| 找菜单找半天 | `Ctrl+Enter` 执行，完事 |
| 连接数据库要点 N 个按钮 | `Ctrl+N`，填完回车，搞定 |
| 导出数据要等加载... | 异步执行，界面不卡 |
| 密码明文存配置文件 | AES-256-GCM 加密，安全感拉满 |

## 截图

![Screenshot](gridix.png)

*看到右边那个表格了吗？用 `hjkl` 就能导航，用 `c` 就能改内容。是的，就像 Vim 一样。*

## 特性

**数据库支持：**
- SQLite - 本地轻量首选
- PostgreSQL - 生产环境标配  
- MySQL/MariaDB - 兼容并包

**安全连接：**
- SSH 隧道 - 跳板机？没问题
- MySQL SSL/TLS - 5 种模式任选
- 密码加密存储 - 再也不用担心配置文件泄露

**键盘流操作：**
- Helix/Vim 风格键位 - 三种模式（Normal/Select/Insert）
- `hjkl` 导航 - 肌肉记忆直接迁移
- 数字前缀 - `10j` 向下跳 10 行

**智能编辑器：**
- 语法高亮 - SQL 不再是黑白的
- 自动补全 - 149+ 关键字 + 50+ 函数 + 表名列名
- SQL 格式化 - 一键美化你的意大利面条 SQL

**主题系统：**
- 19 种预设主题
- Tokyo Night / Catppuccin / One Dark / Gruvbox / Dracula...
- 日夜模式独立配置 - 白天亮色，晚上暗色，自动护眼

## 安装

### Arch Linux (AUR)

Arch 用户？一行命令搞定：

```bash
# 预编译版（推荐，秒装）
yay -S gridix-bin

# AppImage 版
yay -S gridix-appimage

# 源码编译版（需要等几分钟）
yay -S gridix
```

用 `paru`？把 `yay` 换成 `paru` 就行。

### 下载预编译版本

从 [Releases](https://github.com/MCB-SMART-BOY/gridix/releases) 下载，解压即用：

| 平台 | 文件 | 说明 |
|------|------|------|
| Linux | `gridix-linux-x86_64.tar.gz` | 通用版本 |
| Linux | `gridix.AppImage` | 开箱即用，双击运行 |
| Windows | `gridix-windows-x86_64.zip` | 解压运行 |
| macOS (ARM) | `gridix-macos-arm64.tar.gz` | M1/M2/M3/M4 |
| macOS (Intel) | `gridix-macos-x86_64.tar.gz` | 老款 Mac |

### 从源码编译

```bash
git clone https://github.com/MCB-SMART-BOY/gridix.git
cd gridix
cargo build --release
# 二进制在 target/release/gridix
```

**Linux 依赖：**
```bash
# Debian/Ubuntu
sudo apt-get install libgtk-3-dev libxdo-dev

# Fedora
sudo dnf install gtk3-devel libxdo-devel

# Arch Linux
sudo pacman -S gtk3 xdotool
```

## 快速上手

1. `Ctrl+N` - 新建连接
2. 选择数据库类型，填写连接信息，回车
3. 在侧边栏选择数据库和表
4. 开始用 `hjkl` 探索你的数据！

## 快捷键速查

### 你会用到的

| 快捷键 | 干嘛的 |
|--------|--------|
| `Ctrl+Enter` | 执行 SQL |
| `Ctrl+N` | 新建连接 |
| `Ctrl+B` | 开关侧边栏 |
| `Ctrl+J` | 开关 SQL 编辑器 |
| `Ctrl+H` | 查询历史 |
| `Ctrl+E` | 导出数据 |
| `Ctrl+D` | 切换日/夜模式 |
| `Ctrl+T` | 换主题 |
| `F5` | 刷新表列表 |

### 表格导航（Normal 模式）

| 快捷键 | 干嘛的 |
|--------|--------|
| `h/j/k/l` | 左/下/上/右 |
| `gg` | 跳到开头 |
| `G` | 跳到结尾 |
| `gh/gl` | 行首/行尾 |
| `Ctrl+u/d` | 翻半页 |
| `5j` | 向下 5 行 |

### 表格编辑

| 快捷键 | 干嘛的 |
|--------|--------|
| `i` | 插入模式 |
| `c` | 修改单元格 |
| `d` | 删除内容 |
| `y` | 复制 |
| `p` | 粘贴 |
| `u` | 撤销 |
| `o/O` | 下方/上方新增行 |
| `Space+d` | 删除整行 |
| `v` | 选择模式 |
| `x` | 选择整行 |

## 高级筛选

16 种操作符，随便组合：

```
Contains / NotContains / Equals / NotEquals
StartsWith / EndsWith / Regex
GreaterThan / LessThan / Between
Empty / NotEmpty
...
```

按 `/` 打开快速筛选，`Ctrl+F` 添加条件，`Ctrl+Shift+F` 清空。

## 数据导入导出

**导出格式：**
- CSV - 通用表格格式
- JSON - API 友好
- SQL - INSERT 语句，直接导入其他数据库

**导入格式：**
- CSV/TSV - 自动检测分隔符
- JSON - 支持数组和嵌套对象
- SQL - 直接执行

## 主题预览

**暗色系（适合 996）：**
Tokyo Night, Catppuccin Mocha, One Dark, Gruvbox Dark, Dracula, Nord...

**亮色系（适合摸鱼）：**
Tokyo Night Light, Catppuccin Latte, One Light, Gruvbox Light, GitHub Light...

`Ctrl+T` 打开主题选择器，挑一个顺眼的。

## 配置文件位置

| 系统 | 路径 |
|------|------|
| Linux | `~/.config/gridix/config.toml` |
| macOS | `~/Library/Application Support/gridix/config.toml` |
| Windows | `%APPDATA%\gridix\config.toml` |

## 技术栈

给好奇的朋友：

| 干什么的 | 用什么 |
|---------|--------|
| GUI | egui/eframe 0.29 |
| 异步 | Tokio |
| SQLite | rusqlite |
| PostgreSQL | tokio-postgres |
| MySQL | mysql_async |
| SSH | russh |
| 加密 | ring (AES-256-GCM) |
| 语法高亮 | syntect |

整个项目约 10000 行 Rust 代码，编译后单文件 ~22MB。

## 更新日志

### v0.5.1 (2025-12-20)
- 新增 AUR 包支持（gridix-bin / gridix / gridix-appimage）
- 新增 AppImage 打包
- 文档更新

### v0.5.0 (2025-12-20)
- 新增完整的 Helix 键盘操作文档
- 新增历史面板 Helix 键盘导航
- 数据表格列宽智能自适应
- 统一所有界面组件的操作逻辑

### v0.4.0 (2025-12-18)
- 新增对话框统一键盘导航
- GitHub Actions 自动构建跨平台 Release

### v0.3.0 (2025-12-15)
- 新增侧边栏键盘导航
- 新增数据导入对话框
- UI 优化

### v0.2.0 (2025-12-10)
- 新增 MySQL SSL/TLS 支持
- 新增 SSH 隧道连接
- 新增多标签查询

### v0.1.0 (2024-12-09)
- 初始版本发布

## 贡献

发现 Bug？有好点子？

- [提 Issue](https://github.com/MCB-SMART-BOY/gridix/issues)
- [发 PR](https://github.com/MCB-SMART-BOY/gridix/pulls)
- 或者直接 Fork 改成你喜欢的样子

## 许可证

MIT - 随便用，记得给个 Star ⭐

---

*献给所有被鼠标折磨过的键盘侠们。*
