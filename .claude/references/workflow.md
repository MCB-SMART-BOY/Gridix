# Gridix Development Workflow

## 1. Plan

Read the task navigation table in `CLAUDE.md` to find the right starting point.
Check `references/roadmap.md` for planned features.
Check `references/tech-debt.md` for known issues.
If architecture change, read `references/architecture/decisions.md`.

## 2. Code

**Architecture rules:**
- Respect 6-layer dependency: `types ← core ← data ← session ← state ← ui`
- Layer 2 (session): `pub` fields OK (single-crate project)
- Layer 3 (state): fields accessed via `self.state.xxx`
- Use `match db_type` for backend dispatch — no trait needed

**Code conventions:**
- `use crate::prelude::*;` for common types
- `thiserror` for errors, `// =====...=====` section separators
- Chinese `//!` module docs, English identifiers
- Commit: `type(scope): description` format

## 3. Review

Run `/code-review` skill or manually check:
- Layer dependency direction
- Stale response guards on async handlers
- `needs_repaint` pattern used
- Config save uses `save_config_debounced()`

## 4. Test

```bash
cargo test                                    # All tests
cargo test -p gridix --lib                    # Unit tests only
cargo test --test core_tests                  # Core module tests
```

MySQL integration tests need `GRIDIX_IT_MYSQL_*` env vars.

## 5. Pre-PR

Run `/pr-prep` skill:
```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin check-doc-links
```

## 6. Commit

- One logical change per commit
- Message format: `type(scope): description`
- Types: `feat`, `fix`, `refactor`, `docs`, `chore`, `test`
- Scopes: `session`, `state`, `data`, `ui`, `core`, `config`, `grid`, `editor`

## 7. Release

Run `/release` skill for version bump, changelog, tag, and publish.

## Quick Reference

| Task | Skill |
|------|-------|
| Build & launch | `/run-gridix` |
| Change shortcuts | `/keybindings` |
| Pre-PR checks | `/pr-prep` |
| Review code | `/code-review` |
| Release | `/release` |
| Fix errors | `/troubleshoot` |
