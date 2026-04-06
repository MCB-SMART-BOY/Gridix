# Changelog

All notable changes to this project are documented in this file.  
本文件记录项目的重要变更。

## [Unreleased]

- No unreleased changes yet.
  暂无未发布变更。

## [3.7.1]

### Changed
- Expanded the in-app learning sample into a versioned large relational dataset with 8 main tables, 100+ rows per table, and richer multi-hop relationships.
  将内置学习示例扩展为版本化的大型关系型数据集，包含 8 张主表、每表 100+ 行，并提供更丰富的多跳关系。
- Updated learning-guide overview and onboarding copy so the sample database is described as a real teaching dataset instead of a tiny demo.
  更新学习指南总览与新手引导文案，使示例数据库被明确描述为真实教学数据集，而不是小型演示库。

### Fixed
- Fixed focus-routing regressions where sidebar-to-grid transfer, SQL editor cancel behavior, and DataGrid horizontal movement could stop responding consistently.
  修复焦点路由回归问题，解决侧边栏到表格的切换、SQL 编辑器取消行为以及数据表格横向移动不能稳定响应的问题。
- Fixed legacy learning-sample databases so older files are detected and rebuilt instead of failing to open after the dataset upgrade.
  修复旧版学习示例数据库兼容性问题，使旧文件会被识别并自动重建，而不是在数据集升级后无法打开。
- Fixed a query-learning edge case where sample mutation demos could clash with the new seeded dataset and constraints.
  修复查询学习示例中的边界问题，避免示例更新/删除演示与新的种子数据和约束发生冲突。

## [3.7.0]

### Added
- Added scope-tree navigation, issue-only filtering, and conflict-summary jumping to the keybinding settings dialog.
  为快捷键设置对话框新增作用域树导航、仅看问题项过滤以及冲突摘要跳转。
- Added structured `grid.normal.*` command-sequence support to `keymap.toml` for DataGrid command chains.
  为 `keymap.toml` 新增结构化 `grid.normal.*` 命令序列支持，用于配置数据表格命令链。
- Added regression coverage for scope-tree filtering, issue summaries, and custom Grid command-sequence overrides.
  新增作用域树筛选、冲突摘要以及数据表格自定义命令序列覆盖的回归测试。

### Changed
- Reworked the keybinding settings dialog from flat filters into a scope-tree driven workflow with richer issue analysis.
  将快捷键设置界面从平铺筛选重构为作用域树驱动流程，并增强问题分析能力。
- DataGrid mode help and high-frequency action tooltips now reflect the current runtime command sequences instead of fixed literals.
  数据表格模式帮助和高频操作提示改为反映当前运行时命令序列，不再写死固定字面量。

### Fixed
- Fixed a regression where configurable Grid command prefixes could break counted `gg` jumps.
  修复数据表格可配置命令前缀引入后可能破坏带计数 `gg` 跳转的回归问题。
- Fixed another gap between configurable shortcut infrastructure and DataGrid’s hard-coded command chains.
  修复快捷键可配置基础设施与数据表格硬编码命令链之间的又一处断层。

## [3.6.0]

### Added
- Added original Gridix brand assets under `assets/branding`, including a square app icon and a horizontal wordmark.
  新增原创 Gridix 品牌资产，统一放入 `assets/branding`，包括方形应用图标和横版字标。
- Added native window icon loading from the packaged branding icon.
  新增原生窗口图标加载，直接使用正式品牌图标。
- Added structured local keymap sections and runtime local-shortcut overrides on top of external `keymap.toml`.
  在外置 `keymap.toml` 之上新增结构化局部键位 section 与运行时局部快捷键覆盖能力。
- Added high-level Grid keyboard regression tests covering prefixes, counts, selection, save/quit commands, and filter entry.
  新增表格键盘高层回归测试，覆盖前缀命令、计数、选择模式、保存/退出命令以及筛选入口。

### Changed
- Moved packaging and runtime icon references from the repository root into `assets/branding`.
  将打包与运行时图标引用从仓库根目录迁移到 `assets/branding`。
