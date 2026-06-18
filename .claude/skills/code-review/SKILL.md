---
name: gridix-code-review
description: Review Gridix code changes for correctness, style, and architectural consistency. Use only in the Gridix repository before committing or when asked to review a Gridix PR.
paths:
  - src/**/*.rs
  - tests/**/*.rs
  - Cargo.toml
---

# Code Review Checklist

Run each check in order. Stop on first failure.

## 1. Architecture

- [ ] Layer dependency: does the change respect `types ← core ← data ← session ← state ← ui`?
- [ ] No new cross-layer imports (core→data is the only documented exception)
- [ ] Session fields accessed through `self.session.xxx`, State through `self.state.xxx`
- [ ] No `self.sql` field usage — use `active_sql()`/`set_active_sql()`
- [ ] Workbench UI changes preserve the compatibility bridge: legacy `FocusArea`, sidebar visibility, and sidebar width must stay synchronized with `UiState.workbench` until old paths are removed
- [ ] Toolbar remains global: do not render `Toolbar::show_with_focus()` from `SqlDocument` or other EditorArea document/view tabs
- [ ] EditorArea tabs keep document/view semantics; do not reintroduce the old query-output tab plus standalone SQL scratchpad split
- [ ] Activity switching goes through `AppAction::SetWorkbenchActivity`; widgets should not mutate `WorkbenchState.active_activity` directly
- [ ] BottomPanel UI/command changes go through `AppAction::ToggleBottomPanel`, `SetBottomPanelVisible`, or `SetBottomPanelTab`
- [ ] RightInspector UI/command changes go through `AppAction::ToggleRightInspector`, `SetRightInspectorVisible`, or `SetRightInspectorTab`; never render password/secret fields in inspector content
- [ ] Query results/errors route to BottomPanel Results/Messages; do not reintroduce error/result rendering as a replacement for editor content
- [ ] Schema/ER/cell detail flows should use RightInspector rather than blocking dialogs or replacing EditorArea content

## 2. Correctness

- [ ] Async messages carry `request_id` for stale response guards
- [ ] `clear_result()`/`clear_search()` used for mirror + tab sync
- [ ] `save_config_debounced()` used (not direct `save_config()`) for non-preference changes
- [ ] `needs_repaint = true` set in handlers (not `ctx.request_repaint()`)

## 3. Style

- [ ] Follows existing patterns (emoji icons, theme colors, `icon_button` in toolbar)
- [ ] Chinese module docs (`//!`) + English identifiers
- [ ] `// =====...=====` section separators
- [ ] Commits: `type(scope): description` format

## 4. Tests

- [ ] `cargo test` passes
- [ ] New functionality has at least one test
- [ ] Data layer changes include deterministic SQLite tests where applicable; use a temporary database file when the path opens more than one SQLite connection

## 5. Docs

- [ ] `CLAUDE.md` updated if architecture changes
- [ ] Relevant `~/.codex/rules/` file updated
- [ ] Roadmap updated if feature complete
- [ ] Workbench phase changes update `.claude/references/project-refactor-execution-plan.md`, `.claude/references/workbench-ui-refactor-spec.md`, `.claude/references/roadmap.md`, and `.claude/references/tech-debt.md`

## Quick checks

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin check-doc-links
```
