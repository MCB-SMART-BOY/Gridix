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

**Key methods:**
- `active_sql()`, `set_active_sql()`, `ensure_active_tab()`
- `next_query_request_id()`
- `refresh_connecting_flag()`, `refresh_executing_flag()`
- `task_registry` — typed task registry (`register`/`attach`/`complete`/`cancel_by_key`) that
  supersedes the pending-request maps for new runtime work. `OperationKey::Query { connection,
  document }` is scoped by connection: `cancel_queries_for_connection` cancels only that
  connection's in-flight queries, `cancel_queries_for_document` cancels one tab's query. Every
  long-running task must be `attach`ed so `cancel_by_key` can reach its cancellation token.

## Message handling

`handle_messages()` is on `DbManagerApp`. Uses `self.session.rx.try_recv()` to poll. Handlers set `self.session.needs_repaint = true` instead of `ctx.request_repaint()`. At end of loop: check needs_repaint → ctx.request_repaint() → clear.

## FrameEffects

Types defined in `session/frame_effects.rs`. Not yet wired. `needs_repaint` provides minimal decoupling.

## Invariants

- All Mutex::lock() use `unwrap_or_else(|e| e.into_inner())`
- SSH tunnels stopped via `Handle::spawn()`, not `std::thread::spawn()`
- Request IDs generated via private methods (monotonic, wraparound-safe)