- Updated README branding display to use the dedicated logo asset instead of the old root image path.
  README 的品牌展示改为使用专门的 logo 资产，不再使用旧的根目录图片路径。
- Updated desktop metadata to better reflect Gridix as a database tool.
  更新桌面文件元数据，使其更准确反映 Gridix 的数据库工具定位。
- Reworked dialogs, help/history panels, sidebar, and DataGrid around a shared local action/shortcut layer.
  对话框、帮助/历史面板、侧边栏与数据表格进一步重构为共享的局部动作/快捷键层。
- Expanded shortcut discoverability so hover hints and learning/help content increasingly reflect the current runtime keymap.
  扩展快捷键可发现性，悬停提示与学习/帮助内容开始更多地反映当前运行时真实键位。

### Fixed
- Fixed a branding/resource inconsistency where the root `gridix.png` had become an ad-hoc distribution asset.
  修复品牌资源不一致问题，根目录 `gridix.png` 不再作为临时发行资产继续扩散。
- Fixed several legacy keyboard paths that still relied on deprecated dialog-level handlers.
  修复多处仍依赖旧式对话框级键盘处理器的遗留路径。
- Fixed DataGrid command-buffer edge cases, including stuck prefixes, leaked counts, and incorrect `2gg` / `2G` row jumps.
  修复数据表格命令缓冲区边界问题，包括前缀卡死、计数泄漏以及 `2gg` / `2G` 跳行错误。
- Fixed sidebar command-prefix behavior so `gg` and `gs` work reliably inside the workflow list.
  修复侧边栏命令前缀行为，使 `gg` 与 `gs` 能在工作流列表中可靠工作。

## [3.4.0]

### Added
- Added focus-scoped input routing foundation and keyboard architecture RFC/spec docs.
  新增按焦点作用域分发输入的基础设施，并补充键盘架构 RFC/规范文档。
- Added external `keymap.toml` loading, generation, merge-backfill, and validation path.
  新增外置 `keymap.toml` 的加载、初始化生成、补齐合并与校验链路。
- Added unified local-shortcut tooltip/label helpers and wired them into toolbar, dialogs, sidebar, grid, help, and welcome UI.
  新增统一的局部快捷键提示/标签工具，并接入工具栏、对话框、侧边栏、表格、帮助和欢迎页。
- Added dedicated TSV import/export support with tests.
  新增正式的 TSV 导入/导出支持，并补充相应测试。

### Changed
- Reworked sidebar defaults and workflow toward beginner-friendly connections + filters layout.
  侧边栏默认布局和工作流重构为更适合新手的“连接 + 筛选”优先模式。
- Moved more workspace actions behind scoped helpers instead of global shortcut grabs.
  更多工作区动作改为走作用域化 helper，而不是被全局快捷键直接抢占。
- Help, welcome, learning guide, and configuration docs now reflect real runtime key bindings instead of hard-coded shortcuts.
  帮助、欢迎页、学习指南和配置文档现在会反映运行时真实键位，而不是硬编码快捷键。
- CSV/TSV/JSON import preview now also prepares generated SQL, enabling copy-to-editor flow before execution.
  CSV/TSV/JSON 导入预览现在会同步生成 SQL，可先复制到编辑器检查后再执行。

### Fixed
- Fixed several text-input vs global-shortcut conflicts in editor/sidebar-related paths.
  修复编辑器与侧边栏多处“文本输入被全局快捷键抢占”的冲突。
- Fixed import/export format model inconsistency where TSV existed only as hidden CSV behavior.
  修复导入导出格式模型不一致的问题，TSV 不再只是隐藏在 CSV 行为里的别名。

## [3.3.1]

### Added
- Added dedicated Nix Flake installation guide with run/install/build/overlay usage.
  新增专门的 Nix Flake 安装文档，覆盖运行、安装、构建与 overlay 用法。
- Added edge regression test suite for autocomplete/session/welcome-onboarding boundaries.
  新增边缘回归测试套件，覆盖自动补全/会话状态/欢迎页引导状态机边界。

