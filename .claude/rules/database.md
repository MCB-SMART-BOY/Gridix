---
paths:
  - src/data/**/*.rs
  - src/app/runtime/database.rs
  - src/app/runtime/handler.rs
---

# Gridix database rules

**Code is the source of truth.** Verify claims against `src/data/` before relying on them. Update this file when you change database code.

## Architecture

Three backends with divergent patterns:
- **SQLite**: synchronous via `rusqlite`. Wrapped in `task::spawn_blocking()`.
- **PostgreSQL**: async via `tokio-postgres`. Standalone metadata/grid uses the configuration-keyed pool; SQL editor Tabs each own a dedicated backend keyed by `(ConnectionId, DocumentId)`.
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
- SQLite 将被 SQL 分类器视为查询、但 `Statement::column_count() == 0` 的语句（如设置型 `PRAGMA user_version = 1`）作为无结果语句执行，返回 `AffectedRows`；零列时不得按列数计算行数。读取型 PRAGMA 仍返回 `ResultSet`。
- **PostgreSQL** standalone and ordinary Tab queries use the executing client's `CancelToken`; cancellation sends `CancelRequest` and then awaits the original query future, mapping the server cancellation to `DbError::Cancelled`. SQL Tab `BEGIN`/`COMMIT`/`ROLLBACK` instead closes its dedicated backend on in-flight cancellation to avoid waiting indefinitely or claiming a known transaction outcome. <!-- doc-symbols: ignore: PostgreSQL wire-protocol message name -->
- PostgreSQL Tab queries enqueue synchronously in editor submission order; enqueue receives the app's Tokio runtime `Handle` and spawns its actor on that runtime even when called from the synchronous GUI thread. Each actor serializes execution on one dedicated backend. `BEGIN` keeps a bounded portal-capable transaction until explicit `COMMIT`/`ROLLBACK`. Server-reported SQL errors and in-flight server cancellations leave the transaction aborted until explicit rollback; cancellation before dispatch does not send SQL and does not alter transaction state. Cancelling a server-waiting `BEGIN`, `COMMIT`, or `ROLLBACK` closes the dedicated backend promptly; `COMMIT`/`ROLLBACK` errors or cancellations return `DbError::TransactionOutcomeUnknown` rather than claiming a rollback. Forced Tab close/disconnect can drop the actor's reply instead; the UI classifies a connection/timeout error during control SQL as unknown. Even if the completion is stale after user cancellation, the UI reports confirmed commit/rollback or unknown outcome without overwriting newer Tab results. Timeout must not replace that error with a generic timeout. `DbError::TransactionOutcomeUnknown` retains the driver source for callers, but its Display does not expose server diagnostics in ordinary logs.
- SQL Tab transaction controls must be a single standalone `BEGIN`/`START TRANSACTION`, `COMMIT`, or `ROLLBACK` statement (optional trailing semicolon); transaction options, leading empty statements, savepoints, prepared transaction controls, and multi-statement transaction controls are explicitly unsupported. PostgreSQL control detection must skip nested block comments and CR/CRLF-terminated `--` comments as the server does, or reject malformed comments before dispatch. Closing a Tab via the Dock **or** `CloseActiveQueryTab` must close its actor; do not claim that an in-flight COMMIT has rolled back. Grid operations and metadata never use Tab backends. If cancellation submission fails while the SQL returns, preserve the SQL result for transaction state.
- In a PostgreSQL Tab transaction, a driver error without SQLSTATE indicates transport/protocol failure (`DbError::Connection`), not a recoverable server SQL error. Close that backend and reject already queued statements; do not let them escape into autocommit after server termination.
- Local typed-result decoding errors do not mark an explicit PostgreSQL Tab transaction aborted; only server SQL errors/cancellation do. Do not turn unsupported non-NULL OIDs into exportable placeholder values.
- **MySQL** records the execution `Conn::id()` and opens a separate TLS-configured control `Conn` to issue `KILL QUERY <connection_id>`; never derive the ID from SQL or user input.
- Cancellation is cooperative for runtime query tasks. It is not a substitute for aborting unrelated task kinds.

