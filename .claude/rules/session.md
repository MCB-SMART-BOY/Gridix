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

**Note:** `FrameEffects` types are defined in `session/frame_effects.rs` but not yet wired into the message processing pipeline. `handle_messages()` still lives on `DbManagerApp`.

## Session struct (30 fields)

```rust
pub struct Session {
    // Connection
    pub manager: ConnectionManager,
    // Tab management
    pub tab_manager: QueryTabManager,
    // Async infrastructure
    pub tx: Sender<Message>,
    pub rx: Receiver<Message>,
    pub runtime: tokio::runtime::Runtime,
    // Execution state
    pub connecting: bool, pub executing: bool, pub import_executing: bool,
    // Request IDs
    pub next_connect_request_id: u64, pub next_query_request_id: u64,
    pub next_metadata_request_id: u64,
    // Request tracking (pending maps, query tasks, cancellers)
    // History
    pub query_history: QueryHistory,
    pub last_query_time_ms: Option<u64>,
    pub current_history_connection: Option<String>,
    // Command history
    pub command_history: Vec<String>, pub history_index: Option<usize>,
    // Autocomplete
    pub autocomplete: AutoComplete,
    // Notifications
    pub notifications: NotificationManager, pub progress: ProgressManager,
}
```

**⚠️ All fields are `pub` — there is no encapsulation.** Any code can bypass methods and directly mutate internal state. Fix: make fields `pub(crate)`, expose operations through methods.

## Core methods (implemented)

- `next_connect_request_id()` / `next_metadata_request_id()` — ID generation
- `refresh_connecting_flag()` / `refresh_executing_flag()` — state refresh
- `track_query_task()` — task registration
- `active_sql()` / `set_active_sql()` — SQL editor access
- `ensure_active_tab()` — tab lifecycle

## FrameEffects (defined, not wired)

`session/frame_effects.rs` defines: `FrameEffects`, `QueryResultEffect`, `ConnectionEffect`, `MetadataEffect`, `NotifyLevel`.

These types exist for future Session → State communication. Currently `handle_messages()` on `DbManagerApp` directly mutates both Session and State.

## Message enum

13 variants in `session/message.rs`, all carry `request_id: u64`. Re-exported from `app/runtime/message.rs` for backward compatibility.

## Request lifecycle

- `RequestIdCounter` → Session fields `next_*_request_id`
- `PendingRequests` → Session fields `pending_*`
- Stale response guard: handler compares request_id against latest pending
- Cancel: backend-specific (InterruptHandle/CancelToken/KILL QUERY)

## Tab management

- `QueryTab` is pure data (id, title, sql, result, executing, modified, error, timing)
- `QueryTabManager` holds `Vec<QueryTab>` + `active_index`
- Tab rendering (tab bar widget) lives in `ui/components/query_tabs.rs`
- `self.sql` is eliminated — always accessed via `session.active_sql()`

## Invariants

- `handle_messages()` is called once per frame, before rendering
- `self.sql` is eliminated — single source = tab_manager
- SSH tunnels are stopped via `tokio::runtime::Handle::spawn()`
- All `Mutex::lock()` calls use `unwrap_or_else(|e| e.into_inner())`
