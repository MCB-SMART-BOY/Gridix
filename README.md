# Rust DB Manager

简洁、快速、安全的跨平台数据库管理工具，专为键盘党打造。

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)

## 特性亮点

- **多数据库支持** - SQLite、PostgreSQL、MySQL/MariaDB
- **Helix/Vim 风格键位** - 三种编辑模式（Normal/Select/Insert），hjkl 导航
- **智能 SQL 编辑器** - 语法高亮、上下文感知自动补全、SQL 格式化
- **高级筛选系统** - 16 种操作符、多条件组合（AND/OR）、正则支持
- **19 种主题预设** - Tokyo Night、Catppuccin、One Dark、Gruvbox 等
- **数据导出** - CSV、JSON、SQL INSERT 语句
- **安全存储** - AES-256-GCM 加密密码
- **轻量高效** - 单二进制文件，GPU 加速渲染，内存占用低

## 截图

![Screenshot](rust-db-manager.png)

## 安装

### 下载预编译版本

从 [Releases](https://github.com/MCB-SMART-BOY/rust-db-manager/releases) 页面下载：

| 平台 | 下载 |
|------|------|
| Linux x86_64 | `rust-db-manager-linux-x86_64.tar.gz` (含 AppImage) |
| Windows x86_64 | `rust-db-manager-windows-x86_64.zip` |
| macOS Apple Silicon | `rust-db-manager-macos-arm64.tar.gz` |
| macOS Intel | `rust-db-manager-macos-x86_64.tar.gz` |

### 从源码编译

```bash
git clone https://github.com/MCB-SMART-BOY/rust-db-manager.git
cd rust-db-manager
cargo run --release
```

**Linux 依赖：**
```bash
sudo apt-get install libgtk-3-dev libxdo-dev
```

## 快捷键

### 全局

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+N` | 新建连接 |
| `Ctrl+Enter` | 执行查询 |
| `Ctrl+J` | 切换 SQL 编辑器 |
| `Ctrl+B` | 切换侧边栏 |
| `Ctrl+H` | 查询历史 |
| `Ctrl+E` | 导出结果 |
| `Ctrl+I` | 导入 SQL 文件 |
| `Ctrl+D` | 切换日/夜模式 |
| `Ctrl+T` | 主题选择 |
| `Ctrl+1/2/3` | 快速切换 连接/数据库/表 |
| `Ctrl++/-/0` | 缩放界面 |
| `Ctrl+F` | 添加筛选条件 |
| `Ctrl+Shift+F` | 清空筛选 |
| `Ctrl+G` | 跳转到行 |
| `Ctrl+S` | 保存修改 |
| `F5` | 刷新表列表 |
| `F1` | 帮助 |

### 表格编辑（Helix 风格）

**Normal 模式：**

| 快捷键 | 功能 |
|--------|------|
| `h/j/k/l` | 左/下/上/右移动 |
| `w/b` | 向右/左跳列 |
| `gg/G` | 跳到首行/末行 |
| `gh/gl` | 跳到行首/行尾 |
| `Ctrl+u/d` | 向上/下翻半页 |
| `5j` | 向下移动 5 行（数字前缀） |

**编辑操作：**

| 快捷键 | 功能 |
|--------|------|
| `i/a` | 进入插入模式 |
| `c` | 修改单元格 |
| `r` | 替换单元格 |
| `v` | 进入选择模式 |
| `x` | 选择整行 |
| `d` | 删除内容 |
| `y/p` | 复制/粘贴 |
| `u` | 撤销修改 |
| `o/O` | 在下方/上方新增行 |
| `Space+d` | 标记删除行 |
| `/` | 快速筛选对话框 |

## 主题

支持 19 种预设主题，日/夜模式独立配置：

**暗色主题：** Tokyo Night、Tokyo Night Storm、Catppuccin Mocha/Macchiato/Frappé、One Dark、One Dark Vivid、Gruvbox Dark、Dracula、Nord、Solarized Dark、Monokai Pro、GitHub Dark

**亮色主题：** Tokyo Night Light、Catppuccin Latte、One Light、Gruvbox Light、Solarized Light、GitHub Light

## SQL 自动补全

- **149+ SQL 关键字** - SELECT、INSERT、UPDATE、DELETE、JOIN 等
- **50+ SQL 函数** - COUNT、SUM、AVG、CONCAT、DATE 等
- **表名补全** - 从数据库动态加载
- **列名补全** - 上下文感知（FROM/JOIN/WHERE 后自动提示）

## 高级筛选

支持 16 种操作符：

| 类型 | 操作符 |
|------|--------|
| 文本 | Contains、NotContains、Equals、NotEquals、StartsWith、EndsWith、Regex |
| 比较 | GreaterThan、GreaterOrEqual、LessThan、LessOrEqual |
| 范围 | Between、Empty、NotEmpty |
| 逻辑 | AND、OR 组合 |

## 项目结构

```
src/
├── main.rs              # 程序入口
├── app.rs               # 主应用逻辑
├── core/                # 核心功能
│   ├── autocomplete.rs  # SQL 自动补全
│   ├── config.rs        # 配置管理
│   ├── export.rs        # 数据导出
│   ├── formatter.rs     # SQL 格式化
│   ├── history.rs       # 查询历史
│   ├── syntax.rs        # 语法高亮
│   └── theme.rs         # 主题管理
├── database/            # 数据库连接
│   ├── mod.rs           # 连接池管理
│   └── query.rs         # 查询执行
└── ui/                  # 用户界面
    ├── components/      # 组件（工具栏、搜索栏、表格等）
    ├── dialogs/         # 对话框
    └── panels/          # 面板（侧边栏、历史面板）
```

## 技术栈

| 组件 | 技术 |
|------|------|
| GUI 框架 | egui 0.27 / eframe |
| 异步运行时 | Tokio（多线程） |
| SQLite | rusqlite 0.31 (bundled) |
| PostgreSQL | tokio-postgres 0.7 |
| MySQL | mysql_async 0.33 |
| 加密 | ring 0.17 (AES-256-GCM) |
| 序列化 | serde + toml |

## 配置文件

配置自动保存到：
- **Linux/macOS:** `~/.config/rust-db-manager/config.toml`
- **Windows:** `%APPDATA%\rust-db-manager\config.toml`

包含：连接信息、主题设置、查询历史、UI 缩放等。

## 开发

```bash
# 开发模式
cargo run

# Release 构建
cargo build --release

# 构建 AppImage (Linux)
cargo appimage

# 运行测试
cargo test
```

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
