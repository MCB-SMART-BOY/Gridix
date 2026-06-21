# Gridix Project Refactor Execution Plan

## Purpose

This is the project-wide refactor route for Gridix. It is written so an agent or engineer can execute it phase by phase without needing to redesign the plan.

The workbench UI redesign is only one track. The full refactor also covers configuration, app shell, action/input routing, oversized files, data/session boundaries, tests, documentation, and release discipline.

## Ground Rules

1. No big-bang rewrites. Every phase must compile and preserve current workflows.
2. One PR must have one dominant purpose.
3. Keep the 6-layer dependency rule: `types ← core ← data ← session ← state ← ui/app`.
4. Preserve core invariants from `~/.codex/references/core-flows.md`.
5. Run format/lint/tests before each merge.
6. Prefer adapters and compatibility bridges before deleting old code.
7. Update `~/.codex/` workflow docs when architecture, UI layout, config, or rules change.
8. Do not move database behavior and UI behavior in the same PR unless the feature requires both.

## Current Baseline

Package:
- `gridix` v6.1.0
- Targets: lib, `gridix`, `gridix-driver`, `check-doc-links`
- Test targets: `core_tests`, `database_tests`, `ddl_tests`, `edge_regression_tests`, `export_tests`, `grid_tests`, `mysql_cancel_integration`, `ssh_tests`, `ui_dialogs_tests`

Approximate code size:
- `src + tests`: ~69k lines
- `tests`: ~1.5k lines

Largest files to reduce:

| file | lines | refactor concern |
|---|---:|---|
| `src/ui/dialogs/keybindings_dialog.rs` | 3560 | mixed state, rendering, diagnostics, editing workflow |
| `src/app/input/input_router.rs` | 3393 | oversized routing table, focus logic, tests mixed together |
| `src/core/keybindings.rs` | 2385 | parsing, conflict diagnostics, persistence, defaults in one file |
| `src/ui/components/er_diagram/layout.rs` | 2179 | many algorithms and tests in one file |
| `src/ui/components/er_diagram/render.rs` | 2147 | rendering, interaction, helpers, tests |
| `src/ui/components/grid/keyboard.rs` | 1953 | keyboard state machine and commands in one file |
| `src/app/runtime/handler.rs` | 1589 | message dispatch and many handlers/tests |
| `src/app/action/action_system.rs` | 1573 | action enum, command registry, availability, reducer, tests |
| `src/ui/panels/sidebar/mod.rs` | 1541 | sidebar workflow, layout, keyboard, rendering glue |
| `src/app/surfaces/render.rs` | 1534 | frame orchestration, workbench layout, toolbar/grid/editor glue |

Existing high-risk flows:
- connect/disconnect
- tab create/switch/close
- SQL edit/execute
- query result display
- grid edit/save
- destructive actions
- dialog lifecycle
- ER diagram loading/layout

## Target Architecture

Keep the six layers, but reduce oversized Layer 4 files and introduce stronger feature boundaries.

```text
src/types.rs                 Layer -1
src/core/                    Layer 0
src/data/                    Layer 1
src/session/                 Layer 2
src/state/                   Layer 3
src/app/ + src/ui/           Layer 4
```

Target top-level structure after refactor:

```text
src/app/
  action/
    registry.rs
    availability.rs
    reducer.rs
    command_palette.rs
  dialogs/
  input/
    context.rs
    router.rs
    focus.rs
    text_entry.rs
    global.rs
    workspace.rs
    dialog.rs
    tests.rs
  runtime/
    database.rs
    handler/
      mod.rs
      connection.rs
      query.rs
      grid.rs
      metadata.rs
      er.rs
      tests.rs
  surfaces/
    frame.rs
    workbench.rs
    dialogs.rs
    preferences.rs
  workflow/

src/state/
  mod.rs
  workbench.rs
  dialogs.rs
  grid.rs

src/ui/
  workbench/
  components/
  dialogs/
  panels/
  dock_tabs.rs
  styles.rs

src/core/
  keybindings/
    mod.rs
    binding.rs
    parser.rs
    registry.rs
    diagnostics.rs
    persistence.rs
    defaults.rs
    tests.rs
```

Do not force this final file tree in one PR. Use it as the destination.

## Execution Overview

Recommended order:

```text
Phase 0  Baseline and safety
Phase 1  Workbench config foundation
Phase 2  Workbench state and shell foundation
Phase 3  Global TopBar
Phase 4  ActivityBar and sidebar activities
Phase 5  BottomPanel and result/message routing
Phase 6  EditorArea dock tab semantics
Phase 7  RightInspector
Phase 8  Dialog reduction
Phase 9  Action system split
Phase 10 Input router split
Phase 11 Keybindings core split
Phase 12 Runtime handler split
Phase 13 Data/query cleanup and test expansion
Phase 14 ER diagram and grid module cleanup
Phase 15 Visual tokens and polish
```

## Execution Status

Last updated: 2026-06-19.