### Changed
- Extended flake outputs with standard `apps` and `overlay` entries for more reliable Nix integration.
  Flake 输出增强，补充标准 `apps` 与 `overlay`，提升 Nix 集成稳定性。
- Updated README and getting-started docs with explicit `nix profile install` path.
  README 与新手上手文档补充明确的 `nix profile install` 安装路径。

## [3.3.0]

### Changed
- Updated full dependency lock graph to the latest Rust 1.94.1-compatible versions via `cargo update`.
  使用 `cargo update` 将依赖锁文件整体升级到 Rust 1.94.1 可兼容的最新版本。
- Upgraded direct dependencies to latest compatible versions:
  `eframe/egui/egui_extras/rfd/rusqlite/russh/toml`.
  直接依赖升级到当前兼容最新版本：
  `eframe/egui/egui_extras/rfd/rusqlite/russh/toml`。
- Refactored app architecture by splitting request lifecycle, preferences/config, and metadata loading logic into dedicated modules.
  应用架构重构：将请求生命周期、偏好/配置、元数据加载逻辑拆分为独立模块。
- Slimmed `app/mod.rs` and delegated per-frame orchestration to render flow entry.
  精简 `app/mod.rs`，每帧编排流程下沉到渲染入口方法。
- Updated frame rendering entry to current `eframe` API (`App::ui` + `CentralPanel::show_inside`) and resolved related input/style API changes.
  渲染入口迁移到当前 `eframe` API（`App::ui` + `CentralPanel::show_inside`），并完成输入/样式相关 API 适配。

### Documentation
- Updated keyboard guide baseline to `v3.3.x`.
  键盘文档基线更新为 `v3.3.x`。
- Added platform distribution guide for AUR/Homebrew/nixpkgs.
  新增 AUR/Homebrew/nixpkgs 分发指南文档。
- Refreshed release/process docs and roadmap baseline to `v3.3.0`.
  发布流程与优化路线图基线同步更新至 `v3.3.0`。

## [3.2.1]

### Documentation
- Rebuilt the full documentation set into a bilingual, indexed structure.
  将全套文档重构为中英同页、可索引结构。
- Added beginner and user-operational docs:
  `GETTING_STARTED`, `FAQ`, `TROUBLESHOOTING`, `CONFIGURATION`.
  新增新手与用户操作文档：
  `GETTING_STARTED`、`FAQ`、`TROUBLESHOOTING`、`CONFIGURATION`。
- Added engineering/maintenance docs:
  `ARCHITECTURE`, `TESTING`, `SECURITY`, `RELEASE_PROCESS`,
  `ENVIRONMENT_VARIABLES`, `LEARNING_CURRICULUM`, `DOCS_STYLE`, `CONTRIBUTING`.
  新增工程与维护文档：
  `ARCHITECTURE`、`TESTING`、`SECURITY`、`RELEASE_PROCESS`、
  `ENVIRONMENT_VARIABLES`、`LEARNING_CURRICULUM`、`DOCS_STYLE`、`CONTRIBUTING`。
- Added automated markdown local-link validation:
  script `scripts/check_doc_links.py` and CI workflow `.github/workflows/docs.yml`.
  新增 Markdown 本地链接自动校验：
  脚本 `scripts/check_doc_links.py` 与 CI 工作流 `.github/workflows/docs.yml`。

## [3.2.0]

### Added
- Beginner onboarding loop on welcome/help flows.
  欢迎页与帮助页新增新手上手闭环。
- Structured learning guide split from tool usage guide.
  帮助系统拆分为工具指南与数据库知识学习指南。

### Changed
- SQL editor completion and focus behavior stabilized.
  SQL 编辑器补全与焦点行为稳定性提升。
- Default dark theme set to Tokyo Night Storm.
  默认深色主题为 Tokyo Night Storm。
- New connection dialog defaults to simple mode (advanced collapsed).
  新建连接默认简化模式（高级选项折叠）。

### Fixed
- `Tab` completion and focus transfer conflicts in SQL editor.
  修复 SQL 编辑器 `Tab` 补全与焦点转移冲突。
- Multiple welcome/help layout and alignment issues.
  修复欢迎页与帮助页多处布局对齐问题。
