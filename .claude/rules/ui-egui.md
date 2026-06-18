---
paths:
  - src/ui/**/*.rs
  - src/state/**/*.rs
---

# Gridix UI/egui rules

## Architecture (final state)

```
src/state/ (Layer 3) — UiState (~60 fields)
src/ui/    (Layer 4) — DbManagerApp (~11 fields), rendering, input routing
src/app/   (Layer 4) — input routing, surfaces, workflow, runtime
```

**DbManagerApp (~11 fields):**
```rust
pub struct DbManagerApp {
    session: Session,       // All DB + async state (~30 fields)
    state: UiState,         // All UI rendering state (~60 fields)
    app_config: AppConfig,  // Persisted configuration
    keybindings: KeyBindings, // Keyboard shortcuts
    // +7 cross-layer fields: dock_state, grid_workspaces, command_palette, etc.
}
```

**Migration: ~90 fields moved from DbManagerApp to Session/UiState.** Target achieved.

## egui_dock layout

`src/ui/dock_tabs.rs` — DockTab, WorkspaceViewer, sync_all(). `sync_all()` reads from `self.session.tab_manager`.

## Workbench shell

Current state: `UiState.workbench` and `src/ui/workbench/` exist, `render_top_bar()` renders the toolbar once globally, `WorkbenchActivityBar` drives PrimarySidebar activities, `WorkbenchBottomPanel` owns query results/messages, `WorkbenchRightInspector` owns contextual properties/schema/row/cell/ER/connection details, and EditorArea dock tabs use document/view semantics. Dockable Workbench v2 surface model and shared surface header/action shell are implemented. These fixed regions are now compatibility adapters; the target model is Dockable Workbench v2 peer surfaces.

Rules:
- Keep persisted config in `src/core/config.rs`, runtime state in `src/state/workbench.rs`, reusable widgets in `src/ui/workbench/`, and app-bound adapters in `src/app/surfaces/workbench.rs`.
- Until migration completes, keep legacy `FocusArea`, `show_sidebar`, and `sidebar_width` synchronized with `UiState.workbench`.
- Do not move BottomPanel and RightInspector behavior in the same slice unless tests cover the combined change.
- Do not render Toolbar inside `render_workspace_content()` or any dock tab; global TopBar rendering belongs in `DbManagerApp::render_top_bar()`.
- Activity switches must go through `AppAction::SetWorkbenchActivity`, not direct widget-side state mutation.
- BottomPanel tab/visibility changes must go through `AppAction::ToggleBottomPanel`, `AppAction::SetBottomPanelVisible`, or `AppAction::SetBottomPanelTab` when initiated by UI/commands.
- RightInspector tab/visibility changes must go through `AppAction::ToggleRightInspector`, `AppAction::SetRightInspectorVisible`, or `AppAction::SetRightInspectorTab` when initiated by UI/commands.
- Query success should reveal `BottomPanelTab::Results`; query failure should reveal `BottomPanelTab::Messages`.
- Schema/ER inspect actions should reveal the relevant RightInspector tab instead of opening a blocking detail dialog or replacing editor content.
- EditorArea dock tabs should use document/view roles: `SqlDocument`, `TableData`, `ErDiagram`, `SchemaObject`, `Welcome`, and `AuxPanel`. Do not reintroduce a standalone SQL scratchpad dock tab split from SQL documents.
- New surface kinds must add descriptor metadata with role, title, icon, description, command ID when available, singleton flag, default placement, allowed placements, persistence key, and tests for stable identity/tooltip behavior.
- While the transition bridge exists, keep `DockTab::surface_kind()` updated for every EditorArea tab variant.
- `DockTab::ui()` should render through `DbManagerApp::render_workbench_surface_in_ui()`; do not reintroduce per-tab content rendering in `DockTab::ui()`.
- Use `DockTab::Surface` for new dockable Workbench surfaces. Use `ensure_surface_tab()` when revealing a surface from commands/actions so repeated reveals are idempotent by `WorkbenchSurfaceId`.
- `default_surface_layout()` is the runtime startup seed for the dock workspace: Query Results/data workspace center, SQL editor bottom, ER right. Explorer/Filters/Objects stay in the stable PrimarySidebar by default and RightInspector is revealed only by inspect actions.
- Default dock proportions must use the named split constants in `src/ui/dock_tabs.rs`; do not add anonymous `0.xx` split ratios for workbench geometry.
- The default proportions are locked to the user-approved 2026-06-19 screenshot: fixed PrimarySidebar `280px`, query/ER center-right `0.73/0.27`, results/editor upper-bottom `0.69/0.31`, explicit left dock retain `0.79`. Do not change these defaults without an explicit user request.
- Sidebar visibility must be stable-shell behavior: `ToggleSidebar`/`SetPrimarySidebarVisible` should only flip fixed PrimarySidebar visibility/config/focus. Do not remove or recreate dock tabs from a simple sidebar expand/collapse button.
- Reveal/open paths for activity surfaces, BottomPanel tabs, RightInspector tabs, and ER must go through `DbManagerApp::reveal_workbench_surface()` or an equivalent `ensure_surface_tab()` adapter.
- Fixed-region compatibility adapters should set `WorkbenchFocus::Surface(...)` when the user focuses/clicks a surface body, while preserving legacy `FocusArea` keyboard behavior.
- Fixed-region fallback visibility must be computed at layout level from docked-equivalent surface existence. If the active Activity/BottomPanel/RightInspector surface is already in the dock tree, the fixed PrimarySidebar/BottomPanel/RightInspector fallback must not reserve space or render duplicate content.
- Explorer/Filters/Objects `DockTab::Surface` content must render real navigation content through the Sidebar adapter or future per-surface navigation renderer. Do not reintroduce placeholder-only navigation surfaces.
- Until navigation state is split, avoid mutating `WorkbenchState.active_activity` on every navigation surface render; commit shared sidebar state only when the surface is active, focused, clicked, or produces `SidebarActions`.
- Icon-only surface controls should use `SurfaceAction` plus `surface_icon_button()` or the same tooltip contract; no bare icon button without function name and shortcut/command metadata.
- Do not render ActivityBar/SurfaceRail beside TopBar in the default layout. If a rail returns later, it must have a distinct optional/movable launcher role and descriptor-driven icons through `surface_icon_glyph()` plus `WorkbenchSurfaceDescriptor::tooltip()`.
- Next expected slice: split navigation surface state, then migrate remaining fixed-region chrome into the shared surface shell before reducing Help/History/Settings dialogs.
- Do not add new permanent left/right/bottom content regions unless they are implemented as movable `WorkbenchSurface` items or explicitly marked as compatibility adapters.
- New surface UI must follow `references/gridix-ui-visual-system-v2.md`: shared surface header/body/footer anatomy, icon-first repeated chrome, tooltip with function name and shortcut/command metadata, and no redundant text labels in rails/toolbars.

## Dialog shells

4 contracts in `ui/components/dialogs/common.rs`: Blocking Modal, Form Dialog Shell, Workspace Dialog Shell, Utility Overlay.

## Borrow checker pattern

During rendering: `&self.session` (read-only) + `&mut self.state` (UI mutations). Disjoint fields — guaranteed safe by Rust.
