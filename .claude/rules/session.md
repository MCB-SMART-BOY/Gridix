---
paths:
  - src/session/**/*.rs
---

# Gridix session rules

**Code is the source of truth.** Verify claims against `src/session/` before relying on them. Update this file when you change session code.

## Architecture

`Session` owns all async infrastructure and connection lifecycle. It is the bridge between the data layer (pure DB operations) and the state layer (UI state).

```
data/ (Layer 1) → Session (Layer 2) → FrameEffects → State (Layer 3) → ui/ (Layer 4)
```

## Session struct

```rust
pub struct Session {
    manager: ConnectionManager,
    runtime: tokio::runtime::Runtime,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    tab_manager: QueryTabManager,
    request_ids: RequestIdCounter,
    pending: PendingRequests,
    config: SessionConfig,
}
```

## Core methods

- `connect(name)` — spawns async connect task, tracks request_id
- `disconnect(name)` — clears SSH tunnels, removes pool entries, cancels queries
- `execute(sql)` — spawns async query task, returns request_id
- `cancel(request_id)` — sends cancel signal via backend-specific mechanism
- `poll_messages()` → `FrameEffects` — drains mpsc channel, dispatches to handlers, returns effects
- `ensure_active_tab()` — creates tab if none exists, returns `&mut QueryTab`
- `active_sql()` / `set_active_sql(sql)` — read/write editor SQL through tab manager

## FrameEffects

Defined in `session/frame_effects.rs`. `FrameEffects` 类型已实现；
`poll_messages()` 和 `State::apply_frame_effects()` 将在 handler 迁移后实现。

```rust
pub struct FrameEffects {
    pub queries: Vec<QueryResultEffect>,
    pub connections: Vec<ConnectionEffect>,
    pub metadata: Vec<MetadataEffect>,
    pub notifications: Vec<(NotifyLevel, String)>,
    pub repaint: bool,
}
```

当前 `handle_messages()` 仍在 `DbManagerApp` 上直接修改 State 和 Session。
目标是让 Session handler 返回 `FrameEffects`，由 State 统一应用。

## Message enum

13 variants, all carry `request_id: u64`:
- `ConnectedWithTables`, `ConnectedWithDatabases`
- `DatabaseSelected`, `DatabaseDropped`, `TableDropped`
- `QueryDone`
- `ImportDone`
- `PrimaryKeyFetched`
- `TriggersFetched`, `RoutinesFetched`
- `ForeignKeysFetched`
- `ERTableColumnsFetched`

## Request lifecycle

- `RequestIdCounter` generates monotonic IDs per category (connect, query, metadata)
- `PendingRequests` tracks in-flight operations with `HashMap<u64, JoinHandle/Task>`
- Stale response guard: handler compares message request_id against latest pending for that connection/tab
- Cancel: sends cancel signal, brief grace period, force-abort JoinHandle if needed

## Tab management

- `QueryTab` is pure data (id, title, sql, result, executing, modified, error, timing) — no UI dependency
- `QueryTabManager` holds `Vec<QueryTab>` + `active_index`, provides create/close/switch operations
- Tab rendering (tab bar widget) lives in `ui/components/query_tab_bar.rs`, reads from Session
- `self.sql` is eliminated — always accessed via `session.active_sql()` from the active tab

## Invariants

- `poll_messages()` is called once per frame, before rendering
- Session internal state (autocomplete, history, request tracking) is invisible to State
- SSH tunnels are stopped via `tokio::runtime::Handle::spawn()`, not `std::thread::spawn()`
- All `Mutex::lock()` calls use `unwrap_or_else(|e| e.into_inner())` to handle poison
