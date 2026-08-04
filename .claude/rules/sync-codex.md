---
paths:
  - src/**/*.rs
  - Cargo.toml
  - tests/**/*.rs
  - docs/**/*.md
---

# After code changes: update `~/.codex/`

Every code change that affects Gridix architecture, conventions, APIs, or behavior must be reflected in the relevant Codex-local workflow files under `~/.codex/`.

## What to check

| change type | update this |
|---|---|
| Engineering workflow / delivery policy changed | `references/modern-software-engineering-workflow.md`, `workflow/README.md`, `references/workflow.md` |
| Rust quality gate / Cargo tooling changed | `references/rust-modern-engineering-playbook.md`, `rules/testing.md`, relevant skill docs |
| New module / moved file | `CLAUDE.md` module map, relevant `~/.codex/rules/` paths |
| New dialog / changed dialog shell | `references/dialog-audit.md`, `rules/ui-egui.md` |
| Workbench layout / panel model change | `references/workbench-ui-design.md`, `references/workbench-ui-refactor-spec.md`, `rules/ui-egui.md` |
| Project-wide refactor phase changed | `references/project-refactor-execution-plan.md`, `references/tech-debt.md`, `references/roadmap.md` |
| Changed keybinding / new shortcut | `~/.codex/skills/keybindings/SKILL.md`, `CLAUDE.md` key counts |
| New AppAction / command | `CLAUDE.md` variant counts, `~/.codex/skills/keybindings/SKILL.md` |
| Database driver / pool / query change | `rules/database.md`, `references/query-execution.md` |
| Session / connection lifecycle change | `rules/session.md`, `references/core-flows.md` |
| Data layer change | `rules/database.md` |
| Dock tab change / new panel type | `CLAUDE.md` dock section, `rules/ui-egui.md` dock rules |
| ER diagram change | `references/er-contracts.md` |
| New invariant / changed flow | `references/core-flows.md` |
| Bug fixed / new observation | `references/bug-ledger.md` |
| Tech debt found / resolved | `references/tech-debt.md` |
| Roadmap item completed | `references/roadmap.md` |
| Changed test pattern / new test file | `rules/testing.md` |
| Config field / env var changed | `CLAUDE.md` config/env sections, `~/.codex/skills/troubleshoot/SKILL.md` |
| Build dependency / version bump | `CLAUDE.md` quick commands, `~/.codex/skills/run-gridix/SKILL.md` |

## Layer awareness

When changing code, consider which layer it belongs to:

| Layer | Directory | Can depend on | Cannot depend on |
|-------|-----------|---------------|------------------|
| Types (`-1`) | `src/types.rs` | nothing | nothing (base layer) |
| Core (`0`) | `src/core/` | types | data, session, state, ui, egui |
| Data (`1`) | `src/data/` | types, core | session, state, ui, egui |
| Session (`2`) | `src/session/` | types, core, data | state, ui, egui |
| State (`3`) | `src/state/` | types, core, session (read-only) | ui, egui |
| UI (`4`) | `src/ui/` | all layers | nothing (top layer) |

If a new import violates this dependency direction, the change is architecturally wrong.

## Rule

After finishing a code change, re-read the relevant `~/.codex/` workflow files and update them in the same commit when they describe the changed behavior. The Codex-local workflow knowledge base should not drift from the code.
