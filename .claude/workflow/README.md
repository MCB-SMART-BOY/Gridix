# Gridix Development Workflow

## 7-Stage Lifecycle

```
Stage 1: PLAN     → Understand scope, explore, design approach, get approval
Stage 2: DESIGN    → Architecture decisions, risk assessment, dependency map
Stage 3: IMPLEMENT → Code with layer awareness, incremental verification
Stage 4: REVIEW    → Self-review checklist, cross-layer check, stale ref scan
Stage 5: TEST      → Unit tests, integration tests, regression check
Stage 6: RELEASE   → Version bump, changelog, tag, publish
Stage 7: MONITOR   → Post-release verification, bug ledger update
```

## Quality Gates

Each stage has entry and exit criteria. Every commit passes:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Key Rules (auto-loaded by path matching)

| When editing... | Rule loaded | Key constraint |
|----------------|-------------|----------------|
| `src/data/**` | `rules/database.md` | match db_type, no trait objects |
| `src/session/**` | `rules/session.md` | Async via Session, needs_repaint pattern |
| `src/ui/**`, `src/state/**` | `rules/ui-egui.md` | DialogId match arms, state field access |
| `src/**/mod.rs`, `tests/**` | `rules/testing.md` | SQLite in-memory for data layer |
| Any source file | `rules/sync-claude.md` | Update .claude/ docs after changes |

## Templates (in `templates/`)

- `commit-message.md` — Conventional commit format
- `feature-request.md` — Feature specification template
- `pr-description.md` — PR body template
