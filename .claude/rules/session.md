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
- Request IDs: `next_*_request_id` (private, method access only)
- Pending tracking: `pending_*` maps

**Key methods:**
- `active_sql()`, `set_active_sql()`, `ensure_active_tab()`
- `next_connect_request_id()`, `next_query_request_id()`, `next_metadata_request_id()`
- `refresh_connecting_flag()`, `refresh_executing_flag()`
- `track_query_task()`

## Message handling

`handle_messages()` is on `DbManagerApp`. Uses `self.session.rx.try_recv()` to poll. Handlers set `self.session.needs_repaint = true` instead of `ctx.request_repaint()`. At end of loop: check needs_repaint → ctx.request_repaint() → clear.

## FrameEffects

Types defined in `session/frame_effects.rs`. Not yet wired. `needs_repaint` provides minimal decoupling.

## Invariants

- All Mutex::lock() use `unwrap_or_else(|e| e.into_inner())`
- SSH tunnels stopped via `Handle::spawn()`, not `std::thread::spawn()`
- Request IDs generated via private methods (monotonic, wraparound-safe)
