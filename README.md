# Gridix

<div align="center">

**A keyboard-first database manager for command-style workflow**  
**面向命令式工作流的键盘优先数据库管理工具**

[![Version](https://img.shields.io/badge/version-7.1.0-blue.svg)](https://github.com/MCB-SMART-BOY/Gridix/releases)
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
nix run github:MCB-SMART-BOY/Gridix/v7.1.0
nix profile install github:MCB-SMART-BOY/Gridix/v7.1.0

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
- Full area-by-area guide / 分区域完整指南: see `.claude/CLAUDE.md` keyboard routing section

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
| Database | Typed runtime | Cancellation semantics |
|---|---|---|
| SQLite | Local file DB, bundled driver / 本地文件库，内置驱动 | The typed API accepts a cancellation token, but an already-running synchronous `rusqlite` statement is not interrupted. |
| PostgreSQL | Async typed execution; `NUMERIC` parameters and results preserve exact `DbValue::Decimal` text | A cancellation token sends PostgreSQL `CancelRequest` through the driver's `CancelToken`, then waits for the executing query to finish with the cancellation result. |
| MySQL/MariaDB | Async typed execution + SSL/TLS options; temporal inputs reject nanoseconds at or above one second | A cancellation token keeps the execution connection's `Conn::id`, opens a separate TLS-configured control connection, sends `KILL QUERY`, then waits for the original query to finish. |

The public typed entry points are `execute_typed` and `execute_typed_cancellable`. The cancellable entry is cooperative: PostgreSQL and MySQL can request server-side cancellation; SQLite does not promise in-flight statement interruption.

`execute_typed` 与 `execute_typed_cancellable` 是公开的类型化执行入口。可取消入口采用协作式语义：PostgreSQL 和 MySQL 可以请求服务端取消；SQLite 不承诺中断已执行中的语句。

## Documentation | 文档
- Docs index / 文档索引: [docs/README.md](docs/README.md)
- Changelog / 版本变更: [docs/CHANGELOG.md](docs/CHANGELOG.md)
- Learning curriculum / 学习路线: [docs/LEARNING_CURRICULUM.md](docs/LEARNING_CURRICULUM.md)

## Development | 开发
```bash
cargo run
cargo test
cargo clippy
cargo build --release
```

### Backend integration | 后端集成测试

Set the appropriate local test URL before running these explicit integration binaries. The tests are not ignored, and every command runs serially. The typed-E2E binaries use fixed table names, so serial execution prevents their tests from interfering. Use a disposable database; do not put a real credential in shell history or documentation.

运行以下显式集成测试前，请设置相应的本地测试 URL。测试未被标记为 ignored；每条命令均串行运行。typed E2E 二进制使用固定表名，串行运行可避免测试相互干扰。请使用可丢弃的数据库，不要把真实凭据写入 shell history 或文档。

```bash
GRIDIX_TEST_PG_URL='<PostgreSQL test URL>' \
  cargo test --test postgres_typed_e2e -- --nocapture --test-threads=1
GRIDIX_TEST_PG_URL='<PostgreSQL test URL>' \
  cargo test --test postgres_cancel_integration -- --nocapture --test-threads=1

GRIDIX_TEST_MYSQL_URL='<MySQL test URL>' \
  cargo test --test mysql_typed_e2e -- --nocapture --test-threads=1
GRIDIX_TEST_MYSQL_URL='<MySQL test URL>' \
  cargo test --test mysql_cancel_integration -- --nocapture --test-threads=1
```

GitHub Actions runs the PostgreSQL and MySQL typed integration gates on pull requests, `main`, and `v*` tags, as well as by manual dispatch and a weekly schedule. Each gate preflights its required URL, uses its dedicated service container, and logs the serial test runs with `--nocapture`. These are release-acceptance gates, not a claim that a release has been published. A manual SQLite GUI journey (create, edit/save, reopen, and CSV/JSON/SQL export evidence) is still required for release acceptance; the current `gridix-driver` supports only `launch`, `key`, `ss`, `quit`, and `help`, so it cannot complete that journey automatically.

## Contributing | 参与贡献
- Issues: https://github.com/MCB-SMART-BOY/Gridix/issues
- Discussions: https://github.com/MCB-SMART-BOY/Gridix/discussions
- Pull requests are welcome. / 欢迎提交 PR。

## License | 许可证
[Apache License 2.0](LICENSE)
