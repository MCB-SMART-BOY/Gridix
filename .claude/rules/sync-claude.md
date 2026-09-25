---
paths:
  - src/**/*.rs
  - Cargo.toml
  - tests/**/*.rs
  - docs/**/*.md
---

# After code changes: update `.claude/`

Every code change that affects architecture, conventions, APIs, or behavior must be reflected in `.claude/`. `.claude/` is the project's in-repo engineering knowledge base and the only actively maintained harness copy; the historical `~/.codex/` mirror no longer exists, so there is nothing to sync from.

## Doc symbol gate

`cargo run --bin check-doc-symbols` validates that `.claude/**`, `docs/**`, `README.md`, and `CLAUDE.md` only reference project symbols that exist in `src/`/`tests/` and repo files that exist on disk. Run it after editing docs; CI runs it in `ci.yml` and `docs.yml`.

- Naming a removed symbol or a designed-but-never-implemented API is a failure — fix the reference, not the check.
- Deliberate external/removed references (an egui variant, a rejected trait, migration-era types) end their line with the per-line ignore marker documented in `src/bin/check_doc_symbols.rs`.
- Dated snapshots, design drafts, and general methodology docs carry the file-level ignore marker in an HTML comment line; the checker prints every skipped file so exemptions stay visible.
- Only backticked spans are checked, fenced code blocks and spans containing whitespace or parameter lists are skipped, and file references must be path-qualified and use a checked extension. `self.sql`-style field access, bare file names, globs, and brace lists are outside the check surface.

## What to check

| change type | update this |
|---|---|
| Engineering workflow / delivery policy changed | `references/modern-software-engineering-workflow.md`, `workflow/README.md`, `references/workflow.md` |
| Rust quality gate / Cargo tooling changed | `references/rust-modern-engineering-playbook.md`, `rules/testing.md`, relevant skill docs |
| Doc validation tooling changed | `rules/sync-claude.md`, `rules/testing.md`, `CLAUDE.md` quick commands, `skills/pr-prep/SKILL.md`, `.github/workflows/{ci,docs}.yml` |
| New module / moved file | `CLAUDE.md` module map, relevant `rules/` paths |
| New dialog / changed dialog shell | `references/dialog-audit.md`, `rules/ui-egui.md` |
| Workbench layout / panel model change | `references/workbench-ui-design.md`, `references/workbench-ui-refactor-spec.md`, `references/dockable-workbench-v2.md`, `references/gridix-ui-visual-system-v2.md`, `rules/ui-egui.md` |
| Project-wide refactor phase changed | `references/project-refactor-execution-plan.md`, `references/tech-debt.md`, `references/roadmap.md` |
| Changed keybinding / new shortcut | `skills/keybindings/SKILL.md`, `CLAUDE.md` key counts |
| New AppAction / command | `CLAUDE.md` variant counts, `skills/keybindings/SKILL.md` |
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
| Config field / env var changed | `CLAUDE.md` config/env sections, `skills/troubleshoot/SKILL.md` |
| Build dependency / version bump | `CLAUDE.md` quick commands, `skills/run-gridix/SKILL.md` |

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

After finishing a code change, re-read the relevant `.claude/` files and update them in the same commit. The `.claude/` directory IS the project's engineering knowledge base — don't let it drift from the code.
