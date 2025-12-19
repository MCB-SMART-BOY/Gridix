# Gridix

简洁、快速、安全的跨平台数据库管理工具，专为键盘党打造。

![Version](https://img.shields.io/badge/version-0.5.0-green.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)

> **Gridix** = Grid + Helix，表格数据库管理 + Helix 风格键位操作

## 特性亮点

- **多数据库支持** - SQLite、PostgreSQL、MySQL/MariaDB
- **SSH 隧道支持** - 通过 SSH 安全连接远程数据库，支持密码和密钥认证
- **MySQL SSL/TLS** - 5 种 SSL 模式（Disabled/Preferred/Required/VerifyCa/VerifyIdentity）
- **Helix/Vim 风格键位** - 三种编辑模式（Normal/Select/Insert），hjkl 导航
- **智能 SQL 编辑器** - 语法高亮、上下文感知自动补全、SQL 格式化
- **多标签查询** - 支持多个查询标签页，独立执行和管理
- **高级筛选系统** - 16 种操作符、多条件组合（AND/OR）、正则支持
- **19 种主题预设** - Tokyo Night、Catppuccin、One Dark、Gruvbox 等
- **数据导入导出** - CSV、JSON、SQL（支持列选择、行范围、批量导入）
- **DDL 管理** - 查看和编辑表结构定义
- **安全存储** - AES-256-GCM 加密保存数据库密码
- **轻量高效** - 单二进制文件（~22MB），GPU 加速渲染，内存占用低

## 截图

![Screenshot](gridix.png)

## 安装

### 下载预编译版本

从 [Releases](https://github.com/MCB-SMART-BOY/gridix/releases) 页面下载最新版本：

| 平台 | 文件 | 大小 |
|------|------|------|
| Linux x86_64 | `gridix-linux-x86_64.tar.gz` | 29 MB |
| Linux AppImage | `gridix.AppImage` | 16 MB |
| Windows x86_64 | `gridix-windows-x86_64.zip` | 13 MB |
| macOS Apple Silicon | `gridix-macos-arm64.tar.gz` | 13 MB |
| macOS Intel | `gridix-macos-x86_64.tar.gz` | 13 MB |

### 从源码编译

```bash
git clone https://github.com/MCB-SMART-BOY/gridix.git
cd gridix
cargo build --release
```

编译后的二进制文件位于 `target/release/gridix`。

**Linux 依赖：**
```bash
# Debian/Ubuntu
sudo apt-get install libgtk-3-dev libxdo-dev

# Fedora
sudo dnf install gtk3-devel libxdo-devel

# Arch Linux
sudo pacman -S gtk3 xdotool
```

## 连接配置

### 基本连接

支持三种数据库类型：

| 数据库 | 默认端口 | 说明 |
|--------|----------|------|
| SQLite | - | 本地文件数据库 |
| PostgreSQL | 5432 | 支持远程连接 |
| MySQL/MariaDB | 3306 | 支持 SSL/TLS 加密 |

### SSH 隧道

通过 SSH 隧道安全连接远程数据库：

1. 启用 SSH 隧道选项
2. 配置 SSH 服务器地址和端口（默认 22）
3. 输入 SSH 用户名
4. 选择认证方式：密码 或 私钥文件

### MySQL SSL/TLS

支持 5 种 SSL 模式：

| 模式 | 说明 |
|------|------|
| Disabled | 禁用 SSL（默认） |
| Preferred | 优先使用 SSL，不强制 |
| Required | 强制使用 SSL |
| VerifyCa | 验证服务器 CA 证书 |
| VerifyIdentity | 验证证书和主机名 |

## 快捷键

### 全局快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+N` | 新建连接 |
| `Ctrl+Enter` | 执行查询 |
| `Ctrl+J` | 切换 SQL 编辑器 |
| `Ctrl+B` | 切换侧边栏 |
| `Ctrl+H` | 查询历史 |
| `Ctrl+E` | 导出数据 |
| `Ctrl+I` | 导入文件 |
| `Ctrl+D` | 切换日/夜模式 |
| `Ctrl+T` | 主题选择器 |
| `Ctrl+1/2/3` | 快速切换 连接/数据库/表 |
| `Ctrl++/-/0` | 缩放界面 |
| `Ctrl+F` | 添加筛选条件 |
| `Ctrl+Shift+F` | 清空筛选 |
| `Ctrl+G` | 跳转到指定行 |
| `Ctrl+S` | 保存修改 |
| `F5` | 刷新表列表 |
| `F1` | 帮助文档 |

### 表格编辑（Helix/Vim 风格）

**Normal 模式 - 导航：**

| 快捷键 | 功能 |
|--------|------|
| `h/j/k/l` | 左/下/上/右移动 |
| `w/b` | 向右/左跳列 |
| `gg` | 跳到首行 |
| `G` | 跳到末行 |
| `gh/gl` | 跳到行首/行尾 |
| `Ctrl+u/d` | 向上/下翻半页 |
| `5j` | 向下移动 5 行（数字前缀） |

**Normal 模式 - 编辑：**

| 快捷键 | 功能 |
|--------|------|
| `i` | 在当前位置插入 |
| `a` | 在当前位置后插入 |
| `c` | 修改单元格内容 |
| `r` | 替换单元格 |
| `d` | 删除内容 |
| `y` | 复制 |
| `p` | 粘贴 |
| `u` | 撤销修改 |
| `o` | 在下方新增行 |
| `O` | 在上方新增行 |
| `Space+d` | 标记删除整行 |

**Select 模式：**

| 快捷键 | 功能 |
|--------|------|
| `v` | 进入选择模式 |
| `x` | 选择整行 |
| `Esc` | 退出选择模式 |

**Insert 模式：**

| 快捷键 | 功能 |
|--------|------|
| `Esc` | 退出插入模式 |
| `Enter` | 确认输入 |

## 数据导出

支持三种导出格式，提供丰富的配置选项：

### CSV 格式
- 自定义分隔符（逗号、制表符、分号等）
- 可选包含表头
- 自定义引号字符

### SQL 格式
- 生成 INSERT 语句
- 可选事务包装
- 自定义批量大小

### JSON 格式
- 标准 JSON 数组
- 可选格式化输出（Pretty Print）

**通用选项：**
- 选择导出列
- 设置行范围（起始行、行数限制）
- 导出前预览

## 数据导入

支持导入 CSV、JSON、SQL 文件：

- **CSV** - 自动检测分隔符，支持 TSV
- **JSON** - 支持数组和嵌套对象
- **SQL** - 直接执行 SQL 文件

导入后自动生成 INSERT 语句，可在 SQL 编辑器中预览和编辑。

## 主题系统

支持 19 种预设主题，日间/夜间模式独立配置：

**暗色主题（11种）：**
- Tokyo Night / Tokyo Night Storm
- Catppuccin Mocha / Macchiato / Frappé
- One Dark / One Dark Vivid
- Gruvbox Dark
- Dracula
- Nord
- Solarized Dark
- Monokai Pro
- GitHub Dark

**亮色主题（8种）：**
- Tokyo Night Light
- Catppuccin Latte
- One Light
- Gruvbox Light
- Solarized Light
- GitHub Light

使用 `Ctrl+D` 快速切换日/夜模式，`Ctrl+T` 打开主题选择器。

## SQL 自动补全

智能上下文感知的 SQL 自动补全：

- **149+ SQL 关键字** - SELECT、INSERT、UPDATE、DELETE、JOIN、WHERE 等
- **50+ SQL 函数** - COUNT、SUM、AVG、CONCAT、DATE、COALESCE 等
- **表名补全** - 从当前数据库动态加载
- **列名补全** - 在 FROM/JOIN/WHERE 后自动提示相关列

## 高级筛选

支持 16 种筛选操作符：

| 类型 | 操作符 |
|------|--------|
| 文本匹配 | Contains、NotContains、Equals、NotEquals |
| 文本模式 | StartsWith、EndsWith、Regex（正则表达式） |
| 数值比较 | GreaterThan、GreaterOrEqual、LessThan、LessOrEqual |
| 范围判断 | Between |
| 空值检查 | Empty、NotEmpty |
| 逻辑组合 | AND、OR |

支持多条件组合筛选，使用 `/` 快捷键打开快速筛选对话框。

## 项目结构

```
src/
├── main.rs              # 程序入口
├── app.rs               # 主应用逻辑
├── lib.rs               # 库导出
├── core/                # 核心功能模块
│   ├── autocomplete.rs  # SQL 自动补全引擎
│   ├── config.rs        # 配置管理
│   ├── constants.rs     # 常量定义
│   ├── export.rs        # 数据导出/导入
│   ├── formatter.rs     # SQL 格式化
│   ├── history.rs       # 查询历史管理
│   ├── syntax.rs        # 语法高亮
│   └── theme.rs         # 主题管理
├── database/            # 数据库层
│   ├── mod.rs           # 连接池管理、SSL 配置
│   ├── query.rs         # 查询执行引擎
│   └── ssh_tunnel.rs    # SSH 隧道实现
└── ui/                  # 用户界面
    ├── components/      # UI 组件
    │   ├── grid/        # 数据表格（含筛选器）
    │   ├── sql_editor.rs
    │   ├── toolbar.rs
    │   ├── search_bar.rs
    │   └── query_tabs.rs
    ├── dialogs/         # 对话框
    │   ├── connection_dialog.rs
    │   ├── export_dialog.rs
    │   ├── import_dialog.rs
    │   ├── ddl_dialog.rs
    │   ├── help_dialog.rs
    │   └── about_dialog.rs
    └── panels/          # 面板
        ├── sidebar.rs
        └── history_panel.rs
```

## 技术栈

| 组件 | 技术 | 版本 |
|------|------|------|
| GUI 框架 | egui / eframe | 0.29 |
| 异步运行时 | Tokio | 1.x |
| SQLite 驱动 | rusqlite | 0.31 |
| PostgreSQL 驱动 | tokio-postgres | 0.7 |
| MySQL 驱动 | mysql_async | 0.34 |
| SSH 客户端 | russh | 0.45 |
| 密码加密 | ring (AES-256-GCM) | 0.17 |
| 语法高亮 | syntect | 5.2 |
| 序列化 | serde + toml | 1.x |

## 配置文件

配置文件自动保存到系统配置目录：

| 系统 | 路径 |
|------|------|
| Linux | `~/.config/gridix/config.toml` |
| macOS | `~/Library/Application Support/gridix/config.toml` |
| Windows | `%APPDATA%\gridix\config.toml` |

配置内容包括：
- 数据库连接信息（密码加密存储）
- 主题设置（日间/夜间模式）
- 查询历史记录
- UI 缩放比例
- 窗口位置和大小

## 开发指南

```bash
# 克隆项目
git clone https://github.com/MCB-SMART-BOY/gridix.git
cd gridix

# 开发模式运行
cargo run

# Release 构建
cargo build --release

# 运行测试
cargo test

# 代码检查
cargo clippy

# 构建 AppImage (Linux)
cargo install cargo-appimage
cargo appimage
```

## 更新日志

### v0.5.0 (2024-12-20)
- 新增完整的 Helix 键盘操作文档 (`docs/KEYBINDINGS.md`)
- 新增历史面板 Helix 键盘导航（j/k/g/G/Enter/Esc）
- 对话框焦点管理优化（打开对话框时禁用其他区域键盘响应）
- 数据表格列宽智能自适应（根据内容自动计算，区分中英文字符宽度）
- 列宽限制：最小 60px，最大 400px
- 统一所有界面组件的操作逻辑

### v0.4.0 (2024-12-18)
- 新增对话框统一键盘导航模块
- 所有对话框支持 Esc/q 关闭、Enter 确认
- 确认对话框支持 y/n 快捷键
- 导出/导入对话框支持数字键切换格式
- 新建表对话框支持 o/O 添加列、dd 删除列
- GitHub Actions 自动构建跨平台 Release

### v0.3.0 (2024-12-15)
- 新增侧边栏键盘导航（Ctrl+1/2/3 快速切换连接/数据库/表）
- 新增侧边栏 j/k/g/G/Enter 键位导航支持
- 新增数据导入对话框（CSV、JSON、SQL 文件导入）
- 新增关于对话框（版本信息、GitHub 链接）
- 优化 SQL 编辑器面板布局（固定高度，防止自动增长）
- 优化编辑器/历史区域比例（70%/30%）
- UI 细节改进和 Bug 修复

### v0.2.0 (2024-12-10)
- 新增 MySQL SSL/TLS 支持（5 种模式）
- 新增 SSH 隧道连接功能
- 增强导出对话框（列选择、行范围、格式选项）
- 新增多标签查询支持
- 新增 DDL 查看对话框
- 代码优化和 Clippy 修复
- 更新依赖版本

### v0.1.0 (2024-12-09)
- 初始版本发布
- 支持 SQLite、PostgreSQL、MySQL
- Helix/Vim 风格键位
- 19 种主题预设
- 智能 SQL 自动补全
- 高级筛选系统

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！

- 报告 Bug：[Issues](https://github.com/MCB-SMART-BOY/gridix/issues)
- 功能建议：[Discussions](https://github.com/MCB-SMART-BOY/gridix/discussions)