## Typed value boundaries

- PostgreSQL `NUMERIC` parameters and decoded results preserve exact `DbValue::Decimal` values.
- PostgreSQL `NUMERIC` special values decode as exact `DbValue::Decimal("NaN"|"Infinity"|"-Infinity")`; unsupported non-NULL PostgreSQL result types return an explicit query error rather than a synthetic value.
- MySQL temporal input rejects nanoseconds greater than or equal to one second; validate before dispatch rather than silently truncating.

## Catalog constraint fidelity

- SQLite groups composite primary and foreign keys by constraint identity and ordinal; an implicit foreign-key target resolves to the referenced primary-key columns in order. Only non-partial, non-expression unique indexes represent full-column unique keys.
- PostgreSQL pairs composite foreign-key columns by constraint OID and ordinal, qualifies cross-schema targets, and excludes `INCLUDE` columns from unique keys (`pg_index.indnkeyatts`).
- MySQL pairs composite foreign-key columns by constraint and ordinal, keeps primary-key order, and excludes prefix indexes from full-column unique keys. Do not treat individual columns of a composite constraint as separate constraints.
- Verify backend SQL with a real PostgreSQL/MySQL fixture, not an unset test URL; SQLite catalog tests use an isolated database.

## Read-only schema diff

- The Schema diff dialog reads `SchemaSnapshot::from_catalog` directly (the `load_schema_snapshot`/`load_snapshot` loaders were removed as dead code once the dialog moved to that path). <!-- doc-symbols: ignore: both symbols were deleted -->
- `SchemaDiff` compares one current table with one target table and `sql_preview()`/`to_sql_preview()` only render SQL text; no migration statement is executed.
- SQLite column-definition, primary-key, unique-key, and foreign-key changes are emitted as review warnings rather than unsafe automatic migrations. Identifier names are quoted with `IdentifierDialect`.
## Release-acceptance backend gates

The PostgreSQL, MySQL, and reusable TLS fixture acceptance jobs in `.github/workflows/ci.yml` are required by the tag release job. The PostgreSQL fixture runs the SQL Tab transaction suite as well as typed, cancellation and pool tests. Standalone backend workflows remain diagnostic scheduled/manual/PR checks. Their `GRIDIX_TEST_PG_URL` / `GRIDIX_TEST_MYSQL_URL` preflight is mandatory in CI: a local no-URL return is convenience only, never release evidence.

## Pooling

`data/pool.rs` — manual pooling, NOT a generic pool crate:
- MySQL: `HashMap<String, (Pool, Instant)>` — idle timeout, LRU eviction, health-check
- PostgreSQL: configuration-keyed `HashMap<String, (Arc<PgPooledClient>, Instant)>` — lease owns the client mutex and backend task, whose last-owner drop closes the connection.
- SQLite: not pooled ("doesn't need it")
- SQL Tab PostgreSQL actors are **not** part of the configuration-keyed pool and are not evicted by its LRU; they close on Tab/connection lifecycle events.
- Cache lookups clone the cached pool/client under a short read lock, then await health or capacity checks outside that lock. Before updating last-used or removing an unhealthy entry, verify that its cache identity is still current; disconnecting a removed MySQL pool also runs outside the cache lock. A saturated/busy backend must not block unrelated database pool lookups.
- Concurrent PostgreSQL cold misses publish only one backend for the same key. LRU eviction and explicit cache removal detach a borrowed backend rather than abort it; the connection closes when its last borrower releases the lease.
- PostgreSQL/MySQL cancellable standalone queries select on the cancellation token while waiting for a pool/client or execution connection; once acquired, they check the token again before sending SQL. In-flight cancellation still uses each backend's server-side protocol.

## QueryResult null handling

`QueryResult.null_flags: Vec<Vec<bool>>` is a **parallel array** to `rows`.
`null_flags[row][col] == true` means the value is SQL NULL (the corresponding string is empty).
This is deliberate — avoids sentinel values for distinguishing NULL from empty string.

## Import and export value fidelity

