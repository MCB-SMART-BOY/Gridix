---
paths:
  - src/database/**/*.rs
  - src/app/runtime/database.rs
  - src/app/runtime/handler.rs
---

# Gridix database rules

**Code is the source of truth.** Verify claims against `src/database/` before relying on them. Update this file when you change database code.

## Architecture

Three backends with divergent patterns:
- **SQLite**: synchronous via `rusqlite`. Wrapped in `task::spawn_blocking()`.
- **PostgreSQL**: async via `tokio-postgres`. Single `Arc<Client>` per connection.
- **MySQL**: async via `mysql_async`. Pool-based with idle TTL + health checks.

Orchestrator: `database/query/mod.rs` dispatches via `match db_type`.

## Connection lifecycle

1. `connect()` in `app/runtime/database.rs` → spawns async task with timeout
2. `connect_database()` in `query/mod.rs` → SSH tunnel setup → backend-specific connect
3. Result via `Message::ConnectedWithTables/Databases` on mpsc channel
4. `handle_messages()` dispatches to `handle_connected_with_*()` — validates request_id, updates state

## Cancel flow

Each backend has a different cancel strategy:
- **SQLite**: `rusqlite::InterruptHandle` via `execute_with_interrupt_handle()`
- **PostgreSQL**: `tokio_postgres::CancelToken` with `tokio::select!`
- **MySQL**: `KILL QUERY <connection_id>` via dedicated kill connection with pool fallback

## Pooling

`database/pool.rs` — manual pooling, NOT a generic pool crate:
- MySQL: `HashMap<String, (Pool, Instant)>` — idle timeout, LRU eviction, health-check
- PostgreSQL: `HashMap<String, (Arc<Client>, Instant)>` — `client.is_closed()` health check
- SQLite: not pooled ("doesn't need it")

## QueryResult null handling

`QueryResult.null_flags: Vec<Vec<bool>>` is a **parallel array** to `rows`.
`null_flags[row][col] == true` means the value is SQL NULL (the corresponding string is empty).
This is deliberate — avoids sentinel values for distinguishing NULL from empty string.

## Password security

- `ConnectionConfig.password` is `#[serde(skip_serializing)]`
- `password_ref` (UUID) stored in config.toml, actual secret in OS keyring via `keyring` crate
- Legacy AES-256-GCM encrypted passwords auto-migrated to keyring on load
- `pool_key()` uses SHA-256 of full connection params (including password) for unique pool identity

## SSH tunnel

`database/ssh_tunnel.rs`:
- `SshTunnelManager` singleton via `lazy_static!`
- Tunnels cached by name with `get_or_create`/`stop`
- `russh` + `known_hosts` verification
- Config rewritten to `127.0.0.1:<dynamic_port>` before connecting
- `pool_route_key_material()` includes tunnel routing so pool keys remain stable after rewrite
- `SshError::HostKeyVerification` — distinct error variant for known_hosts mismatch vs. missing known_hosts

## Error handling

`DbError` (thiserror, 5 variants). Helper constructors:
- `connection_typed(db_type, message)` — typed connection errors
- `query_with_context(db_type, message, sql)` — SQL truncated to 200 chars
- Error messages are Chinese-localized for user display
