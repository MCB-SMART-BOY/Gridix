# Core flows & system invariants

These features must not break during refactoring.

## Data flow (per frame, actual)

```
1. DbManagerApp::run_frame()
   ├─ reconcile_active_dialog_owner()
   ├─ handle_messages(&ctx)
   │   └─ while let Ok(msg) = self.session.rx.try_recv() { match msg { ... } }
   │       Handlers directly mutate self.session.* and self.* (State fields)
   ├─ handle_input_router(&ctx)
   ├─ render_dialogs(&ctx) → dialog_results
   ├─ handle_dialog_results(dialog_results)
   └─ CentralPanel rendering: sidebar, editor, grid, toolbar
```

## 9 core flows

| flow | entry | key state | risk |
|---|---|---|---|
| Start/Init | `bootstrap::run()` → `DbManagerApp::new()` | tokio runtime, config, fonts | medium |
| Connect/Disconnect | `app/runtime/database.rs` → `connect()`, `disconnect()` | Session.manager, pool, SSH | high |
| Tab create/switch/close | `Session.tab_manager` + `dock_tabs.rs::sync_all()` | QueryTabManager, GridWorkspaceStore | high |
| SQL edit/execute | `app/runtime/database.rs` → `execute()` | tab.sql, result, request_id | high |
| Query result display | `handler.rs` → `handle_query_done()` | result, active tab | high |
| Grid edit/save | `app/runtime/database.rs` → `execute_grid_save()` | DataGridState, modified_cells | high |
| Destructive actions | dialog → confirm → `confirm_pending_delete()` | pending_delete_target | high |
| Dialog lifecycle | `State::open_dialog()` / close / confirm / cancel | DialogId, active_dialog_owner | high |
| ER diagram | `app/runtime/er_diagram.rs` → `load_er_diagram_data()` | ERDiagramState, layout, viewport | medium |

## System invariants

| # | invariant | what it means |
|---|---|---|
| 1 | **Tab isolation** | Each tab has independent SQL, result. GridWorkspaceStore keyed by (tab_id, conn, db, table) |
| 2 | **One truth source** | `QueryTab.sql` is the sole authority for editor SQL. `self.sql` mirror eliminated |
| 3 | **Async boundary** | All async DB ops go through `self.session.runtime.spawn()`. State and UI are purely synchronous |
| 4 | **Stale response guard** | Every `Message` carries `request_id: u64`. Handlers compare against latest pending ID |
| 5 | **Dialog consistency** | `active_dialog_owner` reconciled at frame start. At most one dialog owns input |
| 6 | **Text entry priority** | Text entry always wins over command keys (`TextEntryGuard`) |
| 7 | **Layer dependency** | Lower layers never import from higher: types ← core ← data ← session ← state ← ui/app |
| 8 | **Session owns async** | `runtime`, `tx`, `rx` live in Session. DbManagerApp accesses via `self.session.rx.try_recv()` |
| 9 | **Config immutability** | AppConfig loaded at startup. Runtime mutations via Session fields, persisted via `save_config()` |
