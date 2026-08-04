# egui_dock integration — COMPLETED v6.1.0

Status update 2026-06-18: this historical plan described the original dock integration. The current workbench model has since replaced the old query-output plus standalone SQL split with EditorArea document/view tabs: `SqlDocument`, `TableData`, `ErDiagram`, `SchemaObject`, `Welcome`, and `AuxPanel`. Use `references/project-refactor-execution-plan.md` and `references/workbench-ui-refactor-spec.md` for current execution.

## Implemented

- **Phase 1:** DockArea replaces manual layout in render.rs. `DockTab` (QueryData, SqlEditor, ErDiagram, AuxPanel). ~500 lines deleted, ~250 lines added.
- **Phase 2:** QueryTabBar visual UI removed (dock renders tabs natively). Keyboard shortcuts preserved via handle_keyboard().
- **Phase 3:** ER/SQL editor toggles (Ctrl+R/Ctrl+J) sync through dock_state. Tab close lifecycle: persist → cancel → cleanup workspace → close.

## Architecture

```
app/mod.rs              dock_state: DockState<DockTab>
ui/dock_tabs.rs         DockTab enum, WorkspaceViewer, sync_all(), layout ops
app/surfaces/render.rs  sync_all() called each frame before DockArea rendering
```

## Key design decisions

- `std::mem::replace` used to move dock_state out for borrow-safe rendering
- `sync_all()` synchronizes query tabs, ER/SQL visibility, and tab titles
- `on_dock_tab_close()` handles full cleanup lifecycle
- Sidebar, toolbar, and dialogs remain outside the dock

## Goal

Replace the manual layout in `render.rs` with `egui_dock`'s `DockArea`, while keeping
Sidebar, Toolbar, and Dialog systems unchanged. Only the main content area changes.

## Scope (phase 1 only)

### Files changed
| file | change | risk |
|---|---|---|
| `Cargo.toml` | +`egui_dock = "0.19"` | low |
| `src/ui/dock_tabs.rs` | NEW — `TabKind` enum + `GridixTabViewer` | low |
| `src/app/mod.rs` | `DbManagerApp` +1 field `dock_state` | low |
| `src/app/surfaces/render.rs` | Replace lines 285-800 with DockArea | **high** |
| `src/ui/mod.rs` | Re-export dock types | low |

### Files NOT changed (phase 1)
- `src/ui/components/query_tabs.rs` — kept, tabs rendered inside dock nodes
- `src/app/input/input_router.rs` — focus unchanged
- `src/app/runtime/handler.rs` — unchanged
- `src/ui/dialogs/` — unchanged
- Sidebar, toolbar — unchanged

## Architecture

### TabKind (new enum)
```rust
enum TabKind {
    QueryTab {
        tab_index: usize,
        title: String,
    },
    SqlEditor,
    ErDiagram,
}
```

### DockState layout
```
DockArea
├── [Split::Above]
│   └── [Tab: QueryTab(0), QueryTab(1), ...]  ← grid area
└── [Split::Below]
    └── [Tab: SqlEditor]                        ← SQL editor
```

When ER diagram is toggled ON: add a right split.
When ER diagram is toggled OFF: remove the right split.

### Focus management
FocusArea is unchanged. `FocusArea::DataGrid` and `FocusArea::SqlEditor` still work —
the `TabViewer::ui()` checks `self.app.focus_area` to determine if it's focused.

### Tab lifecycle
- `QueryTabManager` continues to own tab STATE (sql, result, error, timestamps).
- `DockState` manages tab LAYOUT (which tabs are visible, their order, split ratios).
- Bridge: `DockState::main_surface_mut()` returns the main split node. Tabs are added/removed
  by syncing `dock_state` with `tab_manager.tabs`.

## Logic audit

### Concern 1: Tab identity mismatch
**Risk:** `DockState` tabs are identified by `TabKind` enum. If two `QueryTab(0)` nodes
exist simultaneously (edge case during reorder), state updates to the wrong tab.

