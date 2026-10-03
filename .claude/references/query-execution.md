# Query execution trace

（原始追踪记录未入库；本文档是唯一在库描述。）The full chain from user action to rendered result.

## End-to-end chain

```
handle_sql_editor_actions() → DbManagerApp::execute()
→ PostgreSQL: TaskRegistry::register_queued_query(connection, document)
→ PostgreSQL: enqueue_pg_tab_query() → dedicated backend actor → await_pg_tab_query()
  SQLite/MySQL: TaskRegistry::register(OperationKey::Query)
  SQLite/MySQL: tokio task → execute_typed_cancellable()
→ Message::RuntimeEvent(RuntimeEvent { task_id, key, outcome })
→ handle_messages() → TaskRegistry::is_current() → render
```

`RuntimeEvent` is the completion protocol for the typed runtime path. The event carries `TaskId` and `OperationKey`; query completions also carry their submission `request_id`. A completion may update the tab only when the registry task, live connection identity, document identity and pending request ID all still match. A task still current for connection A's key cannot overwrite a tab reused by connection B. Older `Message::QueryDone` and pending-query descriptions are historical migration context, not the lifecycle to extend. <!-- doc-symbols: ignore: names removed migration-era types on purpose -->

## Authority model

- `QueryTab` holds per-tab facts (SQL, result, timestamp) — the **authority**.
- Active-tab render fields are mirrors, not independent request authorities.
- `TaskRegistry` owns task identity, deduplication, cancellation tokens, and stale-completion filtering for runtime operations.

PostgreSQL SQL editor submissions enqueue before spawning their timeout task, preserving same-Tab ordering. `(ConnectionId, DocumentId)` owns one serial actor and backend; other Tabs, metadata, and grid operations use different clients. `BEGIN`/`START TRANSACTION` starts an actor-held transaction, and `COMMIT`/`ROLLBACK` ends it; the bounded portal SELECT does not add an implicit nested transaction. Server errors or in-flight server cancellations preserve PostgreSQL's aborted transaction state until explicit `ROLLBACK`; pre-dispatch cancellation sends no SQL and leaves the transaction unchanged. Closing via either Dock or the `CloseActiveQueryTab` action, disconnecting, or changing database closes the dedicated backend: uncommitted work rolls back, but an in-flight `COMMIT`/`ROLLBACK` may already have reached the server and is reported as an unknown outcome. Standalone `execute_typed` rejects transaction control even after nested PostgreSQL block comments or CR/CRLF-terminated `--` comments; unclosed comments and unsupported control forms are rejected before dispatch.

PostgreSQL typed result conversion returns a query error for unsupported non-NULL OIDs rather than constructing a display placeholder that could be exported as data. Binary NUMERIC `NaN`, `Infinity`, and `-Infinity` decode as exact `DbValue::Decimal` strings. A result conversion error happens locally after PostgreSQL successfully executes SQL, so it does not mark an explicit SQL Tab transaction aborted; a server SQL error or server-side cancellation still requires an explicit `ROLLBACK`. Standalone typed SELECTs roll back their internal bounded-portal transaction on conversion failure.

PostgreSQL queued SQL is never supersession-cancelled: if `BEGIN` is still pending when an `INSERT` is submitted, both execute in order on the same backend. Cancelling or failing the pending `BEGIN` closes that backend instead of running already-queued writes in autocommit. Query cancellation/connection disconnect targets every outstanding query token for the Tab, while only the latest completion may update the Tab's grid state.

## SQLite zero-column statements

The typed SQLite executor checks the prepared statement's column count before collecting result rows. A zero-column statement classified as a query, such as `PRAGMA user_version = 1`, executes as a command and returns `AffectedRows`; a reading PRAGMA still returns a `ResultSet`. This avoids dividing by zero in the result collector and keeps the public `execute_typed`/`execute_typed_cancellable` entry points consistent.

## Cooperative cancellation

1. A user cancel, timeout, or superseding query requests the query task's `CancellationToken`.
2. A query ordinarily awaits the server's cancellation response before emitting a completion event. If a dedicated PostgreSQL Tab backend does not acknowledge cancellation within `QUERY_CANCEL_GRACE_SECS`, its backend is closed rather than reusing a connection with uncertain protocol state. If the cancel request fails but the SQL returns, the SQL result—not the auxiliary cancellation error—determines success and transaction-aborted state.
3. PostgreSQL sends a `CancelToken` request; MySQL opens a control connection and sends `KILL QUERY` for the execution connection ID.
4. SQLite does not promise interruption of a statement already executing in synchronous `rusqlite`.
5. The completion event updates Tab state only if `TaskRegistry::is_current(key, task_id)` remains true. A stale PostgreSQL `COMMIT`/`ROLLBACK` completion still emits a standalone notification for a confirmed success or an unknown outcome; it cannot overwrite a newer query's result.

Non-query task kinds retain their abort-on-cancellation behavior. Query cancellation is deliberately cooperative so the database cancellation protocol and original query future can finish cleanly.

## Error rendering

Query errors surface through the workspace Welcome surface, which renders `welcome_status` via `ui::Welcome::show` — not as a blank result pane. User cancellation only reports that a cancellation was requested. `DbError::TransactionOutcomeUnknown` reports an ambiguous PostgreSQL `COMMIT`/`ROLLBACK` and closes the dedicated backend; UI classification also treats a backend-close/timeout error during control SQL as unknown. The UI preserves the warning across timeout or stale-result filtering and advises checking the database before retrying. The typed error retains its source chain, but its Display omits the server's sensitive diagnostic detail before GUI/log rendering.

## Remaining UX and migration context

1. `QueryTab` is the SQL authority; active-tab result fields are render mirrors rather than a second request lifecycle.
2. Cancellation feedback is transient; there is no persistent “query was cancelled” indicator.
3. Legacy request-ID structures may still exist for migration compatibility, but new query runtime behavior must use the `TaskRegistry` + `RuntimeEvent` path.

## Verification

Use the targeted unit and integration invocations in `testing-guide.md`. PostgreSQL and MySQL cancellation evidence requires `GRIDIX_TEST_PG_URL` or `GRIDIX_TEST_MYSQL_URL` respectively; an unset URL is a local skip, not proof of the cancellation path.
