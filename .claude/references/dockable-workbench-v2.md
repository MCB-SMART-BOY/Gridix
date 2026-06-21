# Gridix Dockable Workbench v2

## Purpose

This document supersedes the fixed-region interpretation of the Workbench UI.

Visual and interaction rules live in `gridix-ui-visual-system-v2.md`. This file defines layout architecture; the visual-system file defines simplicity, icon usage, tooltips, panel anatomy, density, and feature-specific panel controls.

The previous migration created useful pieces: global TopBar, ActivityBar, PrimarySidebar, BottomPanel, EditorArea dock tabs, and RightInspector. That was a safe compatibility bridge, but it is not the final UI model. The final model should treat most workspace surfaces as peer dock items that can be moved, split, grouped, hidden, or restored.

## Implementation Status

Last updated: 2026-06-19.

- Phase A design pivot is complete: fixed ActivityBar/PrimarySidebar/BottomPanel/RightInspector are compatibility adapters, not the final model.
- Phase B surface foundation is implemented in `src/state/workbench.rs`: `WorkbenchSurfaceKind`, `WorkbenchSurfaceRole`, `WorkbenchPlacement`, `WorkbenchSurfaceId`, descriptors, stable identity keys, default/allowed placements, icon keys, and command/tooltip metadata.
- `src/ui/dock_tabs.rs` now exposes `DockTab::surface_kind()` as a transition bridge from legacy editor-area tabs to Workbench surfaces.
- `src/ui/workbench/surface.rs` provides the shared `WorkbenchSurfaceHeader`, `SurfaceAction`, icon-only button helper, and tooltip contract foundation. BottomPanel and RightInspector close buttons already use it.
- Phase C bridge has started: `WorkbenchFocus::Surface`, activity/bottom/inspector-to-surface mappings, `Explain` as an output surface, unified `DbManagerApp::render_workbench_surface_in_ui()`, and `DockTab::ui()` routed through that renderer are implemented.
- Phase C seed layout is implemented with the April-shell correction: `DockTab::Surface` can carry `WorkbenchSurfaceKind` directly, `default_surface_layout()` seeds Query Results/data workspace center / SQL editor bottom / ER right, and Explorer remains in the stable PrimarySidebar by default. `ensure_surface_tab()` inserts explicit surfaces by stable identity without duplicates.
- Phase C action wiring is implemented: Activity surfaces, BottomPanel surfaces, RightInspector surfaces, and ER reveal/open/focus paths route through `ensure_surface_tab()` via `DbManagerApp::reveal_workbench_surface()` while fixed regions remain fallback adapters.
- Phase C fallback de-duplication is implemented: fixed PrimarySidebar, BottomPanel, and RightInspector fallback regions hide at layout level when their active equivalent docked surface exists.
- Phase C runtime seed migration is implemented: app startup and render-time fallback dock creation use the `default_surface_layout()` surface seed.
- Phase C navigation surface activation is implemented: Explorer, Filters, and Objects dock surfaces render real Sidebar content through a compatibility adapter instead of placeholder text.
- Phase C visual/layout calibration is implemented: default dock split ratios are named in `src/ui/dock_tabs.rs` and locked to the user-approved 2026-06-19 April-shell screenshot (`280px` fixed PrimarySidebar, query/ER `0.73/0.27`, results/editor `0.69/0.31`); the duplicate left ActivityBar/SurfaceRail is not rendered by default, and TopBar sidebar visibility controls only the stable PrimarySidebar.
- Next phase is navigation state split + fixed-chrome reduction: give navigation surfaces independent state, then migrate remaining fixed-region headers/tabs/controls into the shared surface shell while preserving compatibility fallback behavior.

## Reference Direction

VS Code and Zed both separate semantic workspace content from default placement:

- VS Code lets users move views and panels between side bars and panel regions, and keeps editor groups independent from workbench chrome.
- Zed moved toward left/right/bottom docks where panels can change dock location, while panes/items remain the primary unit of work.

Gridix should go further for database work: Explorer, filters, objects, result tables, schema detail, ER diagrams, history, and inspector content should all be surfaces with default placements, not hard-coded permanent regions.

## Core Decision

Gridix should have only three truly stable chrome regions:

```text
TopBar
Dockable Workspace
StatusBar
```

Everything inside `Dockable Workspace` is a `WorkbenchSurface`.

The default experience should remain simple. Dockability is power, not visual noise. Do not show every surface by default.

Default layout can still look familiar:

```text
Explorer | SQL Document + ER + Table Data | Inspector
         | Results / Messages / History   |
```

But Explorer, Results, Inspector, Filters, Objects, History, and Settings must be movable. Their current left/right/bottom placement is a default, not a type-level truth.

## Vocabulary

### WorkbenchSurface

A surface is a UI item that can appear in a dock tab or split pane.

Examples:

- `SqlDocument`
- `QueryResult`
- `Explain`
- `TableData`
- `ErDiagram`
- `SchemaObject`
- `Explorer`
- `Filters`
- `Objects`
- `History`
- `Messages`
- `Tasks`
- `Inspector`
- `Settings`
- `Help`
- `Welcome`