**Mitigation:** `DockState` guarantees unique tabs within a surface. Use `tab_index` as the
unique key — it maps 1:1 to `tab_manager.tabs[tab_index]`. When tabs are closed/reordered,
rebuild the dock tree from `tab_manager.tabs`.

### Concern 2: Stale request guard after tab switch
**Risk:** `handle_query_done()` checks `request_id` against the tab that issued the query.
If the dock layout changes tabs during a query, the stale guard still works because
it compares `request_id` to `tab.pending_request_id` — independent of layout.

**Mitigation:** No change needed. The stale guard is tab-state-based, not layout-based.

### Concern 3: Grid workspace isolation
**Risk:** `GridWorkspaceStore` is keyed by `(tab_id, conn, db, table)`. If dock tabs
change identity, workspace state could be orphaned.

**Mitigation:** `tab_id` is the `QueryTab.id` (UUID), not the index. The dock tab's
`tab_index` maps to the same UUID, so workspace identity is preserved across layout changes.

### Concern 4: Keyboard focus routing
**Risk:** `FocusArea::DataGrid` currently activates when the main grid area has focus.
With `DockArea`, focus is also shared with the SQL editor and ER diagram nodes.

**Mitigation:** Keep `FocusArea` unchanged. Tab cycle (Tab/Shift+Tab) moves between
`Sidebar → DataGrid → ErDiagram → SqlEditor`. The `DockArea` does not add new focus
areas — it only changes how the existing areas are laid out.

### Concern 5: egui_dock API compatibility
**Risk:** egui_dock 0.19 requires Rust 1.92 (Gridix has 1.96). API surface:
`DockArea::new()`, `show_inside()`, `DockState::new()`, `TabViewer` trait.
All are stable, well-documented APIs. No known breaking changes in 0.19.

**Mitigation:** Low risk. egui_dock is the most popular egui docking crate (194K downloads/month).

### Concern 6: Resize persistence
**Risk:** `DockState` split ratios are reset on app restart unless persisted.
Currently Gridix persists `sidebar_width`, `sql_editor_height`, `central_panel_ratio`.

**Mitigation:** `DockState` is serializable (behind `serde` feature). Persist alongside
session state in `~/.config/gridix/session.toml`.

## Implementation steps

### Step 1: Add dependency
```bash
cargo add egui_dock@0.19
```

### Step 2: Create src/ui/dock_tabs.rs
- Define `TabKind` enum
- Implement `TabViewer` trait
- Each `ui()` call renders the appropriate widget

### Step 3: Add dock_state to DbManagerApp
```rust
pub dock_state: egui_dock::DockState<TabKind>,
```
Initialize with a default layout: QueryTab(0) above, SqlEditor below.

### Step 4: Sync dock tabs with QueryTabManager
Before rendering, sync:
- New tabs in tab_manager → add to dock_state
- Closed tabs in tab_manager → remove from dock_state
- Active tab change → focus the dock tab

### Step 5: Replace render.rs layout
Replace lines 285-800 (the `allocate_ui_with_layout` main content block) with:
```rust
egui_dock::DockArea::new(&mut self.dock_state)
    .show_inside(ui, &mut GridixTabViewer { app: self, ctx: &ctx });
```

### Step 6: Handle ER diagram toggle
When `show_er_diagram` changes:
- ON: add a right split to the main surface, insert ErDiagram tab
- OFF: remove the right split

### Step 7: Handle SQL editor toggle
When `show_sql_editor` changes:
- ON: add a bottom split with SqlEditor tab
- OFF: remove the bottom split

### Step 8: Test and verify
- `cargo test` — all existing tests must pass
- Manual: tab switching, resize, ER toggle, editor toggle, query execution

## Rollback plan

If integration fails:
1. Revert the 5 changed files
2. Remove `egui_dock` from Cargo.toml
3. All existing functionality is preserved in the reverted code
