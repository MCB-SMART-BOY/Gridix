# Changelog

All notable changes to this project are documented in this file.  
本文件记录项目的重要变更。

## [Unreleased]

### Documentation
- Reserved for next release updates.
  预留给下一个版本的更新内容。

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
