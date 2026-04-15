# Gridix

<div align="center">

**A keyboard-first database manager for command-style workflow**  
**面向命令式工作流的键盘优先数据库管理工具**

[![Version](https://img.shields.io/badge/version-6.0.0-blue.svg)](https://github.com/MCB-SMART-BOY/Gridix/releases)
[![License](https://img.shields.io/badge/license-Apache%202.0-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)]()
[![AUR](https://img.shields.io/aur/version/gridix-bin?label=AUR&logo=archlinux)](https://aur.archlinux.org/packages/gridix-bin)
[![Homebrew](https://img.shields.io/badge/homebrew-tap-brown?logo=homebrew)](https://github.com/MCB-SMART-BOY/homebrew-gridix)
[![Nixpkgs](https://img.shields.io/badge/nixpkgs-search-blue?logo=nixos)](https://search.nixos.org/packages?query=gridix)

</div>

Gridix = Grid + Helix.  
Navigate with `hjkl`, run SQL quickly, and keep onboarding available in-app (`F1`).  
用 `hjkl` 导航、快速执行 SQL，并在应用内通过 `F1` 完成新手上手与数据库学习。

![Gridix Logo](assets/branding/gridix-logo.png)

## At A Glance | 快速了解
- **Keyboard-first focus flow**: sidebar, grid, SQL editor, toolbar.
  **键盘优先焦点流转**：侧边栏、数据表格、SQL 编辑器、工具栏。
- **Unified database UX**: SQLite, PostgreSQL, MySQL/MariaDB.
  **统一数据库体验**：SQLite、PostgreSQL、MySQL/MariaDB。
- **Beginner-friendly onboarding**: welcome hints + learning guide (`F1`).
  **新手友好引导**：欢迎页提示 + 学习指南（`F1`）。
- **Practical security**: encrypted credentials + SSH tunnel + SSL/TLS.
  **实用安全能力**：凭据加密 + SSH 隧道 + SSL/TLS。

## Install | 安装

### Package Managers | 包管理器
```bash
# Arch Linux (AUR)
paru -S gridix-bin
paru -S gridix-appimage
paru -S gridix

# Nix
# latest from default branch
nix run github:MCB-SMART-BOY/Gridix
nix profile install github:MCB-SMART-BOY/Gridix

# pinned release
nix run github:MCB-SMART-BOY/Gridix/v6.0.0
nix profile install github:MCB-SMART-BOY/Gridix/v6.0.0

# Homebrew (macOS/Linux)
brew tap MCB-SMART-BOY/gridix
brew install gridix
```

### Release Binaries | 预编译下载
Download from / 从 [GitHub Releases](https://github.com/MCB-SMART-BOY/Gridix/releases) 下载：

| Platform | Arch | Artifact |
|---|---|---|
| Linux | x86_64 | `gridix-linux-x86_64.tar.gz` |
| Linux | x86_64 | `gridix.AppImage` |
| Windows | x86_64 | `gridix-windows-x86_64.zip` |
| macOS | arm64 | `gridix-macos-arm64.tar.gz` |

### Build From Source | 源码构建
```bash
git clone https://github.com/MCB-SMART-BOY/Gridix.git
cd Gridix
cargo build --release
```

<details>
<summary><b>Linux dependencies | Linux 依赖</b></summary>

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

## 5-Minute Start | 5 分钟上手
1. Press `Ctrl+N` to create the first connection.  
   按 `Ctrl+N` 创建第一个连接。
2. New users can start with SQLite sample path first.  
   新手建议先走 SQLite 学习示例路径。
3. Select a table in sidebar, navigate with `hjkl`.  
   在侧边栏选表并用 `hjkl` 导航。
4. Open SQL editor (`Ctrl+J`), execute by `Ctrl+Enter` (or `F5` while the SQL editor owns focus).
   打开 SQL 编辑器（`Ctrl+J`），用 `Ctrl+Enter` 执行 SQL（SQL 编辑器拥有焦点时也可使用 `F5`）。
5. Press `F1` to open Help & Learning.  
   按 `F1` 打开帮助与学习。

## Keyboard Model | 键盘模型
- Gridix uses a scope-aware, keyboard-first model.
  Gridix 采用作用域感知的键盘优先交互模型。
- `Tab / Shift+Tab` are default bindings for `next_focus_area / prev_focus_area`, not unconditional global-first keys.
  `Tab / Shift+Tab` 是 `next_focus_area / prev_focus_area` 的默认绑定，不是无条件 global-first 按键。
- Full area-by-area guide / 分区域完整指南: [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)

## Core Features | 核心能力
| Area | Description |
|---|---|
| Navigation / 导航 | Helix/Vim style movement across sidebar/grid/editor |
| SQL / 查询 | Highlight, autocomplete, history, execute, explain |
| Data / 数据 | Editable grid, filtering, import/export |
| Learning / 学习 | Tool quick start + database knowledge roadmap |
| Modeling / 建模 | ER diagram and relationship navigation |
| Security / 安全 | Encrypted credentials, SSH tunnel, SSL/TLS |
| Theming / 主题 | Built-in themes, default dark theme: Tokyo Night Storm |

## Database Support | 数据库支持
| Database | Notes |
|---|---|
| SQLite | Local file DB, bundled driver / 本地文件库，内置驱动 |
| PostgreSQL | Async connection / 异步连接 |
| MySQL/MariaDB | Async connection + SSL/TLS options / 异步连接 + SSL/TLS 选项 |

## Documentation | 文档
- Docs index / 文档索引: [docs/README.md](docs/README.md)
- Getting started / 新手上手: [docs/GETTING_STARTED.md](docs/GETTING_STARTED.md)
- Keyboard guide / 键盘指南: [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)
- Architecture / 架构说明: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Changelog / 版本变更: [docs/CHANGELOG.md](docs/CHANGELOG.md)

## Development | 开发
```bash
cargo run
cargo test
cargo clippy
cargo build --release
```

Optional MySQL integration test / 可选 MySQL 集成测试：
```bash
GRIDIX_IT_MYSQL_HOST=127.0.0.1 \
GRIDIX_IT_MYSQL_PORT=3306 \
GRIDIX_IT_MYSQL_USER=root \
GRIDIX_IT_MYSQL_PASSWORD=secret \
GRIDIX_IT_MYSQL_DB=test \
cargo test --test mysql_cancel_integration -- --ignored --nocapture
```

## Contributing | 参与贡献
- Issues: https://github.com/MCB-SMART-BOY/Gridix/issues
- Discussions: https://github.com/MCB-SMART-BOY/Gridix/discussions
- Pull requests are welcome. / 欢迎提交 PR。

## License | 许可证
[Apache License 2.0](LICENSE)
