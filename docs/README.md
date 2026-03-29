# Gridix Docs Index | 文档索引

This folder contains user-facing and engineering docs for Gridix.  
本目录包含 Gridix 的用户文档与工程文档。

## Documents | 文档列表

### 1) User docs | 用户文档
- [GETTING_STARTED.md](GETTING_STARTED.md)
  First-time onboarding path for beginners.
  面向新手的首次上手路径。
- [FAQ.md](FAQ.md)
  Frequent user questions and practical answers.
  高频问题与实用回答。
- [KEYBINDINGS.md](KEYBINDINGS.md)
  Keyboard model, default shortcuts, area-by-area usage.
  键盘模型、默认快捷键、分区域操作说明。
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
  Common issues and fast diagnosis steps.
  常见问题与快速排查步骤。
- [CONFIGURATION.md](CONFIGURATION.md)
  Config file location, fields, defaults, and reset guide.
  配置文件位置、字段、默认值与重置方法。
- [LEARNING_CURRICULUM.md](LEARNING_CURRICULUM.md)
  Structured database learning roadmap and topic dependency policy.
  数据库学习路线与知识点依赖规范。

### 2) Engineering docs | 工程文档
- [OPTIMIZATION_PLAN.md](OPTIMIZATION_PLAN.md)
  Product/engineering optimization roadmap and acceptance targets.
  产品与工程优化路线图、优先级与验收目标。
- [CHANGELOG.md](CHANGELOG.md)
  User-facing release changes and highlights.
  面向用户的版本变更与亮点记录。
- [ARCHITECTURE.md](ARCHITECTURE.md)
  Runtime architecture and module boundaries.
  运行时架构与模块边界说明。
- [TESTING.md](TESTING.md)
  Local/CI testing workflow and regression focus areas.
  本地与 CI 测试流程及回归重点。
- [SECURITY.md](SECURITY.md)
  Security design and operational recommendations.
  安全设计与操作建议。
- [RELEASE_PROCESS.md](RELEASE_PROCESS.md)
  Release trigger, build matrix, and verification checklist.
  发布触发、构建矩阵与校验清单。
- [DISTRIBUTION.md](DISTRIBUTION.md)
  AUR/Homebrew/nixpkgs synchronization workflow.
  AUR/Homebrew/nixpkgs 分发同步流程。
- [NIX_FLAKE.md](NIX_FLAKE.md)
  Nix Flake based run/install/build/overlay usage.
  Nix Flake 运行、安装、构建与 overlay 使用方式。
- [ENVIRONMENT_VARIABLES.md](ENVIRONMENT_VARIABLES.md)
  Runtime and integration-test environment variable reference.
  运行与集成测试环境变量说明。

### 3) Contribution docs | 贡献文档
- [CONTRIBUTING.md](CONTRIBUTING.md)
  Development and pull request workflow.
  开发与 PR 提交流程。
- [DOCS_STYLE.md](DOCS_STYLE.md)
  Documentation writing and maintenance conventions.
  文档编写与维护规范。

## Maintenance Rules | 维护约定
- Keep docs aligned with current default behavior before each release.
  每次发布前必须确保文档与当前默认行为一致。
- Avoid stale status wording like "just finished yesterday".
  避免使用易过时的描述（如“刚刚完成”）。
- For plan/status docs, use fixed labels: `已完成` / `进行中` / `待开始` / `暂缓`.
  计划类文档统一状态标签：`已完成` / `进行中` / `待开始` / `暂缓`。
