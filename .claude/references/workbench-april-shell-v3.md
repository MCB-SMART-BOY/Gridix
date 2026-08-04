# Gridix Workbench April Shell v3

## Purpose

This is the corrected UI architecture after the sidebar toggle regression.

Use the April 2026 layout as the baseline because its mental model was simpler:

```text
TopBar
PrimarySidebar | DockWorkspace | RightInspector fallback
               | BottomPanel fallback
StatusBar
```

The mistake to avoid: making a simple TopBar sidebar button add/remove dock tabs. Visibility toggles must change visibility state only; dock mutations must be explicit surface actions.

## Stable Regions

- `TopBar`: global commands, connection/database context, run/create/action menus, sidebar/editor toggles.
- `PrimarySidebar`: default navigation/content region for Explorer, Filters, and Objects. This is the 4月-style left sidebar.
- `DockWorkspace`: user work surfaces: SQL, Results, TableData, ER, Inspector-as-tab, Help/Settings/History when explicitly docked.
- `BottomPanel fallback`: compatibility output region only when equivalent dock surface is absent.
- `RightInspector fallback`: compatibility inspector region only when equivalent dock surface is absent.
- `StatusBar`: quiet state line.

## Default Layout

Default runtime startup:

```text
TopBar
PrimarySidebar | Query results / data workspace | ER
               | SQL editor
StatusBar
```

## Canonical Default Ratio

The user-approved screenshot from 2026-06-19 is the default visual baseline.
Do not replace it with another imagined VS Code/Zed proportion unless the user
explicitly asks for a new default.

- Fixed `PrimarySidebar`: `PRIMARY_SIDEBAR_WIDTH = 280px`, roughly 17.5% on a 1600px window.
- Dock workspace horizontal split: center query/data workspace retains `DEFAULT_RIGHT_RETAIN_RATIO = 0.73`, right ER surface gets roughly 27%.
- Center vertical split: upper query/result region retains `DEFAULT_BOTTOM_RETAIN_RATIO = 0.69`, bottom SQL editor gets roughly 31%.
- Explicit left dock surfaces are not part of startup, but if opened they use the compact `DEFAULT_LEFT_RETAIN_RATIO = 0.79`.

Implementation rule:

- `default_surface_layout()` seeds Query Results center, SQL editor bottom, ER right.
- Explorer is not part of the default dock tree.
- RightInspector is not part of the default dock tree; it is revealed explicitly by inspect actions.
- Default ratios live in `src/ui/dock_tabs.rs` and must remain named constants, not anonymous split literals.
- `ToggleSidebar` and `SetPrimarySidebarVisible` only update fixed sidebar state/config/focus.
- `ensure_surface_tab()` is for explicit reveal/open actions, not ordinary expand/collapse.

## Navigation Surfaces

Explorer, Filters, and Objects still have `WorkbenchSurfaceKind` descriptors and may be docked explicitly in the future.

Until navigation state is split:

- Fixed PrimarySidebar owns the authoritative navigation state.
- Explicit docked navigation surfaces may reuse the Sidebar adapter, but must not mutate global activity state every frame.
- Do not create or destroy navigation dock tabs from the TopBar sidebar button.

## Why This Fixes The Regression

The prior attempt made the TopBar sidebar button remove/recreate the active navigation dock surface. That mixed a visibility action with dock tree mutation and could destabilize egui_dock state during the same frame.

The corrected model keeps:

- sidebar visibility as a stable shell concern
- dock tab creation as an explicit surface concern
- TopBar as the single global launcher
- ActivityBar/SurfaceRail dormant unless it returns later with a distinct optional role

## Next Implementation Order

1. Keep this boundary enforced by tests.
2. Split fixed PrimarySidebar state from explicit docked navigation surface state.
3. Move BottomPanel and RightInspector fallback chrome into reusable surface shell widgets.
4. Add layout presets after state ownership is clear.
5. Only then revisit optional movable SurfaceRail.
