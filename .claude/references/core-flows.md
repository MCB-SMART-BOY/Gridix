# Core flows & system invariants

These are the features that must not break during refactoring.

## 9 core flows

| flow | entry | key state | risk |
|---|---|---|---|
| Start/Init | `bootstrap::run()` → `DbManagerApp::new()` | tokio runtime, config, fonts | medium |
| Connect/Disconnect | `Session::connect()`, `Session::disconnect()` | ConnectionManager, pool, SSH | high |
| Tab create/switch/close | `Session::create_tab()`, `dock_tabs.rs::sync_all()` | QueryTabManager, GridWorkspaceStore | high |
| SQL edit/execute | `Session::execute()` | tab.sql, result, request_id | high |
| Query result display | `Session::poll_messages()` → `FrameEffects` → `State::apply_frame_effects()` | result, active tab | high |
| Grid edit/save | `Session` → data layer → result back via FrameEffects | DataGridState, modified_cells | high |
| Destructive actions | dialog result → confirm → session action | pending_delete_target, confirm dialog | high |
| Dialog lifecycle | `State::open_dialog()` / `close_dialog()` / confirm/cancel | DialogId, active_dialog_owner | high |
| ER diagram | `Session::load_er_diagram_data()` | ERDiagramState, layout, viewport | medium |

## Layer-level invariants

| # | invariant | what it means |
|---|---|---|
| 1 | **Tab isolation** | Each tab has independent SQL, result state. GridWorkspaceStore keyed by `(tab_id, conn, db, table)`. |
| 2 | **Workspace isolation** | Switching tabs persists/restores grid workspace. |
| 3 | **One truth source** | `QueryTab.sql` is the sole authority for editor SQL. `self.sql` mirror eliminated. |
| 4 | **Column header readability** | DataGrid column headers use theme-aware text colors. |
| 5 | **Query error → surface** | Errors rendered as Welcome surface with message, not silent blank. |
| 6 | **Destructive action chain** | Delete requires: active connection → confirm dialog → disconnect + cleanup. |
| 7 | **Dialog state consistency** | `active_dialog_owner` reconciled at frame start. At most one dialog owns input. |
| 8 | **Session isolation** | `poll_messages()` returns `FrameEffects`; handlers never directly mutate State. |
| 9 | **Async boundary** | All async DB operations go through `Session`. State and UI are purely synchronous. |
| 10 | **Layer dependency** | Lower layers never import from higher layers. `core/` ← `data/` ← `session/` ← `state/` ← `ui/`. |

## Data flow (per frame)

```
1. session.poll_messages()
   ├─ try_recv all pending Messages
   ├─ dispatch to handlers (update tab state, history, autocomplete)
   └─ return FrameEffects { queries, connections, metadata, notifications, repaint }

2. state.apply_frame_effects(effects)
   ├─ apply QueryResultEffect → update notifications
   ├─ apply ConnectionEffect → update sidebar
   ├─ apply MetadataEffect → update sidebar panels
   └─ request repaint if needed

3. ui.render(ctx)
   ├─ Sidebar (reads session.manager + state.sidebar)
   ├─ DataGrid (reads session.active_tab().result + state.grid)
   ├─ SqlEditor (reads session.active_tab().sql + state.editor)
   ├─ Dialogs (reads/writes state.dialogs)
   └─ Toolbar (reads/writes state.toolbar + session)
```
