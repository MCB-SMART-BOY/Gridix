# Gridix Optimization Roadmap | 优化路线图

> Baseline: `v3.7.0`  
> 基线版本：`v3.7.0`  
> Status labels: `已完成` / `进行中` / `待开始` / `暂缓`

## 1. Goal | 目标
- Keep keyboard-first productivity.
  保持键盘优先的高效工作流。
- Lower beginner onboarding cost.
  持续降低新手上手门槛。
- Improve release and package consistency.
  提升发布与多平台分发一致性。

## 2. Baseline Snapshot | 当前基线

### 2.1 Done in v3.3 | v3.3 已完成
- Welcome page onboarding loop and environment checks. `已完成`
- Help split: tool quick-start + DB learning guide. `已完成`
- SQL editor completion/focus stability (`Tab` chain fixed). `已完成`
- GitHub Release + AUR/Homebrew/Nix update flow aligned. `已完成`
- Core docs rewritten and indexed. `已完成`
- Direct dependencies upgraded to latest compatible versions (`egui/eframe/rusqlite/russh/rfd/toml`). `已完成`
- Frame render path migrated to current `eframe` app API (`App::ui` + `show_inside`). `已完成`

## 3. Priorities | 优先级

### P0: Correctness & Stability | 正确性与稳定性

#### P0-1 Focus-scoped input router
- Why: current global-first keyboard handling causes text-input theft and inconsistent local behavior.
  目的：当前全局优先键盘处理会造成文本输入被抢占，以及局部行为不一致。
- Deliverables:
  - Introduce `InputRouter` and scoped dispatch order.
  - Split sidebar/grid/editor/dialog routing by focus scope.
  - Keep only a minimal global shortcut set.
- Acceptance:
  - Typing in any text field never triggers non-text commands.
  - Focus movement can be described by one stable graph.
- Status: `进行中`

#### P0-2 External `keymap.toml` and scoped bindings
- Why: bindings must be editable after release and cannot stay hard-coded in app logic.
  目的：快捷键必须支持发布后修改，不能继续硬编码在应用逻辑中。
- Deliverables:
  - Add `~/.config/gridix/keymap.toml`.
  - Add default generation, merge, diagnostics, and conflict detection.
  - Move keybinding UI from flat action list to scope-oriented editor.
- Acceptance:
  - Missing keymap is auto-initialized.
  - Missing actions are auto-filled from defaults in memory.
- Status: `进行中`

#### P0-3 Sidebar workflow rebuild
- Why: current sidebar flow is list-centric, not workflow-centric.
  目的：当前侧边栏以列表为中心，而不是以工作流为中心。
- Deliverables:
  - Default layout: connections + filters only.
  - Add panel focus graph and optional edge-transfer behavior.
  - Promote filter panel to first-class keyboard workspace.
- Acceptance:
  - User can go from table list to filter editing without mouse.
  - Trigger/routine panels stay hidden unless needed.
- Status: `进行中`

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

#### P1-3 Unified import/export pipeline
- Deliverables:
  - Introduce shared transfer schema, preview, mapping, and execution plan.
  - Make CSV/TSV/JSON/SQL import/export use one option vocabulary.
  - Add dialect-aware SQL export.
- Acceptance:
  - Import and export share one staged workflow.
  - Preview and validation semantics remain consistent across formats.
- Status: `进行中`

### P2: Power User Efficiency | 进阶效率

#### P2-1 Command palette
- `Ctrl+P`, fuzzy action search, >= 20 high-frequency commands.
- 状态：`待开始`

#### P2-2 Editor focus/completion regression suite
- Define critical scenarios for completion accept, cursor relocation, focus transitions, and `F5` semantics.
- Must cover new scope-based routing after P0-1.
- 状态：`待开始`

#### P2-3 SQL editor capabilities
- Find/replace, line jump, comment toggle, snippets.
- Must not break scoped input routing.
- 状态：`待开始`

#### P2-4 ER keyboard navigation
- Node focus/move/arrange via keyboard.
- Complete basic ER browsing without mouse.
- 状态：`待开始`

## 4. Milestones | 里程碑

| Milestone | Timebox | Scope |
|---|---|---|
| `M1` | 1-2 weeks | P0-1 router skeleton + P0-2 keymap file loading |
| `M2` | 2-4 weeks | P0-3 sidebar workflow + filter keyboard workspace |
| `M3` | 4-6 weeks | P1-3 import/export unified MVP + P2-1 command palette |

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
