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
| Tab create/switch/close | `Session.tab_manager` + `dock_tabs.rs::refresh_dock_from_session()` | QueryTabManager, GridWorkspaceStore | high |
| SQL edit/execute | `app/runtime/database.rs` → `execute()` | QueryTab SQL/result, `TaskRegistry`, `RuntimeEvent` | high |
| Query result display | runtime-event handler → `RuntimeOutcome::ExecutionFinished` | task identity, active-tab mirrors | high |
| Grid edit/save | `app/runtime/database.rs` → `execute_grid_save_typed()` | DataGridState, `MutationBatch`, modified cells | high |
| Destructive actions | dialog → confirm → `confirm_pending_delete()` | pending_delete_target | high |
| Dialog lifecycle | `DbManagerApp::open_dialog()` / close / confirm / cancel (`app/dialogs/host.rs`) | DialogId, active_dialog_owner | high |
| ER diagram | `app/runtime/er_diagram.rs` → `load_er_diagram_data()` | ERDiagramState, layout, viewport | medium |

## System invariants

| # | invariant | what it means |
|---|---|---|
| 1 | **Tab isolation** | Each tab has independent SQL and result; dock document identity uses stable `QueryTab.id` across main and floating surfaces after dragging/reordering/closing. PostgreSQL editor queries use an exclusive backend keyed by (connection ID, Tab document ID), with its own manual transaction; Tab/connection close drops the backend. Metadata/grid remain pooled. GridWorkspaceStore is keyed by (tab_id, connection name, db, table); submitted saves additionally carry `ConnectionId`, so a replaced same-name connection cannot retire new drafts. Saving locks only the owning workspace, confirmed commit retires its submitted edits, and that workspace remains locked until a post-commit refresh succeeds. Unknown COMMIT/ROLLBACK acknowledgment or MySQL incomplete-rollback warning quarantines the draft until the user checks database state and explicitly discards it. |
| 2 | **One truth source** | `QueryTab.sql` is the sole authority for editor SQL. `self.sql` mirror eliminated |
| 3 | **Async boundary** | All async DB ops go through `self.session.runtime.spawn()`. State and UI are purely synchronous |
| 4 | **Stale response guard** | `RuntimeEvent` carries `TaskId` + `OperationKey`; query completions also carry the submission request ID. `TaskRegistry::is_current()` plus live connection/document/request identity reject obsolete query results, even if the old task remains current for a previous connection key. Legacy pending-query handling is historical migration context. |
| 5 | **Dialog consistency** | `active_dialog_owner` reconciled at frame start. At most one dialog owns input |
| 6 | **Text entry priority** | Text entry always wins over command keys (`TextEntryGuard`) |
| 7 | **Layer dependency** | Lower layers never import from higher: types ← core ← data ← session ← state ← ui/app |
| 8 | **Session owns async** | `runtime`, `tx`, `rx` live in Session. DbManagerApp accesses via `self.session.rx.try_recv()` |
| 9 | **Config immutability** | AppConfig loaded at startup. Runtime mutations via Session fields, persisted via `save_config()` |
| 10 | **No zombie active** | A failed active connection must reset `manager.active=None` (`handle_connection_error`). A failed db-switch must clear the now-stale autocomplete/triggers/routines. |
| 11 | **Schema-change invalidation** | A successful DDL on the active connection must invalidate dependent views via `invalidate_after_schema_change`: table DDL → reload tables/autocomplete (+ ER if open); trigger/routine DDL → reload that sidebar panel. Detection is `analyze_sql_for_ui` → `SqlUiHints`. Reloads reuse existing primitives with their own stale-guards; table reload uses the quiet `ActiveTablesReloaded` path (no connect toast). |
| 12 | **Catalog context** | A switch-db catalog load must connect to the same target database it uses in the task key and completion. Every valid catalog result is cached by (connection ID, database), but metadata, autocomplete and an open ER diagram update only for the current active context. SQLite's catalog key is its configured file path, not `selected_database=None`. |
| 13 | **Grid identity transition** | Save and deactivate the current grid workspace before switching connection or selected database; old drafts and save locks never migrate to a same-named table in another database. On deletion, deactivate before removing the deleted database's workspaces. |
| 14 | **Result and deletion provenance** | `QueryTab.result_origin` binds a query result to the submitting connection ID and selected database. A tab opened on a different identity cannot project that result or save it as the new database's grid. Table/database deletion events carry their target identity and clear only matching workspaces, including the visible grid when it belongs to the deleted table. Reconnect refresh retains the selected database for PostgreSQL/MySQL. |
| 15 | **Grid baseline and active tables** | Dirty table grids retain their original row/primary-key baseline across attempted requery and late completions. A sidebar table query binds the Tab's table workspace; navigating away and back restores that draft. Deletion detaches old save snapshots so late outcomes cannot mutate a recreated object. `ActiveTablesReloaded` carries its source database and must be ignored after a database switch, even when the `ConnectionId` still matches. |
| 16 | **SSH owner isolation** | `ConnectionManager::add` assigns a runtime `ConnectionId` to each config. `ConnectionConfig::ssh_tunnel_name()` includes the connection name and instance ID in the shared key used by query setup, pool routing, and disconnect. Two logical connections through the same jump host cannot share a tunnel whose `stop()` aborts live forwarding tasks; a recreated connection cannot inherit the old tunnel or pool. |
