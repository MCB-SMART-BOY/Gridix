---
paths:
  - tests/**/*.rs
  - src/**/mod.rs
---

# Gridix testing rules

**Code is the source of truth.** Verify patterns against existing tests in `tests/` and `src/`. Update this file when test infrastructure changes.

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

- Always run `cargo test` after writing tests — don't assume they pass
- If testing egui keyboard behavior, use `egui::Event::Key` with `Key::Character` or `Key::Named`
- MySQL integration tests must be `#[ignore]`d — they need external DB
- Session tests should not require `egui::Context`
- Data layer tests should not require `Session`

## Layer-specific testing

| Layer | Test type | Example |
|-------|-----------|---------|
| `core/` | Pure unit tests | `#[test] fn test_format_sql()` |
| `data/` | Async integration (SQLite in-memory) | `#[tokio::test] async fn test_connect()` |
| `session/` | Unit with mock runtime | `#[test] fn test_poll_messages()` |
| `state/` | Pure unit tests | `#[test] fn test_apply_effects()` |
| `ui/` | egui Context tests | `#[test] fn test_dialog_rendering()` |

## Known gaps

- No tests for PostgreSQL/MySQL driver implementations (connection, query, types, errors)
- No tests for connection pool (`data/pool.rs`)
- No tests for SSH tunnel establishment
- No benchmarks
