# Core flows & system invariants

From `docs/recovery/11-core-flows-and-invariants.md`. These are the features that must not break.

## 9 core flows

| flow | entry | key state | risk |
|---|---|---|---|
| Start/Init | `bootstrap::run()` → `DbManagerApp::new_with_loaded_config()` | tokio runtime, config, fonts | medium |
| Connect/Disconnect | `runtime/database.rs` → `connect()`, `disconnect()` | ConnectionManager, pool, SSH | high |
| Tab create/switch/close | `surfaces/render.rs` → tab actions | QueryTabManager, GridWorkspaceStore | high |
| SQL edit/execute | `runtime/database.rs` → `execute()` | sql, result, request_id | high |
| Query result display | `runtime/handler.rs` → `handle_query_done()` | result, active tab mirror | high |
| Grid edit/save | `runtime/database.rs` → `execute_grid_save()` | DataGridState, modified_cells, GridSaveContext | high |
| Destructive actions | `surfaces/dialogs.rs` → `confirm_pending_delete()` | pending_delete_target, confirm dialog | high |
| Dialog lifecycle | `dialogs/host.rs` → open/close/confirm/cancel | DialogId, active_dialog_owner | high |
| ER diagram | `runtime/er_diagram.rs` → `load_er_diagram_data()` | ERDiagramState, layout, viewport | medium |

## 7 system invariants

| # | invariant | what it means |
|---|---|---|
| 1 | **Tab isolation** | Each tab has independent SQL, result, grid state. `GridWorkspaceStore` keyed by `(tab_id, conn, db, table)`. |
| 2 | **Workspace isolation** | switching tabs persists/restores grid workspace. `persist_active_tab_state_for_navigation()` before switch. |
| 3 | **Active tab render consistency** | `self.result` mirrors active tab's result. `sync_from_active_tab()` called after query done + before render. |
| 4 | **Column header readability** | DataGrid column headers use theme-aware text colors. Verified: dark theme headers visible. |
| 5 | **Query error → surface** | Errors rendered as Welcome surface with message, not silent blank. |
| 6 | **Destructive action chain** | Delete requires: active connection → confirm dialog → `confirm_pending_delete()` → disconnect + cleanup. |
| 7 | **Dialog state consistency** | `active_dialog_owner` reconciled at frame start. At most one dialog owns input. |

## Rule

Any PR or change must name which flow is affected and which invariant must be preserved.
