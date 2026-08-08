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
cargo test --workspace --all-features
cargo test -p gridix --lib
cargo test --test core_tests
cargo test --test grid_tests
cargo doc --workspace --no-deps
```

## PostgreSQL and MySQL typed integration

The typed execution, mutation, catalog, and cancellation suites require real database services. They use a single URL environment variable per backend:

```bash
GRIDIX_TEST_PG_URL='postgres://user:password@127.0.0.1:5432/database' \
cargo test --test postgres_typed_e2e -- --nocapture --test-threads=1

GRIDIX_TEST_PG_URL='postgres://user:password@127.0.0.1:5432/database' \
cargo test --test postgres_cancel_integration -- --nocapture --test-threads=1

GRIDIX_TEST_MYSQL_URL='mysql://user:password@127.0.0.1:3306/database' \
cargo test --test mysql_typed_e2e -- --nocapture --test-threads=1

GRIDIX_TEST_MYSQL_URL='mysql://user:password@127.0.0.1:3306/database' \
cargo test --test mysql_cancel_integration -- --nocapture --test-threads=1
```

The test binaries return early when their URL is absent for local convenience. That is not release evidence: the PostgreSQL and MySQL Actions workflows preflight the corresponding URL, run these commands serially, and provide their service environments. The MySQL cancellation observer additionally needs `PROCESS` and `CONNECTION_ADMIN`.

## Release-acceptance evidence

PostgreSQL/MySQL Actions runs are configured release-acceptance gates for pull requests, `main`, and `v*` tags; configuration alone does not assert that a release has been published or accepted. The remaining GUI acceptance gap is a manual SQLite journey: create/query/edit/save/reopen/export a SQLite database and retain its screenshots and exported CSV, JSON, and SQL artifacts. `gridix --ci-check` and `gridix-driver` cannot replace it because the driver supports only `launch`, `key`, `ss`, `quit`, and `help`, not typed input or dialog waits.

## Known gaps

- The MySQL cancellation workflow covers direct `mysql:8.4` plus observer/KILL privileges, not TLS, SSH tunnel, or execution-pool-capacity scenarios.
- The post-cancellation `SELECT 1` checks pool usability, not reuse of the exact cancelled MySQL connection.
- No benchmarks or property-based tests (proptest/quickcheck).
