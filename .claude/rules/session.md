---
paths:
  - src/session/**/*.rs
---

# Gridix session rules

**Code is the source of truth.** Verify against `src/session/`.

## Session struct (~30 fields)

```
Layer 2 — owns all async infrastructure and connection lifecycle.
Bridge between data/ (Layer 1) and state/ (Layer 3).
```

**Key fields:**
- `manager`, `tab_manager` — connection and tab state
- `runtime`, `tx`, `rx` — async infrastructure
- `needs_repaint` — handler sets, handle_messages checks + clears
- `notifications`, `progress` — UI feedback
- `autocomplete`, `command_history`, `query_history` — editor state
- Request IDs: `next_query_request_id()` (private field, method access only)
- Pending tracking: `pending_*` maps, presence-only. They answer "is a request in flight for this
  connection/database" and never decide freshness: stale replies are rejected by
  `TaskRegistry::is_current()` (event entry), by the connection id carried in the outcome
  (`does_runtime_connection_match`, used by the connect / select-database / metadata / active-tables
  / drop handlers), and by `metadata_context_matches_current`. Comparing a stored *name* against the
  outcome's *id* never matches, which is why those guards are gone. Do not reintroduce request ids
  into these fields — a stored id can only be compared against itself.
- A same-name connection rebuilt by editing its config gets a new `ConnectionId` and a new
  `OperationKey::Connect(id)`, so the superseded task stays "current" for its own key: the id guard
  is what keeps its late reply from writing into the new connection object and swallowing the real
  reply.
- `QueryTab.result_origin` records the submitting connection ID and database. Reopening a Tab on another identity must hide its result/table grid; saving requires a matching origin. Async table/database deletion workspaces use the event's target identity rather than whichever connection is active when the response arrives.
- `RuntimeOutcome::ActiveTablesReloaded` carries the requested database name as well as `ConnectionId`; verify both at the UI boundary before applying sidebar tables/autocomplete. A current task key on an old database is not proof that the view still belongs to that database.

**Key methods:**
- `active_sql()`, `set_active_sql()`, `ensure_active_tab()`
- `next_query_request_id()`
- `refresh_connecting_flag()`, `refresh_executing_flag()`
- `task_registry` — typed task registry (`register`/`register_queued_query`/`attach`/`complete`/`cancel_by_key`) supersedes pending-request maps. `OperationKey::Query { connection, document }` is scoped by connection: `cancel_queries_for_connection` and `cancel_queries_for_document` cancel *all* in-flight/queued matching tasks, not only the latest. PostgreSQL Tab submissions use `register_queued_query`, so newer SQL never implicitly cancels earlier SQL. Query completion also checks its submitted `request_id`, live `ConnectionId`, active connection and stable document ID before updating a Tab; a task current under a prior connection key must not overwrite a reused Tab or unlock a stale grid refresh. A stale `COMMIT`/`ROLLBACK` completion still reports confirmed success or `TransactionOutcomeUnknown` as a separate notification, never writing its result over the newer Tab state. Every long-running task must be `attach`ed to keep its cancellation token reachable.

- A SQL Tab PostgreSQL backend is keyed by the connection's `ConnectionId` and the Tab's stable UUID-derived `DocumentId`. Closing a document via Dock or `CloseActiveQueryTab` closes its backends across all connections; disconnect and successful database switch close that connection's backends before other queries can reuse them. `Session::drop` closes all remaining registered backends before dropping its runtime.

## Message handling

`handle_messages()` is on `DbManagerApp`. Uses `self.session.rx.try_recv()` to poll. Handlers set `self.session.needs_repaint = true` instead of `ctx.request_repaint()`. At end of loop: check needs_repaint → ctx.request_repaint() → clear.

## FrameEffects

Types defined in `session/frame_effects.rs`. Not yet wired. `needs_repaint` provides minimal decoupling.

## Invariants

- All Mutex::lock() use `unwrap_or_else(|e| e.into_inner())`
- SSH tunnels stopped via `Handle::spawn()`, not `std::thread::spawn()`
- Request IDs generated via private methods (monotonic, wraparound-safe)