- `TransferSession.sql_dialect` is the target SQL dialect for both SQL statements and CSV/TSV/JSON import identifiers; do not add a second dialect field to format options.
- Typed MySQL `BINARY`/`VARBINARY` must remain `DbValue::Bytes` even when the payload is not UTF-8; typed `NULL` is distinct. SQL export renders binary as SQLite `X'HEX'`, MySQL `0xHEX`, PostgreSQL `decode('HEX','hex')`.
- CSV/TSV/JSON export rejects binary values rather than rendering lossy display strings; delimited export also rejects text whose empty/trimmed/numeric/boolean-like content would parse as a different type on import.
- Preserve fractional time precision and decimal numeric ordering on typed paths. Wrapped imports validate raw SQL and generated statements before the first write using the active connection's SQL dialect; transaction control is forbidden on PostgreSQL/SQLite/MySQL. PostgreSQL bare-CR comments, nested comments and dollar quoting, and MySQL backtick identifiers must not hide transaction control. PostgreSQL backticks are operators, not quoting delimiters. PostgreSQL/MySQL backslash-containing statements are rejected under transaction wrapping when escape semantics cannot be safely inferred. MySQL permits only INSERT/UPDATE/DELETE/REPLACE/SELECT and rejects executable versioned comments. Non-transactional MySQL engines remain outside rollback guarantees.
- SQLite's UTF-8 BOM is SQL whitespace outside literals; wrapped-import control detection must skip it at token boundaries without modifying literal contents. PostgreSQL line comments can end at bare CR; MySQL `--` and `#` comments (like SQLite `--`) continue to LF, so a bare CR must not turn commented SQL into executable statements. Keep CR/CRLF inside quoted literals unchanged. PostgreSQL fatal errors use non-localized parsed severity, and absent severity is an uncertain outcome in unwrapped imports.
- MySQL `--` starts a line comment only when followed by an ASCII space or control character, not Unicode whitespace encoded as non-ASCII UTF-8; never allow a hidden quote to make a wrapped import skip transaction control or silently rewrite a `WHERE` clause. PostgreSQL dollar-tag scanning must reject unsupported non-ASCII tags conservatively without treating their byte offsets as character indices. SQL import preserves comment delimiters in unstripped output but must still track their lexical boundaries; a MySQL `DELIMITER` client command is recognized only at statement boundaries, never inside a statement or in PostgreSQL/SQLite SQL text. Validate direct and planned imports before executing any statement.
- A PostgreSQL/MySQL import COMMIT or ROLLBACK acknowledgement error, an unwrapped statement's transport error, or a MySQL incomplete-rollback warning is `DbError::ImportOutcomeUnknown`, not a confirmed rollback. The UI requires explicit database verification and acknowledgement before another import. Do not stringify this error before `RuntimeOutcome::ImportDone` reaches the UI recovery boundary.

## Grid save (transactional batch)

Grid cell edits/inserts/deletes are saved as ONE typed atomic batch, not N independent queries:
- `DbManagerApp::execute_grid_save_typed()` captures submitted edits for the owning `GridWorkspaceId` and `TaskId`, converts them into `MutationBatch` (`domain/mutation.rs`), and runs `apply_mutations()`.
- Result via `RuntimeOutcome::GridSaved { table_view, table, result, elapsed_ms }`, handled on the UI thread.
- Confirmed commit (failed == 0) → retire submitted edits from the owning workspace and lock its stale result until a post-commit refresh succeeds; confirmed rollback → keep draft and unlock. A save snapshot must carry the submitting `ConnectionId`: a same-name replacement must not retire edits or change locks. PostgreSQL/MySQL mutation COMMIT or ROLLBACK acknowledgement failure, or a MySQL incomplete-rollback warning → outcome unknown: keep draft quarantined, do not report rollback or permit replay; after checking database state, explicitly discard the draft and refresh. Other workspaces remain editable.
- Before switching the active connection or selected database (including disconnect, failure and selected-database deletion), persist and deactivate the old grid under its old identity; never save an old grid after the identity changes.
- `QueryTab.result_origin` records the connection ID and database that produced a result. Tab navigation must not project stale results or table selection onto a different connection/database, and grid save must reject a mismatched origin. Successful table/database deletion removes only the target identity's workspaces and deactivates a matching visible grid before removing its drafts.
- `db_type` MUST travel with the `MutationBatch` so identifier quoting stays correct (MySQL backticks, PG/SQLite double-quotes); the historical `generate_save_sql` string path is gone. <!-- doc-symbols: ignore: names the removed pre-typed save path -->
- Do NOT revert to looping `execute()` per statement — that reintroduces partial-commit (audit B2) and post-save stale edits (audit B1).