Completed:
- Phase 0 baseline and safety: format, clippy, full tests, and doc links pass after restoring missing default keybindings and fixing SQLite metadata tests to use a shared temporary database file instead of separate `:memory:` connections.
- Phase 1 workbench config foundation: `AppConfig.workbench` is additive, saved config version is 3, legacy `sidebar.edge_transfer` is preserved for compatibility, dimensions normalize/clamp, and config tests cover empty/missing/invalid workbench config cases.
- Phase 2 workbench state and shell foundation: `UiState.workbench` is seeded from config, `WorkbenchFocus` bridges legacy `FocusArea`, current layout is wrapped by `WorkbenchShell`, and StatusBar content is sourced from existing action/status context.
- Phase 3 global TopBar: toolbar rendering moved out of `QueryData` dock content into `DbManagerApp::render_top_bar()`, TopBar is rendered once above sidebar/editor layout, duplicate QueryData separators were removed, and toolbar keyboard activation/focus transfer remains covered.
- Phase 4 ActivityBar and sidebar activities: `WorkbenchActivityBar` renders left of PrimarySidebar, `SetWorkbenchActivity`/`TogglePrimarySidebar`/`SetPrimarySidebarVisible` are action-routed, activities map to legacy sidebar panel groups or placeholders, and filter workspace opens the Filters activity.
- Phase 5 BottomPanel result/message routing: `WorkbenchBottomPanel` renders below EditorArea, query results route to Results, query errors/messages route to Messages, `ToggleBottomPanel`/`SetBottomPanelVisible`/`SetBottomPanelTab` are action-routed, and BottomPanel tab/visibility/height persist through workbench config.
- Phase 6 EditorArea dock tab semantics: the legacy query-output tab plus standalone SQL editor split was replaced by `SqlDocument` plus target `TableData`/`SchemaObject`/`Welcome` variants, SQL renders as the primary editor document, ER remains an EditorArea view, and stale tab removal now uses `egui_dock::DockState::retain_tabs()` instead of deleting whole leaves.
- Phase 7 RightInspector: `WorkbenchRightInspector` renders as a fixed right workbench region, `ToggleRightInspector`/`SetRightInspectorVisible`/`SetRightInspectorTab` are action-routed, schema inspection opens the Schema tab, ER selection opens the ER tab, Row/Cell views read current result selection, and visibility/tab/width persist through workbench config.
- Dockable Workbench v2 Phase B foundation: `WorkbenchSurfaceKind`, `WorkbenchSurfaceRole`, `WorkbenchPlacement`, `WorkbenchSurfaceId`, descriptor metadata, tooltip contract, and `DockTab::surface_kind()` bridge are implemented. `src/ui/workbench/surface.rs` provides shared `WorkbenchSurfaceHeader` and `SurfaceAction`; BottomPanel and RightInspector close actions use the icon-only tooltip contract.
- Dockable Workbench v2 Phase C bridge: `WorkbenchFocus::Surface` is available, legacy Activity/BottomPanel/RightInspector map to surface kinds, `Explain` is a first-class output surface, fixed BottomPanel/RightInspector/PrimarySidebar clicks set surface focus, and `DockTab::ui()` delegates rendering through `DbManagerApp::render_workbench_surface_in_ui()`.
- Dockable Workbench v2 Phase C seed layout: `DockTab::Surface` can carry `WorkbenchSurfaceKind` directly, `default_surface_layout()` seeds Query Results/data workspace center / SQL editor bottom / ER right while Explorer stays in the stable 4月式 PrimarySidebar by default, and `ensure_surface_tab()` inserts explicit surfaces idempotently by stable surface identity. SQL sync now supports both legacy SQL tabs and surface SQL tabs, and the missing-tab path now focuses the target leaf before pushing.
- Dockable Workbench v2 Phase C action wiring: `set_workbench_activity()`, BottomPanel tab/visible/query reveal, RightInspector tab/visible/inspect reveal, and ER open/focus now route through `ensure_surface_tab()` via `DbManagerApp::reveal_workbench_surface()` while preserving fixed-region fallback state.
- Dockable Workbench v2 Phase C fallback de-duplication: fixed PrimarySidebar, BottomPanel, and RightInspector fallback regions now hide at layout level when their active equivalent surface exists in the dock tree; helper tests cover docked-equivalent detection.
- Dockable Workbench v2 Phase C runtime seed switch: `DbManagerApp` now initializes `dock_state` from `default_surface_layout()`, and the render-time borrow-replacement fallback uses the same surface seed via `default_workbench_surface_layout()`.
- Dockable Workbench v2 Phase C navigation surface activation: Explorer, Filters, and Objects dock surfaces now render real Sidebar content through a compatibility adapter instead of placeholder text, with tests covering surface-to-activity and panel visibility mapping.
- Dockable Workbench v2 Phase C visual/layout calibration: default dock split ratios are named in `src/ui/dock_tabs.rs`, tuned to the user-approved 2026-06-19 April-shell screenshot (`280px` fixed PrimarySidebar, query/ER `0.73/0.27`, results/editor `0.69/0.31`), and the ActivityBar widget is dormant and not rendered in the default runtime layout.
- Dockable Workbench v2 Phase C April-shell correction: default runtime layout no longer reserves or renders the duplicate left ActivityBar/SurfaceRail; `ToggleSidebar`/`SetPrimarySidebarVisible` control only the stable fixed PrimarySidebar and must not mutate the dock tree.
- ER diagram visual redesign: `src/ui/components/er_diagram/render.rs` now uses schema-canvas styling, object-card table rendering, PK/FK badges, key-row highlighting, relation halos/endpoints/cardinality pills, and themed empty/loading cards while preserving existing layout/state/keyboard behavior.

