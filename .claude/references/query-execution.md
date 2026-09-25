# Query execution trace

（原始追踪记录未入库；本文档是唯一在库描述。）The full chain from user action to rendered result.

## End-to-end chain

```
handle_sql_editor_actions() → DbManagerApp::execute()
→ TaskRegistry::register(OperationKey::Query)
→ tokio task → execute_typed_cancellable()
→ Message::RuntimeEvent(RuntimeEvent { task_id, key, outcome })
→ handle_messages() → TaskRegistry::is_current() → render
```

`RuntimeEvent` is the completion protocol for the typed runtime path. The event carries the `TaskId` and `OperationKey`; only the current task for that key may update UI state. Older `Message::QueryDone` and pending-query descriptions are historical migration context, not the lifecycle to extend. <!-- doc-symbols: ignore: names removed migration-era types on purpose -->

## Authority model

- `QueryTab` holds per-tab facts (SQL, result, timestamp) — the **authority**.
- Active-tab render fields are mirrors, not independent request authorities.
- `TaskRegistry` owns task identity, deduplication, cancellation tokens, and stale-completion filtering for runtime operations.

## Cooperative cancellation

1. A user cancel, timeout, or superseding query requests the query task's `CancellationToken`.
2. A query is not force-aborted: it is retained until its typed execution returns and emits a completion event.
3. PostgreSQL sends a `CancelToken` request; MySQL opens a control connection and sends `KILL QUERY` for the execution connection ID.
4. SQLite does not promise interruption of a statement already executing in synchronous `rusqlite`.
5. The completion event is accepted only if `TaskRegistry::is_current(key, task_id)` remains true; otherwise it is discarded as stale.

Non-query task kinds retain their abort-on-cancellation behavior. Query cancellation is deliberately cooperative so the database cancellation protocol and original query future can finish cleanly.

## Error rendering

Query errors surface through the workspace Welcome surface, which renders `welcome_status` via `ui::Welcome::show` — not as a blank result pane. Cancellation arrives through the same error channel as `DbError::Cancelled`, and a user-requested cancel is reported by the `was_cancelled` flag on the emitted frame effect.

## Remaining UX and migration context

1. `QueryTab` is the SQL authority; active-tab result fields are render mirrors rather than a second request lifecycle.
2. Cancellation feedback is transient; there is no persistent “query was cancelled” indicator.
3. Legacy request-ID structures may still exist for migration compatibility, but new query runtime behavior must use the `TaskRegistry` + `RuntimeEvent` path.

## Verification

Use the targeted unit and integration invocations in `testing-guide.md`. PostgreSQL and MySQL cancellation evidence requires `GRIDIX_TEST_PG_URL` or `GRIDIX_TEST_MYSQL_URL` respectively; an unset URL is a local skip, not proof of the cancellation path.
