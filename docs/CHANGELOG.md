# Changelog

All notable changes to this project are documented in this file.  
本文件记录项目的重要变更。

## [Unreleased]

- No unreleased changes yet.
  暂无未发布变更。

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
