---
paths:
  - src/data/**/*.rs
  - src/session/database.rs
  - src/session/handler.rs
---

# Gridix database rules

**Code is the source of truth.** Verify claims against `src/data/` before relying on them. Update this file when you change database code.

## Architecture

Three backends with divergent patterns:
- **SQLite**: synchronous via `rusqlite`. Wrapped in `task::spawn_blocking()`.
- **PostgreSQL**: async via `tokio-postgres`. Single `Arc<Client>` per connection.
- **MySQL**: async via `mysql_async`. Pool-based with idle TTL + health checks.

Orchestrator: `data/query/mod.rs` dispatches via `match db_type`. **No trait** — `match db_type` is the correct pattern for three backends with fundamentally different execution models. A previous `DatabaseDriver` trait was deleted as dead code.

## Connection lifecycle

1. `Session::connect()` in `session/database.rs` → spawns async task with timeout
2. `data::connect_database()` in `data/query/mod.rs` → SSH tunnel setup → backend-specific connect
3. Result via `Message::ConnectedWithTables/Databases` on mpsc channel
4. `Session::poll_messages()` dispatches to handler → validates request_id → updates session state → emits `FrameEffects`

## Typed execution and cancellation

- Public typed entry points are `execute_typed(config, sql)` and `execute_typed_cancellable(config, sql, cancellation)`.
- **SQLite** runs synchronously in `spawn_blocking` and has no supported in-flight cancellation contract.
- **PostgreSQL** uses the executing client's `CancelToken`; cancellation sends `CancelRequest` and then awaits the original query future, mapping the server cancellation to `DbError::Cancelled`.
- **MySQL** records the execution `Conn::id()` and opens a separate TLS-configured control `Conn` to issue `KILL QUERY <connection_id>`; never derive the ID from SQL or user input.
- Cancellation is cooperative for runtime query tasks. It is not a substitute for aborting unrelated task kinds.

## Typed value boundaries

- PostgreSQL `NUMERIC` parameters and decoded results preserve exact `DbValue::Decimal` values.
- MySQL temporal input rejects nanoseconds greater than or equal to one second; validate before dispatch rather than silently truncating.

## Release-acceptance backend gates

PostgreSQL and MySQL typed and cancellation integration workflows run on pull requests, `main`, and `v*` tags. Their `GRIDIX_TEST_PG_URL` / `GRIDIX_TEST_MYSQL_URL` preflight is mandatory in CI: a local no-URL return is convenience only, never release evidence.

## Pooling

`data/pool.rs` — manual pooling, NOT a generic pool crate:
- MySQL: `HashMap<String, (Pool, Instant)>` — idle timeout, LRU eviction, health-check
- PostgreSQL: `HashMap<String, (Arc<Client>, Instant)>` — `client.is_closed()` health check
- SQLite: not pooled ("doesn't need it")

## QueryResult null handling

`QueryResult.null_flags: Vec<Vec<bool>>` is a **parallel array** to `rows`.
`null_flags[row][col] == true` means the value is SQL NULL (the corresponding string is empty).
This is deliberate — avoids sentinel values for distinguishing NULL from empty string.

## Grid save (transactional batch)

Grid cell edits/inserts/deletes are saved as ONE atomic transaction, not N independent queries:
- `DbManagerApp::execute_grid_save(table, statements)` → `execute_import_batch(&config, statements, use_transaction=true, stop_on_error=true)`.
- Result via `Message::GridSaveDone { result, table, request_id, elapsed_ms }` → `handle_grid_save_done`.
- Committed (failed == 0) → `clear_edits()` + `RefreshSelectedTable`; rolled back → keep edits + error.
- `db_type` MUST be threaded into `generate_save_sql` for correct identifier quoting (MySQL backticks, PG/SQLite double-quotes). Do NOT pass `db_type=None` from the grid UI layer.
- Do NOT revert to looping `execute()` per statement — that reintroduces partial-commit (audit B2) and post-save stale edits (audit B1).

## Password security

- `ConnectionConfig.password` is `#[serde(skip_serializing)]`
- `password_ref` (UUID) stored in config.toml, actual secret in OS keyring via `keyring` crate
- Legacy AES-256-GCM encrypted passwords auto-migrated to keyring on load (retain migration path)
- `pool_key()` uses SHA-256 of full connection params (including password) for unique pool identity

## SSH tunnel

`data/ssh_tunnel.rs`:
- `SshTunnelManager` singleton via `std::sync::LazyLock`
- Tunnels cached by name with `get_or_create`/`stop`
- `russh` + `known_hosts` verification with SHA-256 fingerprint logging
- Config rewritten to `127.0.0.1:<dynamic_port>` before connecting
- `pool_route_key_material()` includes tunnel routing so pool keys remain stable after rewrite
- `SshError::HostKeyVerification` — distinct error variant for known_hosts mismatch vs. missing known_hosts
- SSH passwords and private key passphrases are `#[serde(skip_serializing)]`

## Error handling

`DbError` (thiserror, 2 active variants: Connection, Query). All errors use `#[error("...")]` for Display formatting.
SSL/TLS: PG default Prefer, MySQL default Preferred. Required modes validate certificates.
