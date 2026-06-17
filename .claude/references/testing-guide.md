# Testing Guide

## Test locations

- External integration: `tests/*.rs` — `use gridix::*;`, no `#[cfg(test)]`
- Inline unit: `#[cfg(test)] mod tests { ... }` in source files
- Shared helpers: `tests/common/mod.rs` — `begin_key_pass()`, `focus_text_input()`

## Patterns by layer

### Core (Layer 0) — pure unit tests
```rust
#[test]
fn test_format_sql() {
    let formatted = format_sql("select * from users");
    assert!(formatted.contains("SELECT"));
}
```

### Data (Layer 1) — async integration tests
```rust
#[tokio::test]
async fn test_sqlite_connect() {
    let config = ConnectionConfig::default(); // in-memory SQLite
    let result = connect_database(&config).await;
    assert!(result.is_ok());
}
```

SQLite tests use `":memory:"` database — zero external dependencies.

### Session (Layer 2) — can test without egui
```rust
#[test]
fn test_poll_messages_empty() {
    let mut session = Session::new(runtime, tx, rx, history);
    // Test session logic directly
}
```

### UI (Layer 4) — egui Context tests
```rust
#[test]
fn test_widget() {
    let ctx = egui::Context::default();
    ctx.begin_pass(RawInput::default());
    // render widget, assert state
}
```

## Running

```bash
cargo test                              # all tests (~530)
cargo test -p gridix --lib              # unit tests only
cargo test --test core_tests            # core module tests
cargo test --test grid_tests            # grid-specific tests
```

## MySQL integration

MySQL tests require external server (marked `#[ignore]`):
```bash
GRIDIX_IT_MYSQL_HOST=127.0.0.1 GRIDIX_IT_MYSQL_PORT=3306 \
GRIDIX_IT_MYSQL_USER=root GRIDIX_IT_MYSQL_PASSWORD=secret \
GRIDIX_IT_MYSQL_DB=test \
cargo test --test mysql_cancel_integration -- --ignored --nocapture
```

## Known gaps

- No PostgreSQL/MySQL driver unit tests (connection, query, types, errors)
- No connection pool tests (`data/pool.rs`)
- No SSH tunnel establishment tests
- No benchmarks
- No property-based tests (proptest/quickcheck)
