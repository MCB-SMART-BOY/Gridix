# Database Backend Guide

How to add or modify a database backend.

## Architecture

```
data/query/mod.rs       → orchestrator, match db_type dispatch
data/query/sqlite.rs    → SQLite (sync, spawn_blocking)
data/query/postgres.rs  → PostgreSQL (async, tokio-postgres)
data/query/mysql.rs     → MySQL (async, mysql_async, pooled)
```

## Typed query, mutation, and catalog dispatch

`data/query/mod.rs` exposes two public typed execution APIs:

- `execute_typed(config, sql)` for normal execution.
- `execute_typed_cancellable(config, sql, cancellation)` for runtime query tasks.

Both dispatch by `DatabaseType`; SQLite work is run with `spawn_blocking`, while PostgreSQL and MySQL execute asynchronously. `apply_mutations()` and `load_schema_catalog()` use the same three-backend dispatch. Typed `MutationBatch` insert/update/delete and `SchemaCatalog` loading are implemented for SQLite, PostgreSQL, and MySQL.

## Cancellation strategies

| Backend | Runtime behavior | Mechanism |
|---------|------------------|-----------|
| SQLite | Does not promise cancellation of a statement already running in synchronous `rusqlite`. | Continues through the SQLite typed execution path. |
| PostgreSQL | Cooperative server-side cancellation. | The execution client supplies `tokio_postgres::CancelToken`; a cancel request is sent and the original query future is awaited to its cancellation result. |
| MySQL | Cooperative server-side cancellation. | The executing `Conn::id()` identifies the query; a separately opened, TLS-configured control connection sends `KILL QUERY <connection_id>`, then the original execution is awaited. |

Do not substitute task abort for these database protocols. A pre-cancelled token returns `DbError::Cancelled` without dispatching the query. PostgreSQL and MySQL integration tests use an observer marker to establish that an in-flight query was seen, cancelled, disappeared, and left the pool able to execute `SELECT 1`.

## Typed value boundaries

- PostgreSQL `NUMERIC` preserves `DbValue::Decimal` exactly for parameter binding and result decoding; it is not converted through floating point.
- MySQL `DECIMAL` is represented as `DbValue::Decimal`.
- MySQL temporal bindings require whole microseconds and reject invalid components, including `nanos >= 1_000_000_000`; callers must not rely on normalization of an overflowed nanosecond field.

## MySQL cancellation coverage boundary

The release-acceptance workflow covers direct `mysql:8.4` connections with the minimum observer and `KILL QUERY` privileges (`PROCESS`, `CONNECTION_ADMIN`). It does not yet cover TLS, SSH tunnels, or execution-pool capacity pressure. Its post-cancellation `SELECT 1` proves that the pool can execute another query; it does not prove reuse of the exact `Conn` that was cancelled. These are coverage boundaries, not known functional failures.

## Adding a new backend

1. Add variant to `DatabaseType` in `src/types.rs`
2. Create `data/query/<backend>.rs` with all required functions
3. Add dispatch arms in `data/query/mod.rs`
4. Add pooling logic in `data/pool.rs`
5. Update `data/config.rs` for connection string
6. Add SSL mode if applicable

Required backend responsibilities are dispatched from `data/query/mod.rs` rather than exposed through a backend trait:

- typed execution and cancellable typed execution where the backend can support it;
- `apply_mutations()` and `load_schema_catalog()`;
- schema browsing (`get_tables`, databases where applicable, triggers, routines, foreign keys, columns);
- connection, import-batch, and database-drop operations appropriate to the backend.

## Why no trait

SQLite (sync/spawn_blocking), PostgreSQL (async/direct), MySQL (async/pooled) have fundamentally different execution models. A trait forces identical signatures onto incompatible patterns. Enum dispatch allows each backend to use its natural pattern.

Previous attempt: `DatabaseDriver` trait in `database/driver.rs` (deleted as dead code).

## Pool management

`data/pool.rs` — manual pooling, NOT a generic pool crate:
- MySQL: `HashMap<String, (Pool, Instant)>` with TTL + LRU eviction
- PostgreSQL: `HashMap<String, (Arc<Client>, Instant)>` with health check
- SQLite: not pooled (file-based, "doesn't need it")
