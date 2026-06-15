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
- 56 source files have inline tests, 13 external test files

## Patterns

**Pure logic** (most common):
```rust
#[test]
fn test_something() {
    let result = some_pure_function(input);
    assert_eq!(result, expected);
}
```

**egui component** (for widget behavior — uses real egui Context, no GPU needed):
```rust
#[test]
fn test_widget() {
    let ctx = egui::Context::default();
    // Simulate keypress: inject events before begin_pass
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
    // Render widget inside an Area to give it a stable id
    egui::Area::new("test".into()).show(&ctx, |ui| {
        widget.ui(ui);
    });
    // Assert on widget state after rendering
    assert!(widget.some_property);
}
```

Reference: see `src/ui/dialogs/common.rs` and `src/app/surfaces/render.rs` inline tests for real examples.

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
- For input routing tests, reference the ~115 tests in `input_router.rs` as patterns
- MySQL integration tests must be `#[ignore]`d — they need external DB
- No shared test utilities exist yet — if you create one, put it in `tests/common/mod.rs`

## Known gaps

- No tests for PostgreSQL/MySQL driver implementations (connection, query, types, errors)
- No tests for connection pool (`database/pool.rs`)
- No tests for SSH tunnel establishment
- No tests for clipboard or file system operations
- No benchmarks
