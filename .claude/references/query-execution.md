# Query execution trace

From `docs/recovery/02-query-execution-trace.md`. The full chain from user action to rendered result.

## End-to-end chain

```
handle_sql_editor_actions() → execute() → tokio::spawn → execute_query()
→ Message::QueryDone { sql, conn_name, tab_id, request_id, result, elapsed_ms }
→ handle_messages() → handle_query_done() → sync_from_active_tab() → render
```

## Authority model

- `QueryTab` holds per-tab facts (sql, result, timestamp) — the **authority**
- `self.result`, `self.last_query_time_ms` are **active-tab render mirrors**
- `self.sql` is **dual-source** — both main field and active tab copy (structural cost, not yet unified)

## Stale response guard

Every `Message` variant carries `request_id: u64`. `handle_*()` methods:
1. Compare message's request_id against latest pending request for that conn/tab
2. If stale → log + ignore
3. If current → update state + `ctx.request_repaint()`

## Cancel flow

1. User triggers cancel (UI action or timeout)
2. `cancel_query_request()` sends via `oneshot` channel or aborts `JoinHandle`
3. Database-specific cancel (KILL QUERY / CancelToken / InterruptHandle)
4. Grace period (50ms) for DB to process cancel
5. If graceful cancel fails → abort JoinHandle (force kill)

## Error rendering

Query errors are rendered as a **Welcome surface** with the error message, not as a blank result pane. The `is_cancelled_query_error()` function checks both Chinese and English error messages.

## Known structural costs

1. **Dual-source `self.sql`**: both `DbManagerApp::sql` and `active_tab().sql` exist — can diverge
2. **Mirror vs authority**: `self.result` mirrors active tab but is not canonical
3. **No stable cancel UI**: cancel feedback is transient, no persistent "query was cancelled" indicator

## Validation

```bash
cargo test -p gridix --lib runtime   # handler + database tests
cargo test --test edge_regression_tests  # edge cases including tab isolation
```