Current next phase:
- Dockable Workbench v2 Phase C next slice before continuing Phase 8 Dialog Reduction. Split stable PrimarySidebar state from explicitly docked navigation surface state, migrate remaining fixed-region chrome into the shared surface shell, keep TopBar as the primary global launcher, and continue replacing legacy `FocusArea` assumptions with surface-first focus/config routing.

Design pivot:
- Reference: `~/.codex/references/dockable-workbench-v2.md`.
- Visual system reference: `~/.codex/references/gridix-ui-visual-system-v2.md`.
- Explorer/Filters/Objects/History/Settings/Results/Tables/Inspector should become movable `WorkbenchSurface` items.
- TopBar remains global command/context chrome and should not duplicate content surfaces.
- The next implementation slice should split navigation surface state and reduce fixed-region chrome/fallback adapter code, while preserving legacy `FocusArea` keyboard paths until surface focus routing is complete.

## Phase 0: Baseline And Safety

Goal: create a stable baseline before refactor.

Files:
- no production file changes unless needed for tests/docs
- `~/.codex/references/project-refactor-execution-plan.md`
- `~/.codex/references/tech-debt.md`
- `~/.codex/references/roadmap.md`

Commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin check-doc-links
```

If format fails:

```bash
cargo fmt
cargo fmt --check
```

If full clippy/test is too slow during local iteration:

```bash
cargo check
cargo test -p gridix --lib
```

Required audit commands:

```bash
find src tests -type f -name '*.rs' -print | sort | xargs -r wc -l | sort -nr | sed -n '1,80p'
grep -RIn "TODO\\|FIXME\\|HACK" src tests || true
```

Acceptance:
- Completed 2026-06-18.
- Baseline commands pass.
- Current file size table is known.
- No feature behavior changes beyond restoring documented default keybindings and correcting SQLite metadata tests.

Suggested commit:

```text
docs(refactor): add project execution plan
```

## Phase 1: Workbench Config Foundation

Goal: add persisted configuration for the future workbench without changing UI behavior.

Primary reference:
- `~/.codex/references/workbench-ui-refactor-spec.md`

Files:
- `src/core/config.rs`
- `src/core/constants.rs`
- `tests/core_tests.rs` or new config tests
- `~/.codex/references/workbench-ui-refactor-spec.md`

Steps:

1. Add config enums:
   - `WorkbenchActivity`
   - `WorkbenchDensity`
   - `BottomPanelTab`
   - `RightInspectorTab`
   - `TableOpenMode`
   - `ResultPlacement`
2. Add config structs:
   - `WorkbenchConfig`
   - `PrimarySidebarConfig`
   - `BottomPanelConfig`
   - `RightInspectorConfig`
   - `EditorAreaConfig`
   - `StatusBarConfig`
   - `WorkbenchBehaviorConfig`
3. Add `#[serde(default)] pub workbench: WorkbenchConfig` to `AppConfig`.
4. Add default functions and dimension clamps.
5. Preserve legacy `sidebar.edge_transfer`.
6. Bump saved config version to 3 only if this is accepted as a config migration.

Do not:
- Render new UI.
- Move toolbar/sidebar.
- Change shortcuts.

Tests:
- Empty config deserializes to defaults.
- Missing `[workbench]` deserializes to defaults.
- Invalid dimensions clamp.
- `workbench.sidebar.edge_transfer` default matches legacy behavior.

Commands:

