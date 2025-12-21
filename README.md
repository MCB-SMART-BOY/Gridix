# Gridix

[English](#english) | [中文](#中文)

---

<a id="english"></a>

<div align="center">

# Gridix

**A keyboard-first database management tool for developers who live in the terminal**

[![Version](https://img.shields.io/badge/version-1.1.0-blue.svg)](https://github.com/MCB-SMART-BOY/Gridix/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)]()

[![AUR](https://img.shields.io/aur/version/gridix-bin?label=AUR&logo=archlinux)](https://aur.archlinux.org/packages/gridix-bin)
[![Flathub](https://img.shields.io/flathub/v/io.github.mcb_smart_boy.Gridix?logo=flathub)](https://flathub.org/apps/io.github.mcb_smart_boy.Gridix)
[![Homebrew](https://img.shields.io/badge/homebrew-tap-brown?logo=homebrew)](https://github.com/MCB-SMART-BOY/homebrew-gridix)
[![Nixpkgs](https://img.shields.io/badge/nixpkgs-unstable-blue?logo=nixos)](https://github.com/NixOS/nixpkgs)

</div>

---

**Gridix** = **Grid** + **Helix**

Navigate databases with `hjkl`. Edit tables the Vim way. No mouse required.

![Screenshot](gridix.png)

## Features

| Feature | Description |
|---------|-------------|
| **Keyboard-First** | Full Helix/Vim keybindings - `hjkl` navigation, `gg/G` jumps, `ciw` editing |
| **Multi-Database** | SQLite, PostgreSQL, MySQL/MariaDB with unified interface |
| **Secure** | AES-256-GCM encrypted passwords, SSH tunneling, SSL/TLS support |
| **Beautiful** | 19 built-in themes including Catppuccin, Tokyo Night, Dracula, Nord |
| **Fast** | Pure Rust, <1s startup, ~22MB binary, ~50MB memory |
| **ER Diagrams** | Visual table relationships with foreign key detection |
| **Smart SQL** | Syntax highlighting, auto-completion (149 keywords + 50 functions), formatting |
| **Import/Export** | CSV, JSON, SQL with preview and column selection |
| **Advanced Filtering** | 16 operators including regex, between, null checks |

## Installation

### Package Managers

```bash
# Arch Linux (AUR)
paru -S gridix-bin          # Pre-compiled binary
paru -S gridix-appimage     # AppImage bundle
paru -S gridix              # Build from source

# NixOS / Nix
nix run github:MCB-SMART-BOY/Gridix
# or add to configuration.nix after PR merge
environment.systemPackages = [ pkgs.gridix ];

# Flatpak
flatpak install flathub io.github.mcb_smart_boy.Gridix

# Homebrew (macOS/Linux)
brew tap MCB-SMART-BOY/gridix
brew install gridix
```

### Pre-built Binaries

Download from [Releases](https://github.com/MCB-SMART-BOY/Gridix/releases):

| Platform | Architecture | File | Size |
|----------|-------------|------|------|
| Linux | x86_64 | `gridix-linux-x86_64.tar.gz` | ~13 MB |
| Linux | x86_64 | `gridix.AppImage` | ~17 MB |
| Windows | x86_64 | `gridix-windows-x86_64.zip` | ~12 MB |
| macOS | ARM64 (M1/M2/M3/M4) | `gridix-macos-arm64.tar.gz` | ~12 MB |

### Build from Source

```bash
git clone https://github.com/MCB-SMART-BOY/Gridix.git
cd Gridix
cargo build --release
# Binary: target/release/gridix
```

<details>
<summary><b>Linux Dependencies</b></summary>

```bash
# Debian/Ubuntu
sudo apt install libgtk-3-dev libxdo-dev

# Fedora/RHEL
sudo dnf install gtk3-devel libxdo-devel

# Arch Linux
sudo pacman -S gtk3 xdotool

# openSUSE
sudo zypper install gtk3-devel libxdo-devel
```
</details>

## Quick Start

```
1. Launch gridix
2. Ctrl+N → New connection
3. Select database type, fill details, Enter
4. Click a table in sidebar
5. Navigate with hjkl, edit with i/c/d
```

## Keyboard Reference

### Global Shortcuts

| Key | Action | Key | Action |
|-----|--------|-----|--------|
| `Ctrl+N` | New connection | `Ctrl+Shift+N` | New table |
| `Ctrl+Enter` / `F5` | Execute SQL | `Ctrl+S` | Save changes |
| `Ctrl+B` | Toggle sidebar | `Ctrl+J` | Toggle SQL editor |
| `Ctrl+H` | Query history | `Ctrl+R` | Toggle ER diagram |
| `Ctrl+E` | Export data | `Ctrl+I` | Import data |
| `Ctrl+T` | Theme picker | `Ctrl+D` | Toggle dark/light |
| `Ctrl+F` | Add filter | `Ctrl+Shift+F` | Clear filters |
| `Ctrl+G` | Go to row | `Ctrl+L` | Clear SQL editor |
| `Ctrl+1/2/3` | Focus sidebar sections | `Ctrl+Tab` | Next tab |
| `Ctrl++/-/0` | Zoom in/out/reset | `F1` | Help |

### Grid Navigation (Normal Mode)

```
Movement:           Quick Jumps:           Scrolling:
    k               gg → First row         Ctrl+u → Half page up
  h   l             G  → Last row          Ctrl+d → Half page down
    j               gh → First column      PageUp/PageDown
                    gl → Last column

Numeric Prefix:     5j → Down 5 rows | 10k → Up 10 rows | 3w → Right 3 columns
```

### Grid Editing

| Key | Action | Key | Action |
|-----|--------|-----|--------|
| `i` | Insert mode | `a` | Append mode |
| `c` | Change cell | `r` | Replace mode |
| `d` | Delete content | `dd` / `Space d` | Mark row for deletion |
| `y` / `yy` | Copy cell/row | `p` | Paste |
| `o` / `O` | Insert row below/above | `u` / `U` | Undo cell/row |
| `v` | Visual selection mode | `w` / `Ctrl+S` | Write (save) changes |

### Selection Mode (`v`)

| Key | Action |
|-----|--------|
| `h/j/k/l` | Extend selection |
| `x` | Select entire row |
| `d` | Delete selected |
| `y` | Copy selected |
| `Esc` | Exit selection |

## Database Support

| Database | Port | Features |
|----------|------|----------|
| **SQLite** | - | Local file, zero config, bundled driver |
| **PostgreSQL** | 5432 | Async driver, connection pool, full feature support |
| **MySQL/MariaDB** | 3306 | Async driver, connection pool, 5 SSL modes |

### MySQL SSL/TLS Modes

| Mode | Description |
|------|-------------|
| `Disabled` | No encryption (default) |
| `Preferred` | Encrypt if available |
| `Required` | Must encrypt |
| `VerifyCa` | Verify server certificate |
| `VerifyIdentity` | Verify certificate + hostname |

### SSH Tunneling

Connect through jump hosts with SSH tunneling:

```
Your Machine ──SSH──> Jump Host ──────> Database Server
     └── Local Port ←── Tunnel ←── Remote Port
```

**Supported Authentication:**
- Password authentication
- Private key (OpenSSH format, with optional passphrase)

## Data Import/Export

### Export Formats

| Format | Options |
|--------|---------|
| **CSV** | Custom delimiter, quote char, header row |
| **JSON** | Pretty print or compact |
| **SQL** | INSERT statements, transaction wrapping, batch size |

### Import Formats

| Format | Features |
|--------|----------|
| **CSV/TSV** | Auto-detect delimiter, skip rows, max rows limit |
| **JSON** | Array or nested objects, JSON path support |
| **SQL** | Direct execution with transaction |

## Advanced Filtering

Press `/` for quick filter or `Ctrl+F` to add conditions.

**16 Operators:**

| Category | Operators |
|----------|-----------|
| Text | `Contains`, `NotContains`, `Equals`, `NotEquals`, `StartsWith`, `EndsWith`, `Regex` |
| Numeric | `>`, `>=`, `<`, `<=`, `Between`, `NotBetween` |
| Set | `In`, `NotIn` |
| Null | `IsNull`, `IsNotNull`, `IsEmpty`, `IsNotEmpty` |

Combine with `AND` / `OR` logic.

## Themes

**Dark (11):** Tokyo Night, Tokyo Night Storm, Catppuccin Mocha/Macchiato/Frappé, One Dark, One Dark Vivid, Gruvbox Dark, Dracula, Nord, GitHub Dark

**Light (8):** Tokyo Night Light, Catppuccin Latte, One Light, Gruvbox Light, Solarized Light/Dark, GitHub Light, Monokai Pro

- `Ctrl+T` - Open theme picker
- `Ctrl+D` - Toggle dark/light mode
- Independent day/night theme configuration

## SQL Editor

- **Syntax Highlighting** - Keywords, strings, comments, numbers
- **Auto-Completion** - 149 SQL keywords + 50 functions + table/column names
- **Tab Completion** - Press Tab to accept suggestion
- **History Navigation** - `Shift+↑/↓` to browse history
- **SQL Formatting** - One-click beautification

## ER Diagram

Press `Ctrl+R` to view table relationships:
- Column details (type, NULL/NOT NULL, defaults)
- Foreign key relationships (solid lines)
- Inferred relationships from naming conventions (dashed lines)
- Drag to rearrange layout

## Configuration

| OS | Path |
|----|------|
| Linux | `~/.config/gridix/config.toml` |
| macOS | `~/Library/Application Support/gridix/config.toml` |
| Windows | `%APPDATA%\gridix\config.toml` |

**Stored Settings:**
- Database connections (passwords AES-256-GCM encrypted)
- Theme preferences (day/night)
- UI scale (0.5x - 2.0x)
- Query history (up to 100 entries)
- Custom keybindings

## Security

| Feature | Implementation |
|---------|----------------|
| Password Storage | AES-256-GCM encryption with machine-specific key derivation |
| SSH Tunneling | Pure Rust implementation (russh), password and private key auth |
| MySQL SSL/TLS | 5 security levels from disabled to full verification |
| Config Files | Unix permissions 0600 (owner read/write only) |

## Technical Details

| Component | Technology |
|-----------|------------|
| GUI Framework | egui/eframe 0.33 |
| Async Runtime | Tokio (multi-threaded) |
| SQLite Driver | rusqlite 0.38 (bundled) |
| PostgreSQL Driver | tokio-postgres 0.7 |
| MySQL Driver | mysql_async 0.36 |
| SSH | russh 0.55 |
| Encryption | ring 0.17 |
| Syntax Highlighting | syntect 5.3 |

**Stats:**
- ~9,000 lines of Rust
- ~22 MB binary (release, stripped, LTO)
- <1 second startup
- ~50 MB memory usage

## Project Structure

```
src/
├── main.rs                 # Entry point
├── app/                    # Application logic (9 modules)
│   ├── mod.rs              # DbManagerApp
│   ├── state.rs            # State management
│   ├── keyboard.rs         # Global shortcuts
│   ├── database.rs         # DB operations
│   └── ...
├── core/                   # Core features (12 modules)
│   ├── autocomplete.rs     # SQL completion
│   ├── config.rs           # Configuration + encryption
│   ├── keybindings.rs      # Customizable shortcuts
│   ├── theme.rs            # 19 themes
│   └── ...
├── database/               # Database layer (8 modules)
│   ├── connection.rs       # Connection management
│   ├── pool.rs             # Connection pooling
│   ├── ssh_tunnel.rs       # SSH tunneling
│   └── query/              # Drivers (SQLite, PostgreSQL, MySQL)
└── ui/                     # User interface
    ├── components/         # Grid, SQL editor, toolbar, etc.
    │   ├── grid/           # Helix-style data grid (7 modules)
    │   ├── er_diagram/     # ER visualization
    │   └── ...
    └── dialogs/            # 14 dialog types
```

## Development

```bash
cargo run              # Development build
cargo test             # Run tests (13 test modules)
cargo clippy           # Lint
cargo build --release  # Release build
cargo appimage         # Build AppImage (Linux)
```

## Contributing

- **Bug Reports:** [Issues](https://github.com/MCB-SMART-BOY/Gridix/issues)
- **Feature Requests:** [Discussions](https://github.com/MCB-SMART-BOY/Gridix/discussions)
- **Pull Requests:** Welcome!

## Acknowledgments

- [egui](https://github.com/emilk/egui) - Immediate mode GUI
- [Helix Editor](https://helix-editor.com/) - Keybinding inspiration
- [Catppuccin](https://github.com/catppuccin) - Color schemes
- [syntect](https://github.com/trishume/syntect) - Syntax highlighting

## License

[MIT License](LICENSE) - Free for personal and commercial use.

---

<a id="中文"></a>

<div align="center">

# Gridix

**给住在终端里的开发者做的键盘优先数据库工具**

[![Version](https://img.shields.io/badge/version-1.1.0-blue.svg)](https://github.com/MCB-SMART-BOY/Gridix/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)]()

[![AUR](https://img.shields.io/aur/version/gridix-bin?label=AUR&logo=archlinux)](https://aur.archlinux.org/packages/gridix-bin)
[![Flathub](https://img.shields.io/flathub/v/io.github.mcb_smart_boy.Gridix?logo=flathub)](https://flathub.org/apps/io.github.mcb_smart_boy.Gridix)
[![Homebrew](https://img.shields.io/badge/homebrew-tap-brown?logo=homebrew)](https://github.com/MCB-SMART-BOY/homebrew-gridix)
[![Nixpkgs](https://img.shields.io/badge/nixpkgs-unstable-blue?logo=nixos)](https://github.com/NixOS/nixpkgs)

</div>

---

**Gridix** = **Grid** + **Helix**

用 `hjkl` 导航数据库，用 Vim 的方式编辑表格，不需要鼠标。

![Screenshot](gridix.png)

## 特性一览

| 特性 | 说明 |
|------|------|
| **键盘优先** | 完整 Helix/Vim 键位 - `hjkl` 导航、`gg/G` 跳转、`ciw` 编辑 |
| **多数据库** | SQLite、PostgreSQL、MySQL/MariaDB 统一接口 |
| **够安全** | AES-256-GCM 密码加密、SSH 隧道、SSL/TLS 支持 |
| **够好看** | 19 套内置主题：Catppuccin、Tokyo Night、Dracula、Nord |
| **够快** | 纯 Rust，启动 <1 秒，二进制 ~22MB，内存 ~50MB |
| **ER 图** | 可视化表关系，自动检测外键 |
| **智能 SQL** | 语法高亮、自动补全（149 关键字 + 50 函数）、格式化 |
| **导入导出** | CSV、JSON、SQL，支持预览和列选择 |
| **高级筛选** | 16 种操作符，包括正则、范围、空值检查 |

## 安装

### 包管理器

```bash
# Arch Linux (AUR)
paru -S gridix-bin          # 预编译二进制
paru -S gridix-appimage     # AppImage 包
paru -S gridix              # 源码编译

# NixOS / Nix
nix run github:MCB-SMART-BOY/Gridix
# 或在 PR 合并后添加到 configuration.nix
environment.systemPackages = [ pkgs.gridix ];

# Flatpak
flatpak install flathub io.github.mcb_smart_boy.Gridix

# Homebrew (macOS/Linux)
brew tap MCB-SMART-BOY/gridix
brew install gridix
```

### 预编译下载

从 [Releases](https://github.com/MCB-SMART-BOY/Gridix/releases) 下载：

| 平台 | 架构 | 文件 | 大小 |
|------|-----|------|------|
| Linux | x86_64 | `gridix-linux-x86_64.tar.gz` | ~13 MB |
| Linux | x86_64 | `gridix.AppImage` | ~17 MB |
| Windows | x86_64 | `gridix-windows-x86_64.zip` | ~12 MB |
| macOS | ARM64 (M1/M2/M3/M4) | `gridix-macos-arm64.tar.gz` | ~12 MB |

### 源码编译

```bash
git clone https://github.com/MCB-SMART-BOY/Gridix.git
cd Gridix
cargo build --release
# 二进制: target/release/gridix
```

<details>
<summary><b>Linux 依赖</b></summary>

```bash
# Debian/Ubuntu
sudo apt install libgtk-3-dev libxdo-dev

# Fedora/RHEL
sudo dnf install gtk3-devel libxdo-devel

# Arch Linux
sudo pacman -S gtk3 xdotool

# openSUSE
sudo zypper install gtk3-devel libxdo-devel
```
</details>

## 快速上手

```
1. 启动 gridix
2. Ctrl+N → 新建连接
3. 选择数据库类型，填写连接信息，回车
4. 侧边栏点击表名
5. 用 hjkl 导航，用 i/c/d 编辑
```

## 快捷键速查

### 全局快捷键

| 按键 | 功能 | 按键 | 功能 |
|------|------|------|------|
| `Ctrl+N` | 新建连接 | `Ctrl+Shift+N` | 新建表 |
| `Ctrl+Enter` / `F5` | 执行 SQL | `Ctrl+S` | 保存修改 |
| `Ctrl+B` | 开关侧边栏 | `Ctrl+J` | 开关 SQL 编辑器 |
| `Ctrl+H` | 查询历史 | `Ctrl+R` | 开关 ER 图 |
| `Ctrl+E` | 导出数据 | `Ctrl+I` | 导入数据 |
| `Ctrl+T` | 主题选择 | `Ctrl+D` | 切换日/夜模式 |
| `Ctrl+F` | 添加筛选 | `Ctrl+Shift+F` | 清空筛选 |
| `Ctrl+G` | 跳转到行 | `Ctrl+L` | 清空 SQL 编辑器 |
| `Ctrl+1/2/3` | 聚焦侧边栏区域 | `Ctrl+Tab` | 下一个标签 |
| `Ctrl++/-/0` | 缩放界面 | `F1` | 帮助 |

### 表格导航（Normal 模式）

```
移动:               快速跳转:              翻页:
    k               gg → 第一行           Ctrl+u → 上翻半页
  h   l             G  → 最后一行          Ctrl+d → 下翻半页
    j               gh → 第一列            PageUp/PageDown
                    gl → 最后一列

数字前缀:  5j → 下移 5 行 | 10k → 上移 10 行 | 3w → 右移 3 列
```

### 表格编辑

| 按键 | 功能 | 按键 | 功能 |
|------|------|------|------|
| `i` | 插入模式 | `a` | 追加模式 |
| `c` | 修改单元格 | `r` | 替换模式 |
| `d` | 删除内容 | `dd` / `Space d` | 标记删除行 |
| `y` / `yy` | 复制单元格/行 | `p` | 粘贴 |
| `o` / `O` | 下/上方插入行 | `u` / `U` | 撤销单元格/行 |
| `v` | 选择模式 | `w` / `Ctrl+S` | 保存修改 |

### 选择模式 (`v`)

| 按键 | 功能 |
|------|------|
| `h/j/k/l` | 扩展选择 |
| `x` | 选择整行 |
| `d` | 删除选中 |
| `y` | 复制选中 |
| `Esc` | 退出选择 |

## 数据库支持

| 数据库 | 端口 | 特性 |
|--------|------|------|
| **SQLite** | - | 本地文件、零配置、内置驱动 |
| **PostgreSQL** | 5432 | 异步驱动、连接池、完整功能 |
| **MySQL/MariaDB** | 3306 | 异步驱动、连接池、5 种 SSL 模式 |

### MySQL SSL/TLS 模式

| 模式 | 说明 |
|------|------|
| `Disabled` | 不加密（默认） |
| `Preferred` | 尽量加密 |
| `Required` | 必须加密 |
| `VerifyCa` | 验证服务器证书 |
| `VerifyIdentity` | 验证证书 + 主机名 |

### SSH 隧道

通过跳板机连接远程数据库：

```
你的电脑 ──SSH──> 跳板机 ──────> 数据库服务器
   └── 本地端口 ←── 隧道 ←── 远程端口
```

**支持的认证方式：**
- 密码认证
- 私钥认证（OpenSSH 格式，可带密码保护）

## 数据导入导出

### 导出格式

| 格式 | 选项 |
|------|------|
| **CSV** | 自定义分隔符、引号、是否含表头 |
| **JSON** | 格式化输出或紧凑格式 |
| **SQL** | INSERT 语句、事务包装、批量大小 |

### 导入格式

| 格式 | 特性 |
|------|------|
| **CSV/TSV** | 自动检测分隔符、跳过行数、最大行数 |
| **JSON** | 数组或嵌套对象、JSON 路径支持 |
| **SQL** | 直接执行，带事务 |

## 高级筛选

按 `/` 打开快速筛选，或 `Ctrl+F` 添加条件。

**16 种操作符：**

| 类别 | 操作符 |
|------|--------|
| 文本 | `Contains`、`NotContains`、`Equals`、`NotEquals`、`StartsWith`、`EndsWith`、`Regex` |
| 数值 | `>`、`>=`、`<`、`<=`、`Between`、`NotBetween` |
| 集合 | `In`、`NotIn` |
| 空值 | `IsNull`、`IsNotNull`、`IsEmpty`、`IsNotEmpty` |

支持 `AND` / `OR` 逻辑组合。

## 主题系统

**暗色（11 套）：** Tokyo Night、Tokyo Night Storm、Catppuccin Mocha/Macchiato/Frappé、One Dark、One Dark Vivid、Gruvbox Dark、Dracula、Nord、GitHub Dark

**亮色（8 套）：** Tokyo Night Light、Catppuccin Latte、One Light、Gruvbox Light、Solarized Light/Dark、GitHub Light、Monokai Pro

- `Ctrl+T` - 打开主题选择器
- `Ctrl+D` - 切换日/夜模式
- 日间和夜间主题独立配置

## SQL 编辑器

- **语法高亮** - 关键字、字符串、注释、数字
- **自动补全** - 149 个 SQL 关键字 + 50 个函数 + 表名/列名
- **Tab 补全** - 按 Tab 确认建议
- **历史导航** - `Shift+↑/↓` 浏览历史
- **SQL 格式化** - 一键美化

## ER 关系图

按 `Ctrl+R` 查看表关系：
- 列详情（类型、NULL/NOT NULL、默认值）
- 外键关系（实线连接）
- 命名推断的关系（虚线连接）
- 拖动调整布局

## 配置文件

| 系统 | 路径 |
|------|------|
| Linux | `~/.config/gridix/config.toml` |
| macOS | `~/Library/Application Support/gridix/config.toml` |
| Windows | `%APPDATA%\gridix\config.toml` |

**保存的设置：**
- 数据库连接（密码 AES-256-GCM 加密）
- 主题偏好（日/夜）
- UI 缩放（0.5x - 2.0x）
- 查询历史（最多 100 条）
- 自定义快捷键

## 安全性

| 特性 | 实现方式 |
|------|---------|
| 密码存储 | AES-256-GCM 加密，基于机器特征的密钥派生 |
| SSH 隧道 | 纯 Rust 实现（russh），密码和私钥认证 |
| MySQL SSL/TLS | 5 种安全级别，从禁用到完全验证 |
| 配置文件 | Unix 权限 0600（仅所有者可读写） |

## 技术细节

| 组件 | 技术 |
|------|------|
| GUI 框架 | egui/eframe 0.33 |
| 异步运行时 | Tokio（多线程） |
| SQLite 驱动 | rusqlite 0.38（内置） |
| PostgreSQL 驱动 | tokio-postgres 0.7 |
| MySQL 驱动 | mysql_async 0.36 |
| SSH | russh 0.55 |
| 加密 | ring 0.17 |
| 语法高亮 | syntect 5.3 |

**数据：**
- 代码量 ~9,000 行 Rust
- 二进制 ~22 MB（release、stripped、LTO）
- 启动 <1 秒
- 内存 ~50 MB

## 项目结构

```
src/
├── main.rs                 # 入口
├── app/                    # 应用逻辑（9 个模块）
│   ├── mod.rs              # DbManagerApp
│   ├── state.rs            # 状态管理
│   ├── keyboard.rs         # 全局快捷键
│   ├── database.rs         # 数据库操作
│   └── ...
├── core/                   # 核心功能（12 个模块）
│   ├── autocomplete.rs     # SQL 补全
│   ├── config.rs           # 配置 + 加密
│   ├── keybindings.rs      # 可配置快捷键
│   ├── theme.rs            # 19 套主题
│   └── ...
├── database/               # 数据库层（8 个模块）
│   ├── connection.rs       # 连接管理
│   ├── pool.rs             # 连接池
│   ├── ssh_tunnel.rs       # SSH 隧道
│   └── query/              # 驱动（SQLite、PostgreSQL、MySQL）
└── ui/                     # 用户界面
    ├── components/         # 表格、SQL 编辑器、工具栏等
    │   ├── grid/           # Helix 风格数据表格（7 个模块）
    │   ├── er_diagram/     # ER 图可视化
    │   └── ...
    └── dialogs/            # 14 种对话框
```

## 开发

```bash
cargo run              # 开发构建
cargo test             # 运行测试（13 个测试模块）
cargo clippy           # 代码检查
cargo build --release  # 发布构建
cargo appimage         # 构建 AppImage（Linux）
```

## 贡献

- **报告 Bug：** [Issues](https://github.com/MCB-SMART-BOY/Gridix/issues)
- **功能建议：** [Discussions](https://github.com/MCB-SMART-BOY/Gridix/discussions)
- **代码贡献：** 欢迎 PR！

## 致谢

- [egui](https://github.com/emilk/egui) - 即时模式 GUI
- [Helix Editor](https://helix-editor.com/) - 键位设计灵感
- [Catppuccin](https://github.com/catppuccin) - 配色方案
- [syntect](https://github.com/trishume/syntect) - 语法高亮引擎

## 许可证

[MIT License](LICENSE) - 个人和商业用途均可免费使用。

---

<div align="center">
<i>Less mouse, more keyboard.</i>
</div>
