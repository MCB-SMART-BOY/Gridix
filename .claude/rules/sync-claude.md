---
paths:
  - src/**/*.rs
  - Cargo.toml
  - tests/**/*.rs
  - docs/**/*.md
---

# After code changes: update `.claude/`

Every code change that affects architecture, conventions, APIs, or behavior must be reflected in `.claude/`.

## What to check

| change type | update this |
|---|---|
| New module / moved file | `CLAUDE.md` module map, relevant `rules/` paths |
| New dialog / changed dialog shell | `references/dialog-audit.md`, `rules/ui-egui.md` |
| Changed keybinding / new shortcut | `skills/keybindings/SKILL.md`, `CLAUDE.md` key counts |
| New AppAction / command | `CLAUDE.md` variant counts, `skills/keybindings/SKILL.md` |
| Database driver / pool / query change | `rules/database.md`, `references/query-execution.md` |
| Dock tab change / new panel type | `CLAUDE.md` dock section, `rules/ui-egui.md` dock rules |
| ER diagram change | `references/er-contracts.md` |
| New invariant / changed flow | `references/core-flows.md` |
| Bug fixed / new observation | `references/bug-ledger.md` |
| Tech debt found / resolved | `references/tech-debt.md` |
| Roadmap item completed | `references/roadmap.md` |
| Changed test pattern / new test file | `rules/testing.md` |
| Config field / env var changed | `CLAUDE.md` config/env sections, `skills/troubleshoot/SKILL.md` |
| Build dependency / version bump | `CLAUDE.md` quick commands, `skills/run-gridix/SKILL.md` |

## Rule

After finishing a code change, re-read the relevant `.claude/` files and update them in the same commit. The `.claude/` directory IS the project's engineering knowledge base — don't let it drift from the code.