```bash
cargo test -p gridix --lib config
cargo test --test core_tests
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

Acceptance:
- Completed 2026-06-18.
- Existing config files remain loadable.
- New config serializes with `[workbench]`.
- No visual behavior changes.

Suggested commit:

```text
feat(config): add workbench layout preferences
```

Rollback:
- Remove `workbench` field and related structs.
- Existing old config remains unaffected because migration is additive.

## Phase 2: Workbench State And Shell Foundation

Goal: introduce runtime state and shell wrapper while preserving current UI.

Files:
- `src/state/workbench.rs` new
- `src/state/mod.rs`
- `src/app/surfaces/workbench.rs` new
- `src/app/surfaces/mod.rs`
- `src/app/surfaces/render.rs`
- `src/ui/workbench/mod.rs` new
- `src/ui/workbench/shell.rs` new
- `src/ui/workbench/status_bar.rs` new

Steps:

1. Add `WorkbenchState`.
2. Add `WorkbenchFocus`.
3. Add bridge methods:
   - `WorkbenchFocus::from_focus_area(FocusArea)`
   - `FocusArea` fallback mapping where needed
4. Add `UiState.workbench`.
5. Add `DbManagerApp::render_workbench()`.
6. Wrap current layout in a shell without changing region behavior.
7. Add StatusBar using existing `ActionContext::status_line()` or equivalent snapshot.
8. Move frame orchestration comments from "CentralPanel" to "Workbench".

Do not:
- Move Toolbar out yet if it makes the PR too large.
- Delete old `show_sidebar`/`sidebar_width`.
- Delete old `FocusArea`.

Tests:
- `UiState::default()` includes valid workbench state.
- Focus bridge maps all existing `FocusArea` variants.
- Status line can be generated with no active connection.

Commands:

```bash
cargo check
cargo test -p gridix --lib workbench
cargo test -p gridix --lib
cargo fmt --check
```

Acceptance:
- Completed 2026-06-18.
- App renders same major UI plus status bar if enabled.
- No workflow breakage in full test suite.
- New shell module exists in `src/ui/workbench/`.
- Legacy sidebar visibility/width and focus state synchronize into `UiState.workbench`.

Suggested commit:

```text
refactor(ui): introduce workbench shell state
```

## Phase 3: Global TopBar

Goal: move toolbar out of dock tab content and render it once globally.

Files:
- `src/app/surfaces/render.rs`
- `src/app/surfaces/workbench.rs`
- `src/ui/components/toolbar/mod.rs`
- `src/ui/workbench/top_bar.rs` optional
- tests around rendering/action availability if present

Steps:

1. Extract the toolbar rendering block from `render_workspace_content()`.
2. Create `DbManagerApp::render_top_bar(ui) -> ToolbarActions`.
3. Render it in the shell top region before sidebar/editor layout.
4. Keep `handle_toolbar_actions()` unchanged.
5. Remove duplicate separators left behind in QueryData content.
6. Ensure toolbar focus still works through existing `FocusArea::Toolbar`.

Do not:
- Redesign toolbar visuals yet.
- Replace toolbar icons yet.

Tests:
- Toolbar action handling remains unchanged.
- Focus area `Toolbar` still handles keyboard navigation.
- QueryData content no longer renders toolbar.

Manual checks:
- Launch app.
- Toggle sidebar.
- Toggle editor.
- Open action/create/theme menus.
- Run existing keyboard shortcuts.

Commands:

```bash
cargo test -p gridix --lib toolbar
cargo test -p gridix --lib input_router
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Acceptance:
- Completed 2026-06-18.
- Toolbar appears exactly once.
- Dock tabs contain document content only.
- `ToolbarActions` and `handle_toolbar_actions()` remain the action boundary.
- Toolbar keyboard activation and focus transfer still work through `FocusArea::Toolbar`.

Suggested commit:

```text
refactor(ui): render toolbar as global top bar
```

## Phase 4: ActivityBar And Sidebar Activities

Goal: replace always-stacked sidebar panels with activity-driven sidebar.

Files:
- `src/core/config.rs`
- `src/state/workbench.rs`
- `src/app/action/action_system.rs`
- `src/app/input/input_router.rs`
- `src/app/surfaces/workbench.rs`
- `src/ui/workbench/activity_bar.rs`
- `src/ui/workbench/primary_sidebar.rs`
- `src/ui/panels/sidebar/mod.rs`
- `src/ui/panels/sidebar/state.rs`

Steps:

1. Add app actions:
   - `SetWorkbenchActivity(WorkbenchActivity)`
   - `TogglePrimarySidebar`
   - `SetPrimarySidebarVisible(bool)`
2. Add command descriptors for activity switches.
3. Render ActivityBar.
4. Map activities:
   - Explorer -> current connection/database/table panels
   - Filters -> current filter panel only
   - Objects -> triggers/routines
   - History -> history list placeholder or existing panel adapter
   - Help -> help navigation placeholder
   - Settings -> keybindings/theme links placeholder
5. Keep old `SidebarPanelState` but make activity decide which sections render.
6. Opening filter workspace should set activity to Filters and reveal sidebar.

Do not:
- Delete old sidebar section keyboard behavior in this phase.
- Move History/Help full content yet.

Tests:
- Activity action changes state.
- Filter action reveals Filters activity.
- Explorer is default.
- Hiding sidebar keeps ActivityBar visible if configured.

Commands:

```bash
cargo test -p gridix --lib action_system
cargo test -p gridix --lib input_router
cargo test -p gridix --lib sidebar
cargo fmt --check
```

Acceptance:
- Completed 2026-06-18.
- Sidebar no longer shows all major panels by default; Explorer maps to connections only.
- Activity switching is visible through ActivityBar and keyboard-accessible through command descriptors.
- Existing Explorer/table selection flow still uses legacy sidebar panels.
- Hiding PrimarySidebar leaves ActivityBar visible.

Suggested commit:

```text
feat(ui): add activity bar and sidebar activities
```

## Phase 5: BottomPanel Results And Messages

Goal: put query output into a bottom panel instead of mixing it into editor content.

Files:
- `src/state/workbench.rs`
- `src/app/action/action_system.rs`
- `src/app/surfaces/workbench.rs`
- `src/app/surfaces/render.rs`
- `src/ui/workbench/bottom_panel.rs`
- `src/ui/components/grid/mod.rs`
- `src/ui/panels/history_panel.rs` later adapter

Steps:

1. Add app actions:
   - `ToggleBottomPanel`
   - `SetBottomPanelVisible(bool)`
   - `SetBottomPanelTab(BottomPanelTab)`