### Surface Role

Role describes behavior, not physical placement:

```rust
pub enum WorkbenchSurfaceRole {
    Document,
    Data,
    Navigation,
    Output,
    Inspector,
    Utility,
}
```

### Surface Placement

Placement is a preference and persistence concern:

```rust
pub enum WorkbenchPlacement {
    Center,
    Left,
    Right,
    Bottom,
}
```

Placement should not decide state ownership.

`Floating` is a future placement. Current implementation supports `Center`, `Left`, `Right`, and `Bottom`.

### Surface Descriptor

Each surface should have metadata:

```rust
pub struct WorkbenchSurfaceDescriptor {
    pub kind: WorkbenchSurfaceKind,
    pub role: WorkbenchSurfaceRole,
    pub title: String,
    pub icon: &'static str,
    pub description: &'static str,
    pub command_id: Option<&'static str>,
    pub singleton: bool,
    pub default_placement: WorkbenchPlacement,
    pub allowed_placements: &'static [WorkbenchPlacement],
    pub persistence_key: String,
}
```

## Architecture Rules

1. Layout owns geometry, tab grouping, and placement only.
2. Existing domain state remains authoritative:
   - SQL text remains in `QueryTabManager`.
   - Query results remain in tab/session state until a dedicated result store exists.
   - Grid edit state remains in `GridWorkspaceStore`.
   - ER state remains in `UiState.er_diagram_state`.
   - Connection state remains in `Session.manager`.
3. Surfaces render through app-bound adapters and dispatch `AppAction`.
4. TopBar is global command/context chrome only. It must not duplicate Explorer, Filters, Objects, History, Settings, or Inspector as permanent right/left regions.
5. Surface launchers are not content owners. A future SurfaceBar may reveal or focus surfaces, but the surface content lives in the dock tree.
6. Transactional flows can stay dialogs:
   - connect/edit connection
   - create table/database/user
   - destructive confirmation
   - import/export wizard until panel UX is proven
7. Non-transactional flows should become dockable surfaces:
   - history
   - help
   - settings/keybindings overview
   - schema detail
   - messages/tasks
8. All dockable surfaces should share the same surface shell anatomy before more panels are migrated.
9. Repeated chrome should be icon-first with tooltip and command/shortcut metadata.

## Corrected UI Model

### TopBar

TopBar should include:

- command palette
- active connection/database breadcrumb
- run/cancel state
- create/action menus
- layout preset selector
- theme/density controls

TopBar should not permanently host Explorer/Filters/Objects/History/Settings content. It may expose commands to reveal those surfaces.

### SurfaceBar

The current ActivityBar should eventually become a compact SurfaceBar:

- It shows shortcuts to important surfaces.
- It can be hidden or moved.
- Clicking an item focuses or reveals a surface.
- It does not define where the surface must live.

This fixes the current duplication problem: Explorer/Filters/Objects/History/Settings are surface types, not separate top/right/side feature copies.

### Explorer / Sidebar

Explorer is not "the sidebar"; it is a `Navigation` surface.

Default:
- left split
- width around 280 px

Allowed:
- left
- right
- bottom
- center tab

If Explorer is dragged into the center, it behaves like a document-style tab with navigation content. If it is dragged right, it is still Explorer, not RightInspector.

### Filters / Objects / History / Settings

These should be separate singleton surfaces:

- `Filters`: can dock beside Results or Explorer.
- `Objects`: can dock beside Explorer or center.
- `History`: can dock bottom, side, or center.
- `Settings`: can dock center or side.

They should not be hard-wired to one PrimarySidebar activity.

### Results And Tables

Tables/results need the same flexibility as editor tabs.

Surface kinds:

- `QueryResult { query_tab_id, result_id }`
- `TableData { connection, database, table }`
- `Messages`
- `Explain`

Default:
- query result opens below the active SQL document
- table data may open as center tab or bottom split depending on user setting

Allowed:
- center
- bottom
- right
- left

This makes "table area can be dragged anywhere" architecturally true.

### Inspector

Inspector should stop being a fixed right region.

It should become:

```rust
WorkbenchSurfaceKind::Inspector { mode: InspectorMode }
```

Modes:
- properties
- schema
- row
- cell
- ER selection
- connection

Default placement is right, but the user can dock it elsewhere or group it with Explorer/Filters.

## Layout Persistence

The long-term persisted layout should store:

```rust
pub struct WorkbenchLayoutV2 {
    pub schema_version: u32,
    pub dock_tree: WorkbenchDockTree,
    pub hidden_surfaces: Vec<WorkbenchSurfaceId>,
    pub surface_preferences: HashMap<WorkbenchSurfaceKind, SurfacePreference>,
}
```

Avoid persisting raw runtime state. Persist layout identity and placement only.

If `egui_dock` serialization is used, wrap it behind an internal versioned adapter so Gridix can recover from incompatible layouts.

## Implementation Route

### Phase A: Design Pivot

Goal: stop adding fixed panel concepts.