## Password security

- `ConnectionConfig.password` is `#[serde(skip_serializing)]`
- `password_ref` (UUID) stored in config.toml, actual secret in OS keyring via `keyring` crate
- Legacy AES-256-GCM encrypted passwords auto-migrated to keyring on load (retain migration path)
- `pool_key()` uses SHA-256 of the connection identity material, including database credentials, TLS mode/CA, and SSH tunnel routing.
- Connection previews use `ConnectionConfig::connection_string_masked()` rather than constructing a secret-bearing string and replacing raw password text; encoded PostgreSQL/MySQL passwords must never appear on screen.
- `AppConfig` parse failures log only the config path and TOML error span, not the parser's source-line-bearing Display output.
- Only a user edit explicitly clearing a database password revokes its `password_ref` and deletes its keyring item. Missing/unreadable keyring values leave the reference intact; deletion failure still revokes the reference and reports an orphaned-secret warning.

## SSH tunnel

`data/query/mod.rs` + `data/pool.rs` + `data/ssh_tunnel.rs`:
- `SshTunnelManager` singleton via `std::sync::LazyLock`
- Tunnels are keyed by SSH endpoint **and** the owning `ConnectionConfig` name/runtime `ConnectionId`; connection registration assigns the runtime ID, which is not serialized. Query setup, pool routing, and disconnect must use the same key. Two connections sharing a jump host must never share an independently stopped tunnel or pool; replacing a same-named connection must not let its late cleanup stop the replacement.
- `stop` waits for the listener and all forwarding tasks to exit; dropping an un-stopped tunnel aborts its owner task. Concurrent first-creation losers are explicitly stopped before returning the cached winner.
- `russh` host-key callback checks `known_hosts`; rejects certificates, unknown keys, mismatches, and read errors with `Ok(false)` after logging diagnostic context.
- Host certificates are explicitly rejected (no certificate CA policy); ordinary server public keys must pass `known_hosts`.
- Runtime TCP endpoint rewrites to `127.0.0.1:<dynamic_port>` while preserving the original database host as `tls_server_name`
- PostgreSQL uses `host=<tls_server_name>` plus `hostaddr=<loopback>`; MySQL uses `tls_hostname_override` for `VerifyIdentity`
- `pool_route_key_material()` includes the SSH tunnel identity and original TLS server name; rewritten loopback endpoints do not split a reusable pool, while different TLS names cannot share one.
- SSH host-key mismatch and missing-entry diagnostics are logged at the callback boundary; the callback does not propagate a distinct `SshError::HostKeyVerification` to the caller.
- SSH passwords and private key passphrases are `#[serde(skip_serializing)]`

## Error handling

`DbError` (thiserror, 2 active variants: Connection, Query). All errors use `#[error("...")]` for Display formatting.
SSL/TLS: 新建连接通过 `ConnectionConfig::new` 默认使用 PostgreSQL `VerifyFull` 和 MySQL `VerifyIdentity`。旧配置缺少 SSL 字段时保留兼容默认：PostgreSQL `Prefer` 先尝试 SSL，失败时回退明文；MySQL `Preferred` 使用 SSL 但跳过证书和主机名验证，不回退明文。显式 `Require`/`VerifyCa`/`VerifyFull`/`Required`/`VerifyIdentity` 模式必须使用证书验证；SSH 隧道下 `VerifyCa` 不因 loopback 改变 CA-only 语义，`VerifyFull`/`VerifyIdentity` 使用原始数据库 TLS server name。