2. Render BottomPanel below EditorArea.
3. Move `DataGrid::show_editable()` render path into `BottomPanelTab::Results`.
4. Move query error surface into `BottomPanelTab::Messages`.
5. Add empty states:
   - no result
   - no message
   - explain not available
   - no active tasks
6. Query success should set bottom tab to Results when `auto_open_on_query`.
7. Query failure should set bottom tab to Messages.

Do not:
- Refactor DataGrid internals.
- Rewrite history panel.
- Delete old `WorkspaceSurface` until replacement is tested.

Tests:
- `classify_workspace_surface()` replacement logic or new bottom panel classification.
- Query error maps to Messages.
- Results tab renders empty state with no result.
- Bottom panel height clamps.

Manual checks:
- Execute successful query.
- Execute failing query.
- Toggle bottom panel.
- Add filter from grid.
- Save grid changes.

Commands:

```bash
cargo test -p gridix --lib render
cargo test --test grid_tests
cargo test --test edge_regression_tests
cargo test
```

Acceptance:
- Completed 2026-06-18.
- SQL editor and other editor documents remain visible while results/errors update below.
- Query error no longer replaces editor content; `QueryData` shows a routing placeholder and details live in `BottomPanel::Messages`.
- `DataGrid::show_editable()` is rendered from `BottomPanel::Results` and existing grid/filter/save action paths are preserved.
- BottomPanel visibility, active tab, and height synchronize to `AppConfig.workbench.bottom_panel`.

Suggested commit:

```text
feat(ui): route query output through bottom panel
```

## Phase 6: EditorArea Dock Tab Semantics

Goal: replace ambiguous `QueryData`/`SqlEditor` dock model with document/view semantics.

Files:
- `src/ui/dock_tabs.rs`
- `src/session/tab.rs`
- `src/app/mod.rs`
- `src/app/surfaces/workbench.rs`
- `src/app/surfaces/render.rs`
- `src/app/input/input_router.rs`

Steps:

1. Define target tab enum:
   - `SqlDocument`
   - `TableData`
   - `ErDiagram`
   - `SchemaObject`
   - `Welcome`
2. Start conservatively:
   - Rename `QueryData` to a clearer current-purpose variant if behavior remains.
   - Move SQL editor into primary editor path.
3. Keep `QueryTabManager` as SQL document state owner.
4. Ensure closing SQL document follows tab close lifecycle.
5. Open ER diagram as editor tab/canvas.
6. Welcome becomes editor empty state.
7. Remove old `show_sql_editor` semantics only after new SQL document behavior is stable.

Do not:
- Serialize full DockState yet.
- Change query execution semantics.

Tests:
- Tab create/switch/close still isolates SQL/result.
- Closing active SQL document cancels pending query.
- ER toggle opens/closes editor tab.
- Welcome appears with no active document/result.

Commands:

```bash
cargo test -p gridix --lib dock_tabs
cargo test -p gridix --lib tab
cargo test --test edge_regression_tests
cargo test
```

Acceptance:
- Completed 2026-06-18.
- Editor tabs represent documents/views: `SqlDocument`, `TableData`, `ErDiagram`, `SchemaObject`, `Welcome`, and `AuxPanel`.
- SQL is no longer created as a separate bottom scratchpad dock tab; the compatibility `show_sql_editor` flag only hides/reveals SQL document content.
- Existing query tab behavior is preserved through `QueryTabManager`.
- This follows the VS Code/Zed direction at a Gridix scale: VS Code separates editor inputs from editor groups, while Zed separates workspace panes from pane items.

Suggested commit:

```text
refactor(ui): give dock tabs document semantics
```

## Phase 7: RightInspector

Goal: add a non-blocking contextual inspector.

Files:
- `src/state/workbench.rs`
- `src/app/action/action_system.rs`
- `src/app/surfaces/workbench.rs`
- `src/ui/workbench/right_inspector.rs`
- `src/app/runtime/metadata.rs`
- `src/app/runtime/er_diagram.rs`
- `src/ui/components/er_diagram/state.rs`

Steps:

1. Add actions:
   - `ToggleRightInspector`
   - `SetRightInspectorVisible(bool)`
   - `SetRightInspectorTab(RightInspectorTab)`
2. Add inspector context state:
   - selected table schema
   - selected row
   - selected cell
   - selected ER table/node
   - connection metadata
3. Route table schema inspection to RightInspector.
4. Route ER selection detail to RightInspector.
5. Add cell full-value view.

Do not:
- Remove schema dialogs until inspector has coverage.

Tests:
- Inspect table opens Schema tab.
- Selecting cell updates Cell tab context.
- Inspector width clamps.

Commands:

```bash
cargo test -p gridix --lib er_diagram
cargo test --test grid_tests
cargo test
```

Acceptance:
- Completed 2026-06-18.
- Inspect actions do not overwrite editor or results.
- Inspector can be hidden/reopened with state intact.
- Schema and ER inspect paths reveal the appropriate inspector tab.
- Row/Cell tabs render current result selection without adding a blocking dialog.

Suggested commit:

```text
feat(ui): add contextual right inspector
```

## Phase 8: Dialog Reduction

Goal: reduce floating windows where panel-based UX is better.

