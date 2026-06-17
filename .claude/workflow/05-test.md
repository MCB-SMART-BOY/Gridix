# Stage 5: Test

## Entry Criteria
- [ ] Stage 4 review passed
- [ ] All build checks pass

## Test Strategy by Layer

| Layer | Test type | Location | Dependency |
|-------|-----------|----------|------------|
| core | `#[test]` pure logic | `tests/core_tests.rs` + inline | None |
| data | `#[tokio::test]` SQLite in-memory | inline in `sqlite.rs` | None (in-memory) |
| session | `#[test]` with mock data | inline | No egui Context |
| state | `#[test]` state transitions | inline | No egui Context |
| ui | `#[test]` egui Context::default() | inline + `tests/ui_dialogs_tests.rs` | egui |

## Required Tests

### For data layer changes
```rust
#[test]
fn test_sqlite_new_feature() {
    let config = ConnectionConfig {
        db_type: DatabaseType::SQLite,
        database: ":memory:".to_string(),
        ..Default::default()
    };
    // Test the new functionality
}
```

### For UI changes
```rust
#[test]
fn test_new_widget() {
    let ctx = egui::Context::default();
    ctx.begin_pass(RawInput::default());
    // Render and assert
}
```

## Running

```bash
cargo test                              # All tests
cargo test -p gridix --lib              # Unit only
cargo test --test core_tests            # Specific suite
cargo test data::query::sqlite::tests   # Specific module
```

## Exit Criteria
- [ ] `cargo test` passes (0 failures)
- [ ] New functionality has at least one test
- [ ] No existing tests broken
- [ ] MySQL integration tests (if applicable) can be `#[ignore]`d
