# Gridix

> 给不想碰鼠标的人做的数据库工具

![Version](https://img.shields.io/badge/version-0.5.1-green.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
[![AUR](https://img.shields.io/aur/version/gridix-bin?label=AUR)](https://aur.archlinux.org/packages/gridix-bin)

**Gridix** = Grid + Helix。用 `hjkl` 操作数据库，用 Vim 的方式编辑表格。

![Screenshot](gridix.png)

## 凭什么用它？

**够快** - 纯 Rust 写的，启动不到 1 秒，不是 Electron 套壳货

**够安全** - SSH 隧道连跳板机，SSL/TLS 加密传输，AES-256 存密码

**够顺手** - Helix 键位，`hjkl` 移动，`c` 改内容，`gg` `G` 跳转，你懂的

**够好看** - 19 套主题随便换，Catppuccin、Tokyo Night、Dracula 都有

## 装一个

**Arch 用户（爽）：**
```bash
paru -S gridix-bin
```

**其他人：** 去 [Releases](https://github.com/MCB-SMART-BOY/gridix/releases) 下载

**硬核玩家：**
```bash
git clone https://github.com/MCB-SMART-BOY/gridix.git
cd gridix && cargo build --release
```

## 怎么用

启动后 `Ctrl+N` 建连接，选个表，然后：

```
移动      h j k l          （左下上右，和 Vim 一样）
快速跳    gg / G           （首行 / 末行）
翻页      Ctrl+u / Ctrl+d  （上 / 下半页）
改内容    c                 （进入编辑）
删除      d                 （删掉）
复制粘贴  y / p             （你懂的）
撤销      u                 （后悔药）
新增行    o / O             （下方 / 上方）
删整行    Space d           （空格再按d）
执行SQL   Ctrl+Enter        （跑起来）
```

不会？按 `F1` 看帮助。

## 能干啥

| 功能 | 支持程度 |
|------|----------|
| SQLite / PostgreSQL / MySQL | ✅ 都行 |
| SSH 隧道 | ✅ 密码和密钥都支持 |
| SSL/TLS | ✅ 5 种模式 |
| 导入导出 | ✅ CSV / JSON / SQL |
| 筛选过滤 | ✅ 16 种操作符 |
| 语法高亮 | ✅ 自动补全也有 |
| 多标签页 | ✅ 同时开多个查询 |
| 暗色主题 | ✅ 11 套 |
| 亮色主题 | ✅ 8 套 |

## 一些快捷键

| 干啥 | 按啥 |
|------|------|
| 新建连接 | `Ctrl+N` |
| 执行 SQL | `Ctrl+Enter` |
| 切侧边栏 | `Ctrl+B` |
| 切编辑器 | `Ctrl+J` |
| 查历史 | `Ctrl+H` |
| 导出 | `Ctrl+E` |
| 导入 | `Ctrl+I` |
| 换主题 | `Ctrl+T` |
| 日/夜切换 | `Ctrl+D` |
| 筛选 | `/` |
| 刷新 | `F5` |

## 主题预览

暗的：Tokyo Night / Catppuccin Mocha / One Dark / Gruvbox Dark / Dracula / Nord...

亮的：Tokyo Night Light / Catppuccin Latte / One Light / Gruvbox Light...

`Ctrl+T` 打开选择器，挑一个顺眼的。

## 配置在哪

- Linux: `~/.config/gridix/config.toml`
- macOS: `~/Library/Application Support/gridix/config.toml`  
- Windows: `%APPDATA%\gridix\config.toml`

密码加密存的，放心。

## 更新记录

| 版本 | 干了啥 |
|------|--------|
| 0.5.1 | 上了 AUR，打了 AppImage |
| 0.5.0 | Helix 键位全面支持，列宽自适应 |
| 0.4.0 | 对话框也能用键盘了，GitHub Actions 自动构建 |
| 0.3.0 | 侧边栏键盘导航，加了导入功能 |
| 0.2.0 | SSH 隧道，MySQL SSL |
| 0.1.0 | 能用了 |

## 有问题？

- Bug 和建议：[Issues](https://github.com/MCB-SMART-BOY/gridix/issues)
- 想贡献代码：[Pull Requests](https://github.com/MCB-SMART-BOY/gridix/pulls)

MIT 协议，随便用。

---

*少点鼠标，多写代码。*