Tasks:
- Add this document.
- Mark fixed ActivityBar/PrimarySidebar/BottomPanel/RightInspector as compatibility bridge.
- Do not start Help/History dialog reduction until the surface model is chosen.

Acceptance:
- `.codex` docs describe dockable surfaces as the target model.

### Phase B: Surface Model Types

Goal: add logical surface metadata without changing layout behavior.

Files:
- `src/state/workbench.rs`
- `src/ui/dock_tabs.rs`
- `src/ui/workbench/surface.rs`

Tasks:
- Add `WorkbenchSurfaceKind`.
- Add `WorkbenchSurfaceRole`.
- Add `WorkbenchPlacement`.
- Add `WorkbenchSurfaceId`.
- Add a registry function that maps surface kind to descriptor.
- Add a shared surface header/action shell for icon-first controls.

Acceptance:
- Completed 2026-06-18.
- Existing UI compiles and behaves the same except compatible icon-only close controls now use richer tooltips.
- Tests cover descriptor defaults, allowed placements, stable IDs, tooltip metadata, and DockTab surface bridge.

### Phase C: One Dock Tree For Workspace

Goal: make the workspace dock tree own Explorer, SQL, Results, Inspector, and ER as peer surfaces.

Status:
- Bridge started 2026-06-18.
- Completed bridge pieces: surface focus, legacy region-to-surface mapping, unified surface renderer, DockTab rendering through surface kind, direct `DockTab::Surface`, default surface layout seeding, and idempotent surface insertion.
- Completed fallback de-duplication: fixed left/bottom/right fallback regions stop reserving layout space when the active equivalent surface is present in the dock tree.
- Completed runtime seed switch: new app state and render-time fallback dock creation use `default_surface_layout()`.
- Completed navigation surface activation: Explorer/Filters/Objects surfaces render real Sidebar content through the compatibility adapter.
- Completed visual/layout calibration: named dock split constants keep fixed Explorer compact, Results/data dominant in the center, SQL editor below, and ER secondary on the right; default runtime layout no longer reserves duplicate ActivityBar width.
- Completed April-shell correction: `ToggleSidebar` and `SetPrimarySidebarVisible` no longer mutate the dock tree; default Explorer/Filters/Objects content belongs to the stable PrimarySidebar, while explicit docked navigation surfaces remain possible through `ensure_surface_tab()`.
- Remaining work: fixed PrimarySidebar and explicit docked navigation surfaces still share one legacy `SidebarPanelState`; fixed BottomPanel/RightInspector chrome still exists as compatibility adapters and should be reduced or migrated into shared surface shell controls.

Tasks:
- Add a dock-tab type or adapter that stores `WorkbenchSurfaceKind` directly. Completed 2026-06-18.
- Replace fixed central `EditorArea + BottomPanel + RightInspector` composition with one `DockState<WorkbenchSurface>` as fixed chrome is reduced.
- Keep TopBar and StatusBar outside the dock.
- Seed default layout with fixed left PrimarySidebar, center Query Results/data workspace, bottom SQL editor, and right ER.
- Preserve current render adapters.

Acceptance:
- Explorer can be dock tab content.
- Results can be split beside SQL.
- Inspector can be moved/grouped by dock layout.
- Query execution still routes to a result surface.

### Phase D: Table And Result Identity

Goal: make table/result surfaces first-class.

Tasks:
- Add stable identity for `TableData`.
- Add stable identity for `QueryResult`.
- Stop treating DataGrid as only BottomPanel content.
- Let table open mode decide center tab vs current result surface.

Acceptance:
- Multiple table data surfaces can coexist.
- Query result surface remains bound to the SQL document that produced it.
- Grid edits remain isolated by `GridWorkspaceStore`.

### Phase E: SurfaceBar Instead Of ActivityBar

Goal: remove duplicate fixed activities.

Tasks:
- Rename/adapt ActivityBar concept to SurfaceBar.
- SurfaceBar reveals/focuses surfaces, not sidebar activities.
- Remove `WorkbenchActivity` as the primary navigation model after adapters are stable.

Acceptance:
- Explorer/Filters/Objects/History/Settings can all be launched from SurfaceBar or command palette.
- Moving a surface does not change its identity.

### Phase F: Dialog Reduction On The New Model

Goal: move non-transactional dialogs into dockable surfaces.

Tasks:
- History surface.
- Help surface.
- Settings/Keybindings overview surface.

Acceptance:
- Help/history no longer obscure the workspace by default.
- Transactional dialogs remain modal.

## Risks

- `egui_dock` may not support every desired cross-region drag affordance out of the box.
- Persisting full dock layout can be brittle if serialized directly.
- Current `FocusArea` is fixed-region-oriented and will need replacement with surface-focused routing.
- Query result identity must be handled carefully to avoid mixing result state across SQL tabs.

## Non-Negotiable Invariants

- Layout refactor must not change DB execution semantics.
- Layout must not own database/session state.
- Secret fields must never be rendered in dockable inspector/settings surfaces.
- Every surface action must go through `AppAction` or an explicitly documented adapter.
- Migration must be incremental and reversible.
