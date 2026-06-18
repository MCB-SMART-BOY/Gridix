# Modern Software Development Workflow

This workflow is a general default for software projects. Project-specific references and rules are overlays.

For the full general process, read `.claude/references/modern-software-engineering-workflow.md`.
For Rust projects, also read `.claude/references/rust-modern-engineering-playbook.md`.

## 8-Stage Lifecycle

```
Stage 0: INTAKE      → classify work type, scope, risk, success criteria         [01-plan.md]
Stage 1: DISCOVERY   → inspect code, reproduce, measure, find invariants         [01-plan.md]
Stage 2: DESIGN      → choose minimal approach, migration and test plan           [02-design.md]
Stage 3: SAFETY NET  → add/identify tests or measurements                        [03-implement.md]
Stage 4: IMPLEMENT   → small slices, compile/check frequently                    [03-implement.md]
Stage 5: REVIEW      → self-review correctness, architecture, docs, risk         [04-review.md]
Stage 6: VERIFY      → targeted and full quality gates                           [05-test.md]
Stage 7: DELIVER     → summarize, document, commit/release, update harness       [06-deliver.md]
```

## Quality Gates

Adapt to each project. Rust defaults:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Workspace Rust pre-merge:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

## Project Overlays

When a project has local rules, apply them after the general workflow.

Gridix overlays:

| When editing... | Rule loaded | Key constraint |
|----------------|-------------|----------------|
| `src/data/**` | `rules/database.md` | match db_type, no trait objects |
| `src/session/**` | `rules/session.md` | Async via Session, needs_repaint pattern |
| `src/ui/**`, `src/state/**` | `rules/ui-egui.md` | DialogId match arms, state field access |
| `src/**/mod.rs`, `tests/**` | `rules/testing.md` | data tests stay deterministic; use SQLite temp files for multi-connection metadata |
| Any source file | `rules/sync-codex.md` | Update `~/.codex/` workflow docs after changes |
| Any source file | `rules/sync-claude.md` | Update `.claude/` workflow docs after changes |

Gridix refactor phases are not complete until `~/.codex/memory` and `.claude/memory`, affected `references/`, `rules/`, and skills reflect the new state.

## Templates (in `templates/`)

- `commit-message.md` — Conventional commit format
- `feature-request.md` — Feature specification template
- `pr-description.md` — PR body template
