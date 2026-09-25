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

GRIDIX_ACCEPTANCE=1 \
GRIDIX_TEST_MYSQL_URL='mysql://user:password@127.0.0.1:3306/database' \
cargo test --test mysql_pool_acceptance -- --nocapture --test-threads=1

GRIDIX_ACCEPTANCE=1 \
GRIDIX_TEST_PG_URL='postgres://user:password@127.0.0.1:5432/database' \
cargo test --test postgres_pool_acceptance -- --nocapture --test-threads=1
```

The pool acceptance suites are split per backend so a workflow only needs the URL it provides: `mysql_pool_acceptance` and `postgres_pool_acceptance`. With `GRIDIX_ACCEPTANCE=1` a missing URL is a hard failure; without the flag the suites skip with a printed reason. `tls_acceptance` builds its own certificate fixtures inside `tls-acceptance.yml` (pull requests, `main`, tags, weekly schedule, and manual dispatch); `ssh_acceptance` runs only through the manually dispatched `ssh-acceptance.yml`, which needs repository SSH secrets.

The test binaries return early when their URL is absent for local convenience. That is not release evidence: the PostgreSQL and MySQL Actions workflows preflight the corresponding URL, run these commands serially, and provide their service environments. The MySQL cancellation observer additionally needs `PROCESS` and `CONNECTION_ADMIN`.

## Release-acceptance evidence

PostgreSQL/MySQL Actions runs are configured release-acceptance gates for pull requests, `main`, and `v*` tags; configuration alone does not assert that a release has been published or accepted. The remaining GUI acceptance gap is a manual SQLite journey: create/query/edit/save/reopen/export a SQLite database and retain its screenshots and exported CSV, JSON, and SQL artifacts. `gridix-driver` covers the driven part of that journey — `launch`, `key`, `type`, `click`, `ss`, `assert-file`/`assert-export`/`assert-reopened`, `quit` — and runs on its own WM-less Xvfb (it falls back to `windowfocus` when `_NET_ACTIVE_WINDOW` is unavailable, as on this workstation). It cannot operate native file dialogs, so the export evidence still needs an operator in a desktop session.

## Known gaps

- The MySQL cancellation workflow covers direct `mysql:8.4` plus observer/KILL privileges, not TLS or SSH tunnel scenarios.
- Execution-pool capacity pressure is covered by `mysql_pool_acceptance` / `postgres_pool_acceptance` (more concurrent queries than `MYSQL_POOL_MAX_CONNECTIONS`, pool/client recreation after `remove_pool` and `clear_all`) against `mysql:8.4` and `postgres:16`, but not over TLS or an SSH tunnel.
- Cancellation returns the pooled connection to the pool (`data/query/mysql.rs` `execute_typed_cancellable`), so the exact cancelled connection can be reused; asserting that specific identity needs pool internals that are not exposed, so the post-cancellation `SELECT 1` remains a liveness check.
- No benchmarks or property-based tests (proptest/quickcheck).
