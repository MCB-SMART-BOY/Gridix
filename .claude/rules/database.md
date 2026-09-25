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

Orchestrator: `data/query/mod.rs` dispatches via `match db_type`. **No trait** — `match db_type` is the correct pattern for three backends with fundamentally different execution models. A previous `DatabaseDriver` trait was deleted as dead code. <!-- doc-symbols: ignore: deliberately names the removed trait -->

## Connection lifecycle

1. `DbManagerApp::connect()` in `app/runtime/database.rs` → spawns async task with timeout
2. `data::connect_database()` in `data/query/mod.rs` → SSH tunnel setup → backend-specific connect
3. Result via `Message::RuntimeEvent` (`RuntimeOutcome::Connected`) on the mpsc channel, stale-guarded by `TaskRegistry`
4. `handle_messages()` (`app/runtime/handler.rs`) dispatches on the UI thread → validates task identity → updates session state → emits `FrameEffects` (the planned Session::poll_messages split described in `session/mod.rs` is still pending)

## Typed execution and cancellation

- Public typed entry points are `execute_typed(config, sql)` and `execute_typed_cancellable(config, sql, cancellation)`.
- **SQLite** runs synchronously in `spawn_blocking` and has no supported in-flight cancellation contract.
- **PostgreSQL** uses the executing client's `CancelToken`; cancellation sends `CancelRequest` and then awaits the original query future, mapping the server cancellation to `DbError::Cancelled`. <!-- doc-symbols: ignore: PostgreSQL wire-protocol message name -->
- **MySQL** records the execution `Conn::id()` and opens a separate TLS-configured control `Conn` to issue `KILL QUERY <connection_id>`; never derive the ID from SQL or user input.
- Cancellation is cooperative for runtime query tasks. It is not a substitute for aborting unrelated task kinds.

## Typed value boundaries

- PostgreSQL `NUMERIC` parameters and decoded results preserve exact `DbValue::Decimal` values.
- MySQL temporal input rejects nanoseconds greater than or equal to one second; validate before dispatch rather than silently truncating.


## Read-only schema diff

- `data::load_schema_snapshot(config, revision, table_name)` only reads the existing backend metadata and returns a domain `SchemaSnapshot`.
- `SchemaDiff` compares one current table with one target table and `sql_preview()`/`to_sql_preview()` only render SQL text; no migration statement is executed.
- SQLite column-definition, primary-key, unique-key, and foreign-key changes are emitted as review warnings rather than unsafe automatic migrations. Identifier names are quoted with `IdentifierDialect`.
## Release-acceptance backend gates

The PostgreSQL and MySQL acceptance jobs in `.github/workflows/ci.yml` are required by the tag release job. Standalone backend workflows remain diagnostic scheduled/manual/PR checks. Their `GRIDIX_TEST_PG_URL` / `GRIDIX_TEST_MYSQL_URL` preflight is mandatory in CI: a local no-URL return is convenience only, never release evidence.

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
- `DbManagerApp::execute_grid_save_typed()` converts the staged edits into a typed `MutationBatch` (`domain/mutation.rs`) and runs it through `execute_import_batch(&config, statements, use_transaction=true, stop_on_error=true)`.
- Result via `RuntimeOutcome::GridSaved { table_view, table, result, elapsed_ms }`, handled by the grid-save runtime-event handler.
- Committed (failed == 0) → `clear_edits()` + `RefreshSelectedTable`; rolled back → keep edits + error.
- `db_type` MUST travel with the `MutationBatch` so identifier quoting stays correct (MySQL backticks, PG/SQLite double-quotes); the historical `generate_save_sql` string path is gone. <!-- doc-symbols: ignore: names the removed pre-typed save path -->
- Do NOT revert to looping `execute()` per statement — that reintroduces partial-commit (audit B2) and post-save stale edits (audit B1).

## Password security

- `ConnectionConfig.password` is `#[serde(skip_serializing)]`
- `password_ref` (UUID) stored in config.toml, actual secret in OS keyring via `keyring` crate
- Legacy AES-256-GCM encrypted passwords auto-migrated to keyring on load (retain migration path)
- `pool_key()` uses SHA-256 of the connection identity material, including database credentials, TLS mode/CA, and SSH tunnel routing.

## SSH tunnel

`data/query/mod.rs` + `data/pool.rs` + `data/ssh_tunnel.rs`:
- `SshTunnelManager` singleton via `std::sync::LazyLock`
- Tunnels cached by name with `get_or_create`/`stop`
- `russh` + `known_hosts` verification with SHA-256 fingerprint logging
- Runtime TCP endpoint rewrites to `127.0.0.1:<dynamic_port>` while preserving the original database host as `tls_server_name`
- PostgreSQL uses `host=<tls_server_name>` plus `hostaddr=<loopback>`; MySQL uses `tls_hostname_override` for `VerifyIdentity`
- `pool_route_key_material()` includes the SSH tunnel identity and original TLS server name; rewritten loopback endpoints do not split a reusable pool, while different TLS names cannot share one.
- `SshError::HostKeyVerification` — distinct error variant for known_hosts mismatch vs. missing known_hosts
- SSH passwords and private key passphrases are `#[serde(skip_serializing)]`

## Error handling

`DbError` (thiserror, 2 active variants: Connection, Query). All errors use `#[error("...")]` for Display formatting.
SSL/TLS: 新建连接通过 `ConnectionConfig::new` 默认使用 PostgreSQL `VerifyFull` 和 MySQL `VerifyIdentity`。旧配置缺少 SSL 字段时保留兼容默认：PostgreSQL `Prefer` 先尝试 SSL，失败时回退明文；MySQL `Preferred` 使用 SSL 但跳过证书和主机名验证，不回退明文。显式 `Require`/`VerifyCa`/`VerifyFull`/`Required`/`VerifyIdentity` 模式必须使用证书验证；SSH 隧道下 `VerifyCa` 不因 loopback 改变 CA-only 语义，`VerifyFull`/`VerifyIdentity` 使用原始数据库 TLS server name。
