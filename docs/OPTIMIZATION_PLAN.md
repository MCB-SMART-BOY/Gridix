# Gridix Optimization Roadmap | 优化路线图

> Baseline: `v3.2.x`  
> 基线版本：`v3.2.x`  
> Status labels: `已完成` / `进行中` / `待开始` / `暂缓`

## 1. Goal | 目标
- Keep keyboard-first productivity.
  保持键盘优先的高效工作流。
- Lower beginner onboarding cost.
  持续降低新手上手门槛。
- Improve release and package consistency.
  提升发布与多平台分发一致性。

## 2. Baseline Snapshot | 当前基线

### 2.1 Done in v3.2 | v3.2 已完成
- Welcome page onboarding loop and environment checks. `已完成`
- Help split: tool quick-start + DB learning guide. `已完成`
- SQL editor completion/focus stability (`Tab` chain fixed). `已完成`
- GitHub Release + AUR/Homebrew/Nix update flow aligned. `已完成`
- Core docs rewritten and indexed. `已完成`

## 3. Priorities | 优先级

### P0: Correctness & Stability | 正确性与稳定性

#### P0-1 Editor focus/completion regression suite
- Why: avoid regressions in `Tab` completion and cursor/focus behavior.
  目的：防止 `Tab` 补全、光标与焦点行为回归。
- Deliverables:
  - Define critical scenarios (manual + CI checklist).
  - Cover: completion accept, cursor relocation, tab switch draft sync, `F5` semantics.
- Acceptance:
  - >= 8 critical paths pass.
  - Same-class regressions stay at zero for 3 releases.
- Status: `待开始`

#### P0-2 Unified query status/error surface
- Why: execution state must be consistent across toolbar/status/notifications.
  目的：执行状态在工具栏、状态栏、通知里保持一致。
- Deliverables:
  - Unified state copy: running/success/failure/canceled.
  - Error format: DB raw message + user-friendly hint.
- Acceptance:
  - SQLite/PostgreSQL/MySQL errors each can be located by SQL and stage.
- Status: `待开始`

#### P0-3 Release verification automation
- Why: prevent package version/hash drift after release.
  目的：避免发布后各平台版本/哈希不一致。
- Deliverables:
  - Add post-release verification script for artifacts + templates.
  - Release completion depends on script pass.
- Acceptance:
  - One command outputs clear diff report.
- Status: `待开始`

### P1: Beginner End-to-End Path | 新手闭环

#### P1-1 Detection -> install/init -> connect -> first query
- Deliverables:
  - Welcome-page guidance for PostgreSQL/MySQL install and init.
  - Connection dialog presets for beginner templates.
  - Step guidance for create DB/create user/first query.
- Acceptance:
  - New users can run first query without leaving app docs/help flow.
- Status: `进行中`

#### P1-2 Learning topic to in-app action linkage
- Deliverables:
  - For each topic: concept -> manual steps -> one-click demo.
  - Keep dependency and next-step hints explicit.
- Acceptance:
  - At least 6 core topics form one coherent path.
- Status: `进行中`

#### P1-3 Beginner/advanced information layering
- Deliverables:
  - Keep connection dialog simple by default.
  - Show advanced options on demand with terminology hints.
- Acceptance:
  - First-screen required fields <= 8.
- Status: `待开始`

### P2: Power User Efficiency | 进阶效率

#### P2-1 Command palette
- `Ctrl+P`, fuzzy action search, >= 20 high-frequency commands.
- 状态：`待开始`

#### P2-2 SQL editor capabilities
- Find/replace, line jump, comment toggle, snippets.
- Must not break existing completion/focus flow.
- 状态：`待开始`

#### P2-3 ER keyboard navigation
- Node focus/move/arrange via keyboard.
- Complete basic ER browsing without mouse.
- 状态：`待开始`

## 4. Milestones | 里程碑

| Milestone | Timebox | Scope |
|---|---|---|
| `M1` | 1-2 weeks | P0-1 + P0-2 first implementation |
| `M2` | 2-4 weeks | P1-1 usable loop + P1-2 core topics |
| `M3` | 4-6 weeks | P2-1 MVP + P2-2 MVP |

## 5. Metrics | 验收指标
- Focus/completion regression issues trend down.
  焦点与补全回归问题持续下降。
- Beginner first-connection and first-query success trend up.
  新手首次连接和首次查询成功率持续上升。
- Release artifact/package mismatch count stays zero.
  发布产物与包管理器版本不一致问题保持为零。
- Docs drift bugs stay zero.
  文档与实际行为偏差问题保持为零。

## 6. Non-goals (Current Stage) | 当前非目标
- No large AI module expansion before P0/P1 closure.
  在 P0/P1 完成前不扩展大型 AI 模块。
- No heavy BI/reporting system as near-term priority.
  近期不优先做重量级 BI/报表系统。

## 7. Maintenance Rules | 维护规则
- Update this roadmap before each minor release.
  每次小版本发布前更新本路线图。
- Add acceptance notes/risk notes after each completed item.
  每完成一个任务都补充验收与风险备注。
- Freeze docs before release publication.
  发布前先冻结文档，避免发布后补文档。
