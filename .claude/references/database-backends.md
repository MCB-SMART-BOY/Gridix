# Database Backend Guide

How to add or modify a database backend.

## Architecture

```
data/query/mod.rs       → orchestrator, match db_type dispatch
data/query/sqlite.rs    → SQLite (sync, spawn_blocking)
data/query/postgres.rs  → PostgreSQL (async, tokio-postgres)
data/query/mysql.rs     → MySQL (async, mysql_async, pooled)
```

## Dispatch pattern

Each public function in `data/query/mod.rs` follows:

```rust
match effective_config.db_type {
    DatabaseType::SQLite => {
        task::spawn_blocking(move || sqlite::function(&config, ...))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?
    }
    DatabaseType::PostgreSQL => {
        postgres::function(&config, ...).await
    }
    DatabaseType::MySQL => {
        mysql::function(&config, ...).await
    }
}
```

SQLite always wrapped in `spawn_blocking`. PG/MySQL use direct async.

## Cancel strategies

| Backend | Mechanism | File |
|---------|-----------|------|
| SQLite | `rusqlite::InterruptHandle` | `sqlite.rs` |
| PostgreSQL | `tokio_postgres::CancelToken` | `postgres.rs` |
| MySQL | `KILL QUERY <connection_id>` via dedicated conn | `mysql.rs` |

## Adding a new backend

1. Add variant to `DatabaseType` in `src/types.rs`
2. Create `data/query/<backend>.rs` with all required functions
3. Add dispatch arms in `data/query/mod.rs`
4. Add pooling logic in `data/pool.rs`
5. Update `data/config.rs` for connection string
6. Add SSL mode if applicable

Required functions (see `sqlite.rs` for signatures):
- `connect()`, `execute()`, `execute_cancellable()`
- `get_tables()`, `get_databases()` (for non-SQLite)
- `get_triggers()`, `get_routines()`, `get_foreign_keys()`, `get_columns()`
- `drop_database()`, `execute_import_batch()`

## Why no trait

SQLite (sync/spawn_blocking), PostgreSQL (async/direct), MySQL (async/pooled) have fundamentally different execution models. A trait forces identical signatures onto incompatible patterns. Enum dispatch allows each backend to use its natural pattern.

Previous attempt: `DatabaseDriver` trait in `database/driver.rs` (deleted as dead code).

## Pool management

`data/pool.rs` — manual pooling, NOT a generic pool crate:
- MySQL: `HashMap<String, (Pool, Instant)>` with TTL + LRU eviction
- PostgreSQL: `HashMap<String, (Arc<Client>, Instant)>` with health check
- SQLite: not pooled (file-based, "doesn't need it")
