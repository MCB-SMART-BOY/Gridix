# Gridix Development Workflow

## 6-Stage Lifecycle

```
Stage 1: PLAN      → Scope, non-goals, risks, observable success criteria
Stage 2: DESIGN    → Impact and trade-off analysis when warranted
Stage 3: IMPLEMENT → Minimal coherent change and targeted checks
Stage 4: REVIEW    → Correctness, security, stale-callsite and documentation review
Stage 5: TEST      → Behavior proof, full validation, release-acceptance evidence
Stage 6: DELIVER   → Summarize evidence and limitations; commit or publish only when authorized
```

## Quality Gates

For a complete project validation, run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo run --bin check-doc-links
cargo audit
```

CI also makes PostgreSQL and MySQL typed/cancellation workflows release-acceptance gates on PRs, `main`, and `v*` tags. Their configured URLs must pass preflight; a local test's no-URL return is not evidence.

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
