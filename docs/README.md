# Gridix Docs Index | 文档索引

This folder keeps only current behavior docs, engineering references, and a small control set for recovery work.  
本目录只保留当前行为文档、工程参考文档，以及少量恢复控制文档。

Do not treat every recovery note as a daily entry document.  
不要把每一份 recovery 记录都当作日常入口文档。

## 1. Read First | 优先阅读

### User and runtime reference | 用户与运行时参考
- [GETTING_STARTED.md](GETTING_STARTED.md)
  First-run onboarding and the shortest usable path.
  首次上手与最短可用路径。
- [KEYBINDINGS.md](KEYBINDINGS.md)
  Current keyboard model, scoped shortcuts, and area-by-area behavior.
  当前键盘模型、作用域快捷键与分区域行为。
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
  Fast diagnosis for common runtime problems.
  常见运行时问题的快速排查。
- [CONFIGURATION.md](CONFIGURATION.md)
  Config file location, defaults, and reset guide.
  配置文件位置、默认值与重置方法。
- [FAQ.md](FAQ.md)
  Short answers to common user questions.
  高频问题的简明回答。
- [LEARNING_CURRICULUM.md](LEARNING_CURRICULUM.md)
  In-app learning roadmap and topic dependency policy.
  应用内学习路线与知识点依赖规范。

### Engineering reference | 工程参考
- [ARCHITECTURE.md](ARCHITECTURE.md)
  Current module boundaries, runtime model, and state ownership notes.
  当前模块边界、运行时模型与状态所有权说明。
- [KEYBOARD_FOCUS_RFC.md](KEYBOARD_FOCUS_RFC.md)
  Living input-model reference for focus scope, dialog ownership, and routing rules.
  焦点作用域、dialog owner 与输入路由规则的长期参考。
- [KEYMAP_TOML_SPEC.md](KEYMAP_TOML_SPEC.md)
  External keymap format and migration rules.
  外部 keymap 格式与迁移规则。
- [TESTING.md](TESTING.md)
  Local verification workflow and current high-risk areas.
  本地验证流程与当前高风险区域。
- [CHANGELOG.md](CHANGELOG.md)
  User-visible release history.
  面向用户的版本历史。
- [SECURITY.md](SECURITY.md)
  Security and operational recommendations.
  安全与操作建议。
- [ENVIRONMENT_VARIABLES.md](ENVIRONMENT_VARIABLES.md)
  Runtime and integration-test environment variables.
  运行时与集成测试环境变量。
- [RELEASE_PROCESS.md](RELEASE_PROCESS.md)
  Release checklist and packaging flow.
  发布检查表与打包流程。
- [DISTRIBUTION.md](DISTRIBUTION.md)
  AUR / Homebrew / nixpkgs distribution sync notes.
  AUR / Homebrew / nixpkgs 分发同步说明。
- [NIX_FLAKE.md](NIX_FLAKE.md)
  Nix Flake usage for run/build/install.
  Nix Flake 的运行、构建与安装说明。

## 2. Active Recovery Control Docs | 当前恢复控制文档

These are the current top-level recovery entry points.  
这些是当前恢复工作的顶层入口文档。

- [recovery/10-master-recovery-plan.md](recovery/10-master-recovery-plan.md)
  Workstreams, core flows, and staged recovery roadmap.
  工作流拆分、核心功能流与阶段性恢复路线图。
- [recovery/02-query-execution-trace.md](recovery/02-query-execution-trace.md)
  Long-term recovery ledger for the query execution and active-tab render path.
  查询执行与 active-tab 渲染链路的长期恢复账本。
- [recovery/11-core-flows-and-invariants.md](recovery/11-core-flows-and-invariants.md)
  Core feature list and system invariants that must not break.
  不能破坏的核心功能清单与系统不变量。
- [recovery/12-bug-ledger-4.1.0.md](recovery/12-bug-ledger-4.1.0.md)
  Current bug ledger grouped by root cause and priority.
  按根因和优先级整理的当前问题账本。
- [recovery/20-dialog-layout-audit.md](recovery/20-dialog-layout-audit.md)
  Current dialog/layout audit and remaining UI shell risks.
  当前 dialog/layout 审计与剩余 UI 壳层风险。
- [recovery/24-grid-save-context-isolation-fix.md](recovery/24-grid-save-context-isolation-fix.md)
  Long-term recovery ledger for grid edit/save isolation and `GridSaveDone` context handling.
  grid 编辑/保存隔离与 `GridSaveDone` 上下文处理的长期恢复账本。
- [recovery/43-dialog-responsive-row-design.md](recovery/43-dialog-responsive-row-design.md)
  Next-step design packet for the remaining dialog horizontal overflow root cause.
  剩余 dialog 横向失控根因的下一阶段设计包。
- [recovery/44-er-ownership-and-design-audit.md](recovery/44-er-ownership-and-design-audit.md)
  Next-step audit for ER ownership boundaries and design language.
  ER 权威状态边界与设计语言的下一阶段审计。
- [recovery/47-er-workspace-and-keyboard-contract.md](recovery/47-er-workspace-and-keyboard-contract.md)
  Pre-implementation contract for ER workspace role, state ownership, and keyboard flow.
  ER 在实现前的 workspace 角色、状态所有权与键盘流 contract。
- [recovery/48-er-visibility-entry-matrix-and-state-ledger.md](recovery/48-er-visibility-entry-matrix-and-state-ledger.md)
  Entry matrix and field ledger for ER visibility and internal state writes.
  ER 显隐入口与内部状态写入的入口矩阵和字段账本。
- [recovery/49-er-keyboard-flow-graph.md](recovery/49-er-keyboard-flow-graph.md)
  Keyboard-flow design for ER as a first-class workspace area.
  ER 作为正式工作区的键盘流图设计。
- [recovery/50-er-token-map.md](recovery/50-er-token-map.md)
  Token mapping plan for aligning ER visuals with the Gridix theme system.
  ER 视觉 token 与 Gridix 主主题系统对齐的映射方案。

## 3. Historical Recovery Notes | 历史恢复记录

Detailed historical notes should be merged into the long-term ledgers above once the fix has landed.  
更细的阶段性记录在修复落地后应继续合并回上面的长期账本，而不是长期并存。

Use them only when you are repairing that exact flow, state boundary, or dialog family.  
只有在修复对应功能流、状态边界或某类 dialog 时再进入这些记录。

## 4. Contribution Docs | 贡献文档

- [CONTRIBUTING.md](CONTRIBUTING.md)
  Development and pull request workflow.
  开发与 PR 流程。
- [DOCS_STYLE.md](DOCS_STYLE.md)
  Documentation writing and maintenance conventions.
  文档写作与维护规范。

## 5. Maintenance Rules | 维护约定

- Keep docs aligned with current default behavior before each release.
  每次发布前都要确保文档与当前默认行为一致。
- Use reference docs as the source of truth for current UX; use recovery docs for evidence and risk tracking.
  当前 UX 以参考文档为准；recovery 文档负责证据与风险跟踪。
- Delete obsolete plan docs once the implementation has landed and the fix note exists.
  当实现已落地且已有 fix note 时，删除对应的过时计划文档。
- Avoid stale wording such as “just completed” or “coming soon”.
  避免使用“刚完成”“即将上线”这类易过时措辞。
