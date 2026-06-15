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

## Cancel flow

Each backend has a different cancel strategy:
- **SQLite**: `rusqlite::InterruptHandle` via `execute_with_interrupt_handle()`
- **PostgreSQL**: `tokio_postgres::CancelToken` with `tokio::select!`
- **MySQL**: `KILL QUERY <connection_id>` via dedicated kill connection with pool fallback

## Pooling

`data/pool.rs` — manual pooling, NOT a generic pool crate:
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
