# Gridix

> 给不想碰鼠标的人做的数据库工具

![Version](https://img.shields.io/badge/version-0.5.2-green.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)
[![AUR](https://img.shields.io/aur/version/gridix-bin?label=AUR)](https://aur.archlinux.org/packages/gridix-bin)

**Gridix** = Grid + Helix。用 `hjkl` 操作数据库，用 Vim 的方式编辑表格。

![Screenshot](gridix.png)

## 凭什么用它？

**够快** - 纯 Rust，启动不到 1 秒，22MB 单文件，不是 Electron 套壳货

**够安全** - SSH 隧道连跳板机，SSL/TLS 加密传输，AES-256-GCM 存密码

**够顺手** - Helix 键位，`hjkl` 移动，`c` 改内容，`gg` `G` 跳转，你懂的

**够好看** - 19 套主题随便换，Catppuccin、Tokyo Night、Dracula 都有

**够全面** - SQLite、PostgreSQL、MySQL 三大主流全支持

## 装一个

### Arch Linux (AUR)

```bash
paru -S gridix-bin          # 预编译，秒装
paru -S gridix-appimage     # AppImage，自带依赖
paru -S gridix              # 源码编译，硬核
```

### NixOS / Nix

```bash
# nixpkgs (unstable)
nix-shell -p gridix

# 或使用 flake
nix run github:MCB-SMART-BOY/Gridix
```

### Flatpak (Flathub)

```bash
flatpak install flathub io.github.mcb_smart_boy.Gridix
flatpak run io.github.mcb_smart_boy.Gridix
```

### macOS / Linux (Homebrew)

```bash
brew tap MCB-SMART-BOY/gridix
brew install gridix
```

### 下载预编译

去 [Releases](https://github.com/MCB-SMART-BOY/gridix/releases) 下载：

| 平台 | 文件 | 大小 |
|------|------|------|
| Linux | `gridix-linux-x86_64.tar.gz` | ~13 MB |
| Linux | `gridix.AppImage` | ~17 MB |
| Windows | `gridix-windows-x86_64.zip` | ~12 MB |
| macOS (M1/M2/M3/M4) | `gridix-macos-arm64.tar.gz` | ~12 MB |
| macOS (Intel) | `gridix-macos-x86_64.tar.gz` | ~12 MB |

### 源码编译

```bash
git clone https://github.com/MCB-SMART-BOY/gridix.git
cd gridix
cargo build --release
```

<details>
<summary>Linux 依赖</summary>

```bash
# Debian/Ubuntu
sudo apt-get install libgtk-3-dev libxdo-dev

# Fedora
sudo dnf install gtk3-devel libxdo-devel

# Arch
sudo pacman -S gtk3 xdotool

# openSUSE
sudo zypper install gtk3-devel libxdo-devel
```
</details>

## 快速上手

1. 启动程序
2. `Ctrl+N` 新建连接
3. 选数据库类型，填连接信息，回车
4. 侧边栏点个表
5. 用 `hjkl` 开始探索

## 键盘操作

这是核心卖点，认真看。

### 全局快捷键

| 按键 | 干啥 |
|------|------|
| `Ctrl+N` | 新建连接 |
| `Ctrl+Enter` | 执行 SQL |
| `Ctrl+B` | 开关侧边栏 |
| `Ctrl+J` | 开关 SQL 编辑器 |
| `Ctrl+H` | 查询历史 |
| `Ctrl+E` | 导出数据 |
| `Ctrl+I` | 导入数据 |
| `Ctrl+T` | 选主题 |
| `Ctrl+D` | 日/夜模式切换 |
| `Ctrl+1/2/3` | 快速切换连接/数据库/表 |
| `Ctrl++/-/0` | 缩放界面 |
| `Ctrl+F` | 添加筛选条件 |
| `Ctrl+Shift+F` | 清空筛选 |
| `Ctrl+G` | 跳转到指定行 |
| `Ctrl+S` | 保存修改 |
| `F5` | 刷新表列表 |
| `F1` | 帮助 |

### 表格导航（Normal 模式）

```
基础移动：
     k
   h   l      ← 就这四个键，上下左右
     j

快速跳转：
  gg    → 跳到第一行
  G     → 跳到最后一行
  gh    → 跳到行首
  gl    → 跳到行尾

翻页：
  Ctrl+u  → 向上翻半页
  Ctrl+d  → 向下翻半页

数字前缀：
  5j    → 向下移动 5 行
  10k   → 向上移动 10 行
  3w    → 向右跳 3 列
```

### 表格编辑

| 按键 | 干啥 |
|------|------|
| `i` | 在当前位置插入 |
| `a` | 在当前位置后插入 |
| `c` | 修改单元格内容 |
| `r` | 替换单元格 |
| `d` | 删除内容 |
| `y` | 复制 |
| `p` | 粘贴 |
| `u` | 撤销 |
| `o` | 下方新增一行 |
| `O` | 上方新增一行 |
| `Space d` | 标记删除整行 |

### 选择模式

| 按键 | 干啥 |
|------|------|
| `v` | 进入选择模式 |
| `x` | 选择整行 |
| `Esc` | 退出选择模式 |

### 插入模式

| 按键 | 干啥 |
|------|------|
| `Esc` | 退出插入模式 |
| `Enter` | 确认输入 |

## 数据库支持

| 数据库 | 默认端口 | 特殊功能 |
|--------|----------|----------|
| SQLite | - | 本地文件，零配置 |
| PostgreSQL | 5432 | 全功能支持 |
| MySQL/MariaDB | 3306 | SSL/TLS 加密 |

## SSH 隧道

连不上远程数据库？要过跳板机？用 SSH 隧道：

```
你的电脑 → SSH 服务器 → 数据库服务器
```

支持两种认证：
- 密码认证
- 私钥认证（支持 OpenSSH 格式）

在连接对话框里勾选"启用 SSH 隧道"，填好 SSH 信息就行。

## MySQL SSL/TLS

5 种安全级别：

| 模式 | 说明 |
|------|------|
| Disabled | 不加密（默认） |
| Preferred | 能加密就加密 |
| Required | 必须加密 |
| VerifyCa | 验证服务器证书 |
| VerifyIdentity | 验证证书 + 主机名 |

## 数据导入导出

### 导出

支持三种格式：
- **CSV** - 自定义分隔符、引号、是否含表头
- **JSON** - 可选格式化输出
- **SQL** - INSERT 语句，可选事务包装

还能选择导出哪些列、从哪行开始、导出多少行。

### 导入

支持导入：
- **CSV/TSV** - 自动检测分隔符
- **JSON** - 支持数组和嵌套对象
- **SQL** - 直接执行

导入前会生成预览，确认没问题再执行。

## 高级筛选

16 种操作符：

| 类型 | 操作符 |
|------|--------|
| 文本匹配 | Contains, NotContains, Equals, NotEquals |
| 文本模式 | StartsWith, EndsWith, Regex |
| 数值比较 | GreaterThan, GreaterOrEqual, LessThan, LessOrEqual |
| 范围 | Between |
| 空值 | Empty, NotEmpty |
| 逻辑 | AND, OR 组合 |

按 `/` 打开快速筛选对话框。

## 主题系统

**暗色（11套）：**
Tokyo Night, Tokyo Night Storm, Catppuccin Mocha, Catppuccin Macchiato, Catppuccin Frappé, One Dark, One Dark Vivid, Gruvbox Dark, Dracula, Nord, Monokai Pro, GitHub Dark

**亮色（8套）：**
Tokyo Night Light, Catppuccin Latte, One Light, Gruvbox Light, Solarized Light, GitHub Light

日夜模式独立配置，白天自动用亮色，晚上自动用暗色。

`Ctrl+D` 快速切换，`Ctrl+T` 打开选择器。

## SQL 编辑器

- **语法高亮** - 关键字、字符串、注释、数字都有颜色
- **自动补全** - 149 个 SQL 关键字 + 50 个函数 + 表名 + 列名
- **SQL 格式化** - 一键美化，告别意大利面条

## 配置文件

| 系统 | 路径 |
|------|------|
| Linux | `~/.config/gridix/config.toml` |
| macOS | `~/Library/Application Support/gridix/config.toml` |
| Windows | `%APPDATA%\gridix\config.toml` |

存了啥：
- 数据库连接信息（密码加密存储）
- 主题设置
- 窗口位置和大小
- 缩放比例
- 查询历史

## 技术细节

| 组件 | 用的啥 |
|------|--------|
| GUI | egui/eframe 0.29 |
| 异步 | Tokio |
| SQLite | rusqlite |
| PostgreSQL | tokio-postgres |
| MySQL | mysql_async |
| SSH | russh |
| 加密 | ring (AES-256-GCM) |
| 语法高亮 | syntect |

数据：
- 代码量：~10,000 行 Rust
- 二进制大小：~22 MB
- 内存占用：~50 MB（看数据量）
- 启动时间：< 1 秒

## 项目结构

```
src/
├── main.rs              # 入口
├── app.rs               # 主逻辑
├── core/                # 核心功能
│   ├── autocomplete.rs  # 自动补全
│   ├── config.rs        # 配置管理
│   ├── export.rs        # 导入导出
│   ├── formatter.rs     # SQL 格式化
│   ├── history.rs       # 查询历史
│   ├── syntax.rs        # 语法高亮
│   └── theme.rs         # 主题
├── database/            # 数据库层
│   ├── mod.rs           # 连接池
│   ├── query.rs         # 查询引擎
│   └── ssh_tunnel.rs    # SSH 隧道
└── ui/                  # 界面
    ├── components/      # 组件
    ├── dialogs/         # 对话框
    └── panels/          # 面板
```

## 更新日志

### v0.5.2 (2025-12-20)
- 修复 LICENSE 文件未包含在发布标签中的问题
- 更新打包配置

### v0.5.1 (2025-12-20)
- 上架 AUR（gridix-bin / gridix / gridix-appimage）
- 新增 AppImage 打包
- 修复 GitHub Actions macOS 构建

### v0.5.0 (2025-12-20)
- Helix 键盘操作完整支持
- 历史面板支持 j/k/g/G 导航
- 表格列宽智能自适应
- 对话框焦点管理优化

### v0.4.0 (2025-12-18)
- 所有对话框支持键盘导航
- Esc/q 关闭、Enter 确认
- 确认框支持 y/n
- GitHub Actions 自动构建

### v0.3.0 (2025-12-15)
- 侧边栏 Ctrl+1/2/3 快速切换
- 侧边栏 j/k/g/G/Enter 导航
- 数据导入功能
- 关于对话框

### v0.2.0 (2025-12-10)
- MySQL SSL/TLS（5 种模式）
- SSH 隧道连接
- 多标签查询
- DDL 查看

### v0.1.0 (2024-12-09)
- 初始版本
- SQLite/PostgreSQL/MySQL 支持
- Helix 键位
- 19 种主题
- SQL 自动补全

## 开发

```bash
# 克隆
git clone https://github.com/MCB-SMART-BOY/gridix.git
cd gridix

# 开发运行
cargo run

# 测试
cargo test

# 代码检查
cargo clippy

# Release 构建
cargo build --release

# 打包 AppImage
cargo appimage
```

## 贡献

- 报 Bug：[Issues](https://github.com/MCB-SMART-BOY/gridix/issues)
- 提建议：[Discussions](https://github.com/MCB-SMART-BOY/gridix/discussions)
- 贡献代码：[Pull Requests](https://github.com/MCB-SMART-BOY/gridix/pulls)

## 致谢

- [egui](https://github.com/emilk/egui) - 即时模式 GUI 框架
- [Helix](https://helix-editor.com/) - 键位设计灵感来源
- [Catppuccin](https://github.com/catppuccin) - 好看的配色
- [syntect](https://github.com/trishume/syntect) - 语法高亮引擎

## 许可证

MIT License - 随便用，改着玩，商用也行。

---

*少点鼠标，多写代码。* ⌨️