Files:
- `src/ui/panels/history_panel.rs`
- `src/ui/dialogs/help_dialog.rs`
- `src/ui/dialogs/keybindings_dialog.rs`
- `src/app/surfaces/dialogs.rs`
- `src/app/surfaces/workbench.rs`
- `src/ui/workbench/primary_sidebar.rs`
- `src/ui/workbench/bottom_panel.rs`

Steps:

1. Extract `HistoryPanel::show_in_ui()`.
2. Use History as Sidebar activity or BottomPanel tab.
3. Extract Help content into a reusable `show_in_ui()` path.
4. Keep Keybindings as workspace dialog initially unless editing UX becomes clean in Settings activity.
5. Ensure dialogs remain for transactional workflows:
   - connection create/edit
   - create table/database/user
   - import/export wizard if still complex
   - destructive confirmation

Do not:
- Convert all dialogs blindly.
- Break dialog focus routing.

Tests:
- History keyboard navigation works in panel mode.
- Help close/toggle behavior works.
- Dialog ownership remains single-owner.

Commands:

```bash
cargo test -p gridix --lib dialogs
cargo test --test ui_dialogs_tests
cargo test -p gridix --lib input_router
cargo test
```

Acceptance:
- Help/history no longer obscure main workspace by default.
- Transactional dialogs still behave consistently.

Suggested commit:

```text
refactor(ui): move history and help toward workbench panels
```

## Phase 9: Action System Split

Goal: split `action_system.rs` without behavior change.

Current file:
- `src/app/action/action_system.rs` ~1573 lines

Target files:

```text
src/app/action/action_system.rs      thin facade or mod
src/app/action/actions.rs            AppAction enum
src/app/action/context.rs            ActionContext, ActionAvailability
src/app/action/registry.rs           CommandDescriptor and command list
src/app/action/search.rs             command search/scoring
src/app/action/reducer.rs            dispatch and effects
src/app/action/tests.rs              moved tests
```

Steps:

1. Move pure data types first.
2. Move command registry.
3. Move availability/scoring.
4. Move reducer last.
5. Keep public imports stable through `app/action/mod.rs`.

Do not:
- Add new behavior in split PR.
- Rename command IDs.

Tests:
- All action tests pass.
- Command palette still finds existing commands.
- Availability unchanged for no connection / active connection / result states.

Commands:

```bash
cargo test -p gridix --lib action_system
cargo test -p gridix --lib command_palette
cargo test
```

Acceptance:
- `action_system.rs` reduced below 400 lines.
- No behavior changes.

Suggested commits:

```text
refactor(action): split command registry from reducer
refactor(action): extract availability and scoring
```

## Phase 10: Input Router Split

Goal: split `input_router.rs` into focus/context/workspace/dialog/global routers.

Current file:
- `src/app/input/input_router.rs` ~3393 lines

Target files:

```text
src/app/input/context.rs
src/app/input/focus.rs
src/app/input/router.rs
src/app/input/global.rs
src/app/input/workspace.rs
src/app/input/dialog.rs
src/app/input/text_entry.rs
src/app/input/tests.rs
```

Steps:

1. Move passive enums and context structs.
2. Move text-entry guard logic.
3. Move dialog scope resolution.
4. Move global shortcut resolution.
5. Move workspace/local routing.
6. Leave old file as a facade until imports settle.

Do not:
- Change keybindings.
- Change focus order during the split.

Tests:
- Existing input router tests pass after each move.
- Text entry priority tests pass.
- Dialog escape/recording input behavior unchanged.
- ER focus shortcuts unchanged.

Commands:

```bash
cargo test -p gridix --lib input_router
cargo test -p gridix --lib keybindings
cargo test
```

Acceptance:
- No file in `src/app/input/` exceeds 900 lines except tests during transition.
- Focus and shortcut behavior preserved.

Suggested commits:

```text
refactor(input): extract router context and focus scopes
refactor(input): split dialog and workspace routing
```

## Phase 11: Keybindings Core Split

Goal: split parser/registry/diagnostics/persistence/defaults.

Current file:
- `src/core/keybindings.rs` ~2385 lines

Target files:

```text
src/core/keybindings/mod.rs
src/core/keybindings/binding.rs
src/core/keybindings/parser.rs
src/core/keybindings/registry.rs
src/core/keybindings/diagnostics.rs
src/core/keybindings/persistence.rs
src/core/keybindings/defaults.rs
src/core/keybindings/tests.rs
```

Steps:

1. Move `KeyBinding` and parsing to `binding.rs/parser.rs`.
2. Move diagnostics to `diagnostics.rs`.
3. Move loading/saving to `persistence.rs`.
4. Move default bindings and registry helpers to `defaults.rs/registry.rs`.
5. Keep `pub use` in `mod.rs` to avoid broad import churn.

Do not:
- Change keymap file format.
- Change command IDs.
- Change default bindings unless required by a specific UX phase.

Tests:
- Existing keybinding parse tests pass.
- Conflict diagnostics unchanged.
- Legacy config migration unchanged.
- Save file permissions unchanged.

Commands:

```bash
cargo test -p gridix --lib keybindings
cargo test --test core_tests
cargo test
```

