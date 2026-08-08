---
paths:
  - tests/**/*.rs
  - src/**/mod.rs
---

# Testing rules with Gridix overlay

Use `~/.codex/references/modern-software-engineering-workflow.md` for the cross-project testing strategy and `~/.codex/references/rust-modern-engineering-playbook.md` for Rust-specific gates.

**Code is the source of truth.** Verify patterns against existing tests in `tests/` and `src/`. Update this file when test infrastructure changes.

## Universal policy

- Test the risk, not the implementation detail.
- Put pure logic tests at the lowest layer that can express the behavior.
- Add regression tests for fixed bugs when practical.
- Use characterization tests before high-risk refactors.
- Use measurements, not intuition, for optimization claims.
- Keep tests deterministic and independent of external services by default.

## Rust gates

Fast local loop:

```bash
cargo check
cargo test -p <package> <test_name>
```

Pre-merge / CI-quality validation:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo run --bin check-doc-links
cargo audit
```

Optional when installed/configured:

```bash
cargo nextest run
cargo llvm-cov nextest --workspace --all-features
cargo audit
cargo deny check
cargo fuzz run <target>
cargo bench
```

## Test locations

- External integration: `tests/*.rs` — use `use gridix::*;`, no `#[cfg(test)]` wrapper
- Inline unit: `#[cfg(test)] mod tests { use super::*; ... }` in source files
- Shared test utilities: `tests/common/mod.rs` provides `begin_key_pass()` and `focus_text_input()`

## Patterns

**Pure logic** (most common):
```rust
#[test]
fn test_something() {
    let result = some_pure_function(input);
    assert_eq!(result, expected);
}
```

**Session test** (new — no egui Context needed):
```rust
#[test]
fn test_session_execute() {
    let mut session = Session::new_with_test_runtime();
    session.connect("test".to_string());
    let effects = session.poll_messages();
    assert!(effects.connections.len() > 0);
}
```

**egui component** (for widget behavior — uses real egui Context, no GPU needed):
```rust
#[test]
fn test_widget() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::Enter,
            pressed: true,
            modifiers: egui::Modifiers::default(),
            repeat: false,
            physical_key: None,
        }],
        ..Default::default()
    });
    egui::Area::new("test".into()).show(&ctx, |ui| {
        widget.ui(ui);
    });
    assert!(widget.some_property);
}
```

**Async** (for DB operations):
```rust
#[tokio::test]
async fn test_query() {
    let result = execute_query(&config, "SELECT 1").await;
    assert!(result.is_ok());
}
```

## Rules

- Run the narrowest affected test while iterating, then `cargo test --workspace --all-features` before merge.
- If testing egui keyboard behavior, use `egui::Event::Key` with `Key::Character` or `Key::Named`.
- PostgreSQL/MySQL typed and cancellation integration tests read `GRIDIX_TEST_PG_URL` / `GRIDIX_TEST_MYSQL_URL`. Without a URL they may return locally; CI must preflight a non-empty URL and run them serially with `--nocapture --test-threads=1`.
- Server-side cancellation acceptance must observe a unique query marker, cancel it, observe `DbError::Cancelled`, confirm the marker disappears, then prove the connection remains usable. Do not replace this with fixed sleeps or task-abort assertions.
- Session tests should not require `egui::Context`.
- Data layer tests should not require `Session`.

## Layer-specific testing

| Layer | Test type | Example |
|-------|-----------|---------|
| `core/` | Pure unit tests | `#[test] fn test_format_sql()` |
| `data/` | Async integration (SQLite in-memory for single connection; temp file for multi-connection metadata) | `#[tokio::test] async fn test_connect()` |
| `session/` | Unit with mock runtime | `#[test] fn test_poll_messages()` |
| `state/` | Pure unit tests | `#[test] fn test_apply_effects()` |
| `ui/` | egui Context tests | `#[test] fn test_dialog_rendering()` |

## Release-acceptance boundary

- Backend Actions workflows—not an unconfigured local test run—are the PostgreSQL/MySQL acceptance gates for PRs, `main`, and `v*` tags.
- RA2 remains a manual SQLite GUI journey: it requires initial, saved, and reopened-result screenshots plus non-empty CSV/JSON/SQL exports. CSV must contain `after`, JSON `"name":"after"`, and SQL `'after'` with `NULL`; `gridix --ci-check` and driver screenshots alone do not prove this journey.
- Do not report a release or RA2 as accepted without the corresponding observed evidence.
