---
paths:
  - src/session/**/*.rs
---

# Gridix session rules

**Code is the source of truth.** Verify claims against `src/session/` before relying on them. Update this file when you change session code.

## Architecture

`Session` owns all async infrastructure and connection lifecycle.

```
data/ (Layer 1) → Session (Layer 2) → FrameEffects (defined, not yet wired) → state/ (Layer 3) → ui/ (Layer 4)
```

`FrameEffects` types defined in `session/frame_effects.rs`. Wiring delayed until QueryDone handler is ready for migration.

## Session struct (current state)

```rust
pub struct Session {
    pub manager: ConnectionManager,
    pub tab_manager: tab::QueryTabManager,
    pub tx: Sender<message::Message>,
    pub rx: Receiver<message::Message>,
    pub runtime: tokio::runtime::Runtime,
    pub connecting: bool,
    pub executing: bool,
    pub import_executing: bool,
    // Request IDs (private — accessed via methods)
    next_connect_request_id: u64,
    next_query_request_id: u64,
    next_metadata_request_id: u64,
    // Request tracking
    pub pending_connect_requests: HashMap<...>,
    pub pending_database_requests: HashMap<...>,
    // ... (30 fields total)
}
```

**Encapsulation:** Request ID fields are private (accessed via `next_*_request_id()` methods). Other fields are `pub` — no external crate can access them since `app` is `pub(crate)`. Full encapsulation is not a priority for a single-crate project.

## Core methods

- `connect(name)` — spawns async connect task, tracks request_id (on DbManagerApp, pending migration)
- `disconnect(name)` — clears SSH tunnels, removes pool entries (on DbManagerApp)
- `execute(sql)` — spawns async query task (on DbManagerApp)
- `cancel(request_id)` — sends cancel signal (on DbManagerApp)
- `ensure_active_tab()` — creates tab if none exists
- `active_sql()` / `set_active_sql(sql)` — read/write editor SQL
- `next_connect_request_id()`, `next_query_request_id()`, `next_metadata_request_id()`
- `refresh_connecting_flag()`, `refresh_executing_flag()`
- `track_query_task()`

## FrameEffects (defined, minimal wiring pending)

```rust
pub struct FrameEffects {
    pub queries: Vec<QueryResultEffect>,
    pub connections: Vec<ConnectionEffect>,
    pub metadata: Vec<MetadataEffect>,
    pub notifications: Vec<(NotifyLevel, String)>,
    pub repaint: bool,
}
```

Currently defined in `session/frame_effects.rs`. Not yet wired into the message handling pipeline. Will be connected one handler at a time, starting with the simplest (ImportDone).

## Message enum

13 variants in `session/message.rs`. 6 carry `request_id`, 6 are idempotent (no guard needed). All handlers have appropriate stale guards except documented safe cases (DatabaseDropped, TableDropped, ImportDone, ForeignKeysFetched).

## Request lifecycle

- `RequestIdCounter` generates monotonic IDs per category
- `PendingRequests` tracks in-flight operations
- Cancel: backend-specific (SQLite InterruptHandle, PG CancelToken, MySQL KILL QUERY)
- All taken from `DbManagerApp` via `self.session.xxx`

## Invariants

- `poll_messages()` (handle_messages) called once per frame, before rendering
- SSH tunnels stopped via `tokio::runtime::Handle::spawn()`, not `std::thread::spawn()`
- All `Mutex::lock()` calls use `unwrap_or_else(|e| e.into_inner())`