Acceptance:
- `src/core/keybindings.rs` becomes a module folder or facade.
- No behavior changes.

Suggested commits:

```text
refactor(keybindings): split parser and binding model
refactor(keybindings): split diagnostics and persistence
```

## Phase 12: Runtime Handler Split

Goal: split message handlers by domain.

Current file:
- `src/app/runtime/handler.rs` ~1589 lines

Target files:

```text
src/app/runtime/handler/mod.rs
src/app/runtime/handler/connection.rs
src/app/runtime/handler/query.rs
src/app/runtime/handler/grid.rs
src/app/runtime/handler/metadata.rs
src/app/runtime/handler/er.rs
src/app/runtime/handler/tests.rs
```

Steps:

1. Move pure helper functions first.
2. Move connection handlers.
3. Move query/result handlers.
4. Move grid save handlers.
5. Move metadata/ER handlers.
6. Keep `handle_messages()` entry stable.

Do not:
- Change message enum shape unless required by tests.
- Remove `request_id` guards.
- Change `needs_repaint` behavior.

Tests:
- Stale response guards.
- Query result updates correct tab.
- Grid save isolation.
- Connection pending guard.
- ER load handler.

Commands:

```bash
cargo test -p gridix --lib handler
cargo test --test edge_regression_tests
cargo test
```

Acceptance:
- Handler domain files are small and testable.
- All request-id invariants preserved.

Suggested commit:

```text
refactor(runtime): split message handlers by domain
```

## Phase 13: Data Query Cleanup And Coverage

Goal: improve data/query maintainability without changing the no-trait ADR.

Files:
- `src/data/query/mod.rs`
- `src/data/query/sqlite.rs`
- `src/data/query/postgres.rs`
- `src/data/query/mysql.rs`
- `src/data/pool.rs`
- tests

Steps:

1. Keep `match db_type` dispatch.
2. Split repeated schema/result conversion helpers into backend-specific helper modules if needed.
3. Add focused SQLite in-memory tests for new query helpers.
4. Add PostgreSQL/MySQL tests only where mockable or ignored integration env exists.
5. Ensure cancellation logic in MySQL remains covered.

Do not:
- Reintroduce a database trait.
- Change public data API and UI behavior in the same PR.

Tests:
- SQLite schema tests.
- query execution path tests where possible.
- MySQL ignored integration still compiles.

Commands:

```bash
cargo test --test database_tests
cargo test --test mysql_cancel_integration -- --ignored --nocapture
cargo test
```

Acceptance:
- Backend dispatch remains explicit.
- More data/query files have direct tests.

Suggested commits:

```text
test(data): expand query backend coverage
refactor(data): extract query conversion helpers
```

## Phase 14: ER Diagram And Grid Cleanup

Goal: reduce ER/grid oversized files after workbench region responsibilities are stable.

ER files:
- `src/ui/components/er_diagram/layout.rs`
- `src/ui/components/er_diagram/render.rs`
- `src/ui/components/er_diagram/state.rs`

Grid files:
- `src/ui/components/grid/keyboard.rs`
- `src/ui/components/grid/mod.rs`
- `src/ui/components/grid/render.rs`

ER target:

```text
er_diagram/layout/
  mod.rs
  graph_layers.rs
  overlap.rs
  force.rs
  relationship_seeded.rs
  tests.rs

er_diagram/render/
  mod.rs
  canvas.rs
  table_card.rs
  edges.rs
  toolbar.rs
  interaction.rs
```

Grid target:

```text
grid/keyboard/
  mod.rs
  normal.rs
  insert.rs
  select.rs
  commands.rs
  tests.rs
```

Steps:

1. Split pure helpers first.
2. Move tests close to target modules.
3. Keep public component API stable.
4. Avoid visual changes in structural split PRs.

Tests:
- ER layout placement tests.
- ER render geometry tests.
- Grid keyboard mode tests.
- Grid edit/save tests.

Commands:

```bash
cargo test -p gridix --lib er_diagram
cargo test -p gridix --lib grid
cargo test --test grid_tests
cargo test
```

Acceptance:
- No ER/grid file exceeds 1200 lines, then later target <900.
- Behavior remains stable.

Suggested commits:

```text
refactor(er): split layout algorithms
refactor(grid): split keyboard mode handlers
```

## Phase 15: Visual Tokens And Polish

Goal: apply editor-like visual system after layout is stable.

Primary reference:
- `~/.codex/references/workbench-ui-refactor-spec.md`

Files:
- `src/ui/styles.rs`
- `src/ui/workbench/tokens.rs`
- `src/ui/components/toolbar/*`
- `src/ui/workbench/*`
- `src/ui/components/grid/*`
- `src/ui/components/sql_editor.rs`

Steps:

1. Add workbench palette tokens.
2. Map tokens to current theme manager.
3. Reduce emoji-only core chrome.
4. Normalize region backgrounds and 1 px borders.
5. Tune density.
6. Add reset layout action.

Do not:
- Change data behavior.
- Change keyboard behavior unless explicitly part of UI discoverability.

Manual checks:
- 1366x768
- 1920x1080
- narrow ~900 px
- dark/light
- UI scale 0.8/1.0/1.25

Commands:

