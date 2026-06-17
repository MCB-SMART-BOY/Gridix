---
name: code-review
description: Review code changes for correctness, style, and architectural consistency. Use before committing or when asked to review a PR.
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
- [ ] Data layer changes include SQLite in-memory test where applicable

## 5. Docs

- [ ] CLAUDE.md updated if architecture changes
- [ ] Relevant `.claude/rules/` file updated
- [ ] Roadmap updated if feature complete

## Quick checks

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin check-doc-links
```
