# Development Workflow

## 1. Plan

Use `references/modern-software-engineering-workflow.md` as the default process for all projects.
For Rust projects, also use `references/rust-modern-engineering-playbook.md`.
Apply project-specific rules after the general workflow.

For Gridix:
- Check `references/roadmap.md` for planned features.
- Check `references/tech-debt.md` for known issues.
- If architecture changes, read `references/architecture/decisions.md`.
- If project-wide refactor work, read `references/project-refactor-execution-plan.md`.
- After each completed Gridix refactor phase, update `~/.codex/memory` and `.claude/memory`, relevant `references/`, `rules/`, and skills before final delivery.

## 2. Code

Universal rules:
- Preserve behavior before changing structure.
- Separate refactoring from feature behavior unless explicitly scoped.
- Add tests or measurements before high-risk changes.
- Keep changes small and reviewable.

Gridix architecture overlay:
- Respect 6-layer dependency: `types ← core ← data ← session ← state ← ui`
- Layer 2 (session): `pub` fields OK (single-crate project)
- Layer 3 (state): fields accessed via `self.state.xxx`
- Use `match db_type` for backend dispatch — no trait needed

Gridix conventions:
- `use crate::prelude::*;` for common types
- `thiserror` for errors, `// =====...=====` section separators
- Chinese `//!` module docs, English identifiers
- Commit: `type(scope): description` format
- Workbench UI: new dockable surfaces must define descriptor metadata, stable IDs, placement rules, command/tooltip metadata, unified renderer routing, idempotent `ensure_surface_tab()` behavior, and tests before being wired into layout

## 3. Review

For general work, use the review checklist in `modern-software-engineering-workflow.md`.

For Gridix, run the `gridix-code-review` skill or manually check:
- Layer dependency direction
- Stale response guards on async handlers
- `needs_repaint` pattern used
- Config save uses `save_config_debounced()`
- Workbench surface bridge stays consistent: descriptors, `DockTab::surface_kind()`, `DockTab::Surface`, `ensure_surface_tab()`, `DbManagerApp::reveal_workbench_surface()`, `DbManagerApp::render_workbench_surface_in_ui()`, surface focus, docked-equivalent fallback de-duplication, and icon-only tooltip contract

## 4. Test

```bash
cargo test
cargo test -p <package> --lib
cargo test --test <integration_test>
```

For Gridix, MySQL integration tests need `GRIDIX_IT_MYSQL_*` env vars.

## 5. Pre-PR

Generic Rust pre-merge:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

For Gridix, run the `gridix-pr-prep` skill:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin check-doc-links
```

Also run `git diff --check` after documentation or patch-heavy changes.

## 6. Commit

- One logical change per commit
- Message format: `type(scope): description`
- Types: `feat`, `fix`, `refactor`, `docs`, `chore`, `test`
- Scopes: `session`, `state`, `data`, `ui`, `core`, `config`, `grid`, `editor`

## 7. Release

Run the `gridix-release` skill for version bump, changelog, tag, and publish.

## Quick Reference

| Task | Skill |
|------|-------|
| General engineering workflow | `modern-engineering-workflow` |
| Build & launch | `gridix-run` |
| Change shortcuts | `gridix-keybindings` |
| Pre-PR checks | `gridix-pr-prep` |
| Review code | `gridix-code-review` |
| Release | `gridix-release` |
| Fix errors | `gridix-troubleshoot` |
| Execute project refactor | `references/project-refactor-execution-plan.md` |