```bash
cargo test --test ui_dialogs_tests
cargo test -p gridix --lib
cargo run --bin check-doc-links
```

Acceptance:
- UI clearly reads as a stable editor-like shell.
- No major region relies on ambiguous emoji-only controls.

Suggested commit:

```text
style(ui): apply workbench visual tokens
```

## Phase 16: Documentation And Release Readiness

Goal: finish refactor with documentation and quality gates.

Files:
- `CLAUDE.md`
- `docs/CHANGELOG.md`
- `~/.codex/references/*`
- `~/.codex/rules/*`

Steps:

1. Update module map.
2. Update `ui-egui.md` rules to reflect final workbench rules.
3. Update roadmap completed items.
4. Update tech debt resolved/remaining.
5. Add changelog entry.
6. Run full pre-PR.

Commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin check-doc-links
```

Acceptance:
- Docs reflect actual code.
- No stale references to old layout architecture.
- Release notes describe user-visible UI changes.

Suggested commit:

```text
docs(ui): document workbench refactor
```

## PR Queue

Execute in this order:

1. `docs(refactor): add project execution plan`
2. `feat(config): add workbench layout preferences`
3. `refactor(ui): introduce workbench shell state`
4. `refactor(ui): render toolbar as global top bar`
5. `feat(ui): add activity bar and sidebar activities`
6. `feat(ui): route query output through bottom panel`
7. `refactor(ui): give dock tabs document semantics`
8. `feat(ui): add contextual right inspector`
9. `refactor(ui): move history and help toward workbench panels`
10. `refactor(action): split command registry from reducer`
11. `refactor(input): extract router context and focus scopes`
12. `refactor(input): split dialog and workspace routing`
13. `refactor(keybindings): split parser and binding model`
14. `refactor(keybindings): split diagnostics and persistence`
15. `refactor(runtime): split message handlers by domain`
16. `test(data): expand query backend coverage`
17. `refactor(data): extract query conversion helpers`
18. `refactor(er): split layout algorithms`
19. `refactor(grid): split keyboard mode handlers`
20. `style(ui): apply workbench visual tokens`
21. `docs(ui): document workbench refactor`

Do not merge adjacent PRs just because they are related. Small PRs preserve reviewability.

## Branch Strategy

Recommended:

```bash
git switch -c refactor/workbench-config
```

For each PR:

```bash
git status --short
cargo fmt --check
cargo test -p gridix --lib
git add <changed files>
git commit -m "type(scope): description"
```

Before push/merge:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin check-doc-links
```

## Required Regression Matrix

Run manually after Phases 3, 5, 6, 8, and 15:

| Area | Check |
|---|---|
| Startup | app launches with empty config and existing config |
| Connection | create/connect/disconnect SQLite |
| Database tree | select database/table, refresh |
| Query | type SQL, execute, cancel if long-running |
| Results | rows display, empty result, failed query message |
| Grid edit | edit cell, mark delete, save, discard |
| Tabs | new tab, switch tab, close tab, close others/right |
| ER | open ER, refresh, fit, relayout, close |
| Dialogs | connection, import, export, confirm delete |
| Keyboard | toolbar/sidebar/editor/grid/dialog shortcuts |
| Layout | hide/show sidebar, bottom panel, inspector, restart |
| Theme/scale | dark/light, scale changes |

## Stop Conditions

Stop and reassess if any of these occur:

1. A PR changes more than three high-risk flows.
2. Tests require broad snapshot rewrites without clear behavior changes.
3. A refactor needs deleting request-id guards.
4. `QueryTab.sql` is no longer the sole SQL source.
5. Lower layers need to import `egui`, `ui`, `app`, or `state`.
6. Layout config cannot load old configs.
7. More than one large module is rewritten in the same PR.

## Agent Checklist Per Phase

Before coding:

```text
1. Read this phase.
2. Read referenced files.
3. Run targeted baseline tests.
4. Identify exact files to touch.
5. Make one mechanical change at a time.
```

During coding:

```text
1. Prefer moves/extractions before behavior changes.
2. Compile after each large move.
3. Keep old adapters until replacement tests pass.
4. Do not opportunistically restyle unrelated components.
```

After coding:

```text
1. Run targeted tests.
2. Run full fmt.
3. Run clippy/full tests before merge.
4. Update relevant `~/.codex` references.
5. Update `CLAUDE.md` only if repository architecture changed.
6. Update changelog for user-visible changes.
```

## First Concrete Task To Start Implementation

Start with Phase 1.

Exact prompt/task:

```text
Implement Phase 1 from ~/.codex/references/project-refactor-execution-plan.md:
add WorkbenchConfig and related persisted config defaults/tests, without changing UI rendering.
```

Expected touched files:

```text
src/core/config.rs
tests/core_tests.rs or src/core/config.rs tests
~/.codex/references/project-refactor-execution-plan.md if details change
~/.codex/references/workbench-ui-refactor-spec.md if config names change
```

Expected commands:

```bash
cargo test -p gridix --lib config
cargo test --test core_tests
cargo fmt --check
```

Definition of done:

```text
WorkbenchConfig exists, defaults are tested, old configs load, dimensions clamp, no UI behavior changed.
```
