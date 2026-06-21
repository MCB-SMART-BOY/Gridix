# Gridix UI Refactor Configuration And Design Spec

## Purpose

This document is the implementation-level plan for the first fixed-region compatibility shell. It is useful for understanding Phases 0-7, but the final layout target is now Dockable Workbench v2.

The high-level design lives in `~/.codex/references/workbench-ui-design.md`. This spec defines the concrete configuration model, UI state model, module boundaries, layout dimensions, interaction rules, migration order, and validation requirements for the compatibility shell. The current final target lives in `~/.codex/references/dockable-workbench-v2.md` and `~/.codex/references/gridix-ui-visual-system-v2.md`.

## Summary Decision

This compatibility phase uses a stable workbench shell:

```text
TopBar
ActivityBar | PrimarySidebar | EditorArea | RightInspector
            |                | BottomPanel |
StatusBar
```

In the original fixed-region phase, the dock area owned only the `EditorArea`. After the 2026-06-18 design pivot, the target model changes to `TopBar + Dockable Workspace + StatusBar`, where Explorer, Results, Inspector, SQL, ER, and related panels become peer dockable surfaces.

## Implementation Status

Last updated: 2026-06-18.

Completed:
- Phase 0 safety baseline is clean.
- Phase 1 persisted workbench configuration is implemented in `src/core/config.rs` and constants live in `src/core/constants.rs`.
- `AppConfig.workbench` is additive, serialized as `[workbench]`, and saved config version is 3.
- Legacy `sidebar.edge_transfer` is copied into `workbench.sidebar.edge_transfer` when the new value is missing.
- Phase 2 runtime shell foundation is implemented: `src/state/workbench.rs`, `UiState.workbench`, `src/ui/workbench/{shell,status_bar}.rs`, and `src/app/surfaces/workbench.rs`.
- Current layout remains visually compatible and is wrapped by `WorkbenchShell`; StatusBar renders from existing action/status snapshots when enabled.
- Legacy `FocusArea`, sidebar visibility, sidebar width, and sidebar resizing state synchronize into runtime `WorkbenchState`.
- Phase 3 global TopBar is implemented through `DbManagerApp::render_top_bar()`: toolbar rendering moved out of dock document content and is rendered once above the sidebar/editor layout.
- Phase 4 ActivityBar is implemented through `WorkbenchActivityBar`; `WorkbenchActivity` drives PrimarySidebar content by adapting legacy sidebar panels for Explorer/Filters/Objects and placeholders for History/Help/Settings.
- Phase 5 BottomPanel is implemented through `WorkbenchBottomPanel`; `BottomPanelTab::Results` renders the DataGrid path, `Messages` renders query errors/status, Explain/History/Tasks have usable empty or lightweight states, and query success/failure reveal Results/Messages through app actions/config.
- Phase 6 EditorArea dock tab semantics are implemented: SQL editor tabs are `SqlDocument` views backed by `QueryTabManager`, ER remains an EditorArea view, and target variants exist for `TableData`, `SchemaObject`, `Welcome`, and `AuxPanel`.
- Phase 7 RightInspector is implemented through `WorkbenchRightInspector`; schema inspection opens Schema, ER selection opens ErSelection, Row/Cell/Properties/Connection views are non-blocking inspector content, and visibility/tab/width persist through app actions/config.
- Post-pivot Dockable Workbench v2 bridge is partially implemented: `WorkbenchSurfaceKind`, descriptors, `DockTab::Surface`, `default_surface_layout()`, `ensure_surface_tab()`, unified surface rendering, reveal/open action wiring, fixed fallback de-duplication, and runtime startup on the surface dock seed are in place.

Next:
- Follow `dockable-workbench-v2.md` before Phase 8 dialog reduction: migrate remaining fixed-region chrome into the shared surface shell and continue surface-first focus/config cleanup.

## Current State To Replace

| Current item | Current behavior | Target behavior |
|---|---|---|
| `render.rs` root | `CentralPanel + horizontal` manually lays out sidebar and main dock | `WorkbenchShell` owns all fixed regions |
| `Toolbar` | Rendered once globally by `DbManagerApp::render_top_bar()` | Keep as global `TopBar`; visual redesign is separate |
| `Sidebar` | Explorer/Filters/Objects dock surfaces render real Sidebar content through a compatibility adapter; History/Help/Settings still use placeholder/dialog paths | Split independent navigation surface state and continue replacing fixed PrimarySidebar assumptions |
| `DockTab::SqlDocument` | Primary SQL document tab backed by `QueryTabManager`; renders the SQL editor directly in EditorArea | Keep as the SQL document surface until stable document IDs replace index-based query tabs |
| `DockTab::TableData` / `SchemaObject` / `Welcome` | Target document/view variants exist with conservative placeholder rendering where data identity is not implemented yet | Wire table/schema/welcome opening behavior as later EditorArea slices |
| Query results | Rendered in `BottomPanel::Results` | Keep there and harden lifecycle/focus semantics |
| Query errors | Rendered in `BottomPanel::Messages` | Keep there and add richer diagnostics over time |
| RightInspector | Rendered as a fixed right workbench region with Properties/Schema/Row/Cell/ER/Connection tabs | Harden focus/keyboard routing after legacy `FocusArea` is split |
| History/help/keybindings | Float as dialogs/windows | Become panel/activity/editor surfaces where practical |
| Layout config | Mostly hardcoded defaults in `UiState` | Persisted `WorkbenchConfig` in `AppConfig` |

## Architectural Rules

1. `src/core/config.rs` may define serialized workbench config types because `AppConfig` already owns persisted UI preferences.
2. Persisted config types must not depend on `src/ui` or `src/app`.
3. Runtime UI state belongs in `src/state/`, ideally `src/state/workbench.rs`.
4. Rendering belongs in `src/ui/workbench/` for reusable widgets and `src/app/surfaces/workbench.rs` for app-bound orchestration.
5. App actions remain the semantic boundary. Mouse, keyboard, toolbar, command palette, and panel controls should dispatch `AppAction` where possible.
6. The first migration should wrap existing UI. Do not rewrite all components before the shell exists.

## Proposed File Layout

```text
src/core/config.rs
  WorkbenchConfig
  WorkbenchActivity
  WorkbenchDensity
  PrimarySidebarConfig
  BottomPanelConfig
  RightInspectorConfig
  EditorAreaConfig
  WorkbenchBehaviorConfig

src/state/workbench.rs
  WorkbenchState
  WorkbenchFocus
  TopBarState
  ActivityBarState
  PrimarySidebarState
  BottomPanelState
  RightInspectorState
  EditorAreaState

src/state/mod.rs
  pub(crate) workbench: WorkbenchState

src/ui/workbench/mod.rs
src/ui/workbench/shell.rs
src/ui/workbench/top_bar.rs
src/ui/workbench/activity_bar.rs
src/ui/workbench/primary_sidebar.rs
src/ui/workbench/editor_area.rs
src/ui/workbench/bottom_panel.rs
src/ui/workbench/right_inspector.rs
src/ui/workbench/status_bar.rs
src/ui/workbench/tokens.rs

src/app/surfaces/workbench.rs
  DbManagerApp::render_workbench()
  app-bound render adapters for existing components
```

Keep `src/app/surfaces/render.rs` as the frame orchestration entry initially. Move root shell code into `workbench.rs` after tests cover the new shell.

## Persisted Configuration

Add this to `AppConfig`:

```rust
#[serde(default)]
pub workbench: WorkbenchConfig,
```

Recommended config version:

```rust
pub const CONFIG_VERSION_WORKBENCH: u32 = 3;
```

Because the change is additive and all fields use `#[serde(default)]`, old config files can still load safely. Bump `version` to 3 when saving after the migration.

### WorkbenchConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchConfig {
    #[serde(default = "default_workbench_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub activity: WorkbenchActivity,
    #[serde(default)]
    pub density: WorkbenchDensity,
    #[serde(default)]
    pub sidebar: PrimarySidebarConfig,
    #[serde(default)]
    pub bottom_panel: BottomPanelConfig,
    #[serde(default)]
    pub right_inspector: RightInspectorConfig,
    #[serde(default)]
    pub editor: EditorAreaConfig,
    #[serde(default)]
    pub status_bar: StatusBarConfig,
    #[serde(default)]
    pub behavior: WorkbenchBehaviorConfig,
}
```

### WorkbenchActivity

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchActivity {
    #[default]
    Explorer,
    Filters,
    Objects,
    History,
    Help,
    Settings,
}
```

Activities define what the `PrimarySidebar` shows. They do not directly define focus.

### WorkbenchDensity

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchDensity {
    Comfortable,
    #[default]
    Compact,
    Dense,
}
```

Recommended mapping:

| Density | TopBar | ActivityBar | StatusBar | Row height |
|---|---:|---:|---:|---:|
| Comfortable | 44 | 52 | 26 | existing + 4 |
| Compact | 40 | 48 | 24 | existing |
| Dense | 36 | 44 | 22 | existing - 2 |

Do not implement all density modes in the first slice. Add config and defaults first; use Compact.

### PrimarySidebarConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimarySidebarConfig {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_primary_sidebar_width")]
    pub width: f32,
    #[serde(default = "default_primary_sidebar_min_width")]
    pub min_width: f32,
    #[serde(default = "default_primary_sidebar_max_width")]
    pub max_width: f32,
    #[serde(default = "default_true")]
    pub edge_transfer: bool,
}
```

Defaults:

```text
visible = true
width = 280.0
min_width = 220.0
max_width = 460.0
edge_transfer = true
```

Compatibility rule:
- Existing `AppConfig.sidebar.edge_transfer` remains readable for one compatibility window.
- New saves should write `workbench.sidebar.edge_transfer`.
- The old `sidebar` config can remain as a deprecated mirror until the next cleanup release.

### BottomPanelConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottomPanelConfig {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_bottom_panel_height")]
    pub height: f32,
    #[serde(default = "default_bottom_panel_min_height")]
    pub min_height: f32,
    #[serde(default = "default_bottom_panel_max_ratio")]
    pub max_height_ratio: f32,
    #[serde(default)]
    pub active_tab: BottomPanelTab,
    #[serde(default = "default_true")]
    pub auto_open_on_query: bool,
    #[serde(default = "default_true")]
    pub auto_focus_results_on_execute: bool,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BottomPanelTab {
    #[default]
    Results,
    Messages,
    Explain,
    History,
    Tasks,
}
```

Defaults:

```text
visible = true
height = 260.0
min_height = 140.0
max_height_ratio = 0.55
active_tab = results
auto_open_on_query = true
auto_focus_results_on_execute = true
```

Clamping rule:

```text
effective_max_height = viewport_height * max_height_ratio
height = height.clamp(min_height.min(effective_max_height), effective_max_height)
```

### RightInspectorConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightInspectorConfig {
    #[serde(default)]
    pub visible: bool,
    #[serde(default = "default_right_inspector_width")]
    pub width: f32,
    #[serde(default = "default_right_inspector_min_width")]
    pub min_width: f32,
    #[serde(default = "default_right_inspector_max_width")]
    pub max_width: f32,
    #[serde(default)]
    pub active_tab: RightInspectorTab,
    #[serde(default = "default_true")]
    pub auto_open_on_inspect: bool,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RightInspectorTab {
    #[default]
    Properties,
    Schema,
    Row,
    Cell,
    ErSelection,
    Connection,
}
```

Defaults:

```text
visible = false
width = 320.0
min_width = 260.0
max_width = 480.0
active_tab = properties
auto_open_on_inspect = true
```

### EditorAreaConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorAreaConfig {
    #[serde(default)]
    pub table_open_mode: TableOpenMode,
    #[serde(default)]
    pub result_placement: ResultPlacement,
    #[serde(default = "default_true")]
    pub restore_open_editors: bool,
    #[serde(default = "default_true")]
    pub show_welcome_when_empty: bool,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TableOpenMode {
    #[default]
    ReuseActiveTableView,
    OpenNewTableView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResultPlacement {
    #[default]
    BottomPanel,
    EditorTab,
}
```

First implementation should use:

```text
table_open_mode = reuse_active_table_view
result_placement = bottom_panel
restore_open_editors = true
show_welcome_when_empty = true
```

Do not serialize full `DockState` in the first slice.

### StatusBarConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBarConfig {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_true")]
    pub show_focus_area: bool,
    #[serde(default = "default_true")]
    pub show_query_time: bool,
    #[serde(default = "default_true")]
    pub show_row_count: bool,
}
```

Defaults:

```text
visible = true
show_focus_area = true
show_query_time = true
show_row_count = true
```

### WorkbenchBehaviorConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchBehaviorConfig {
    #[serde(default = "default_true")]
    pub command_palette_prefers_context: bool,
    #[serde(default = "default_true")]
    pub close_empty_panels_on_escape: bool,
    #[serde(default = "default_true")]
    pub reveal_sidebar_on_filter_action: bool,
    #[serde(default = "default_true")]
    pub reveal_inspector_on_schema_action: bool,
}
```

Defaults:

```text
command_palette_prefers_context = true
close_empty_panels_on_escape = true
reveal_sidebar_on_filter_action = true
reveal_inspector_on_schema_action = true
```

## Example config.toml

```toml
version = 3
theme_preset = "tokyo_night_storm"
is_dark_mode = true
ui_scale = 1.0

[workbench]
schema_version = 1
activity = "explorer"
density = "compact"

[workbench.sidebar]
visible = true
width = 280.0
min_width = 220.0
max_width = 460.0
edge_transfer = true

[workbench.bottom_panel]
visible = true
height = 260.0
min_height = 140.0
max_height_ratio = 0.55
active_tab = "results"
auto_open_on_query = true
auto_focus_results_on_execute = true

[workbench.right_inspector]
visible = false
width = 320.0
min_width = 260.0
max_width = 480.0
active_tab = "properties"
auto_open_on_inspect = true

[workbench.editor]
table_open_mode = "reuse_active_table_view"
result_placement = "bottom_panel"
restore_open_editors = true
show_welcome_when_empty = true

[workbench.status_bar]
visible = true
show_focus_area = true
show_query_time = true
show_row_count = true

[workbench.behavior]
command_palette_prefers_context = true
close_empty_panels_on_escape = true
reveal_sidebar_on_filter_action = true
reveal_inspector_on_schema_action = true
```

## Config Migration Rules

### Load

1. Parse `AppConfig` with `#[serde(default)]`.
2. Clamp all layout dimensions after parsing.
3. If `workbench.sidebar.edge_transfer` is missing, copy from legacy `sidebar.edge_transfer`.
4. If `version < 3`, keep behavior compatible and save version 3 only after a real save event.

### Save

1. Save layout changes through `save_config_debounced()`.
2. Save immediately on app exit through existing flush path.
3. Do not save continuously on every drag frame.
4. Mark dirty when drag stops or when visibility/activity changes.

### Clamp

Add helpers that are easy to test:

```rust
impl WorkbenchConfig {
    pub fn normalize_for_viewport(&mut self, viewport: egui::Vec2) {
        self.sidebar.normalize();
        self.bottom_panel.normalize_for_height(viewport.y);
        self.right_inspector.normalize();
    }
}
```

If `core/config.rs` cannot depend on `egui`, pass plain width/height values.

## Runtime UI State

Persisted config and runtime state should be separate.

### WorkbenchState

```rust
pub struct WorkbenchState {
    pub active_activity: WorkbenchActivity,
    pub focus: WorkbenchFocus,
    pub top_bar: TopBarState,
    pub activity_bar: ActivityBarState,
    pub primary_sidebar: PrimarySidebarState,
    pub editor_area: EditorAreaState,
    pub bottom_panel: BottomPanelState,
    pub right_inspector: RightInspectorState,
    pub status_bar: StatusBarState,
}
```

The first slice now stores `active_activity`, `focus`, panel visibility/size, active panel tabs, and drag state. Expand it as TopBar, ActivityBar, BottomPanel, and RightInspector migrate out of legacy rendering paths.

### WorkbenchFocus

```rust
pub enum WorkbenchFocus {
    TopBar,
    ActivityBar,
    PrimarySidebar,
    EditorArea,
    BottomPanel,
    RightInspector,
    Dialog,
}
```

Existing `FocusArea` is bridged:

| Existing FocusArea | WorkbenchFocus |
|---|---|
| Toolbar | TopBar |
| QueryTabs | EditorArea |
| Sidebar | PrimarySidebar |
| DataGrid | EditorArea during the compatibility slice; BottomPanel after results routing migrates |
| ErDiagram | EditorArea |
| SqlEditor | EditorArea |
| Dialog | Dialog |

Do not delete `FocusArea` in the first slice. Add bridge methods and migrate gradually.

## Workbench Region Details

### TopBar

Height:

```text
Comfortable: 44
Compact: 40
Dense: 36
```

Layout:

```text
Left: Activity toggle, command palette, create/action menus
Center: active connection, database, table/object breadcrumb
Right: running task, theme, scale, account/about
```

Rules:
- TopBar is rendered once per frame.
- TopBar owns global actions only.
- TopBar must not contain table filters, row controls, or SQL editor-specific controls.
- Critical actions should use labels or stable icons; avoid ambiguous emoji.

Current migration:
- Current `Toolbar::show_with_focus()` call lives in `DbManagerApp::render_top_bar()`.
- `ToolbarActions` and `handle_toolbar_actions()` remain the action boundary.
- Do not reintroduce toolbar rendering inside dock tab content.

### ActivityBar

Width:

```text
Comfortable: 52
Compact: 48
Dense: 44
```

Activity order:

```text
Explorer
Filters
Objects
History
Help
Settings
```

Rules:
- Active activity has a high-contrast left stripe or filled background.
- Disabled activities are still visible but muted if unavailable.
- ActivityBar handles only activity selection, not content actions.

Initial migration:
- Add a simple vertical bar with text/icons.
- Reuse current command actions where possible.
- Selecting Filters should set sidebar visible and activity filters.

### PrimarySidebar

Width:

```text
default = 280
min = 220
max = 460
```

Explorer:
- Connections
- Databases
- Tables/views
- Refresh/edit/delete actions

Filters:
- Active result filters
- Empty state when no result
- Add/clear/toggle controls

Objects:
- Triggers
- Routines
- Future users/indexes

History:
- Query history list
- Select loads SQL into active SQL document

Help:
- Learning/topic navigation
- Detail opens in EditorArea or stays in sidebar for short content

Settings:
- Keybindings link
- Theme/density settings
- Layout reset action

Rules:
- The old stacked panel mode should be removed after Activities are stable.
- Internal splitters may remain only for Explorer tree sections if needed.
- Sidebar drag should update runtime width continuously but mark config dirty only after drag stop.

### EditorArea

Purpose:
- Primary document and canvas region.
- Owned by `egui_dock`.

New conceptual tabs:

```rust
pub enum WorkbenchDockTab {
    SqlDocument { query_tab_id: String },
    TableData { workspace_id: GridWorkspaceId },
    ErDiagram { connection_name: Option<String>, database_name: Option<String> },
    SchemaObject { object_id: SchemaObjectId },
    Welcome,
}
```

Implementation note:
- `GridWorkspaceId` is currently `pub(in crate::app)`. Keep dock tab identity simple until module boundaries are ready.
- First migration can keep `DockTab` in `ui/dock_tabs.rs` and rename later.

Rules:
- SQL editor is a first-class editor tab.
- Welcome is an editor empty state, not hidden inside data result logic.
- ER diagram is an editor document/canvas.
- Table data can be editor tab or bottom result based on `result_placement`.

Recommended first stable model:
- Keep SQL in EditorArea.
- Put query results in BottomPanel.
- Open ER diagram in EditorArea.
- Keep table data as bottom result until table-data document identity is implemented.

### BottomPanel

Height:

```text
default = 260
min = 140
max = viewport * 0.55
```

Tabs:

```text
Results
Messages
Explain
History
Tasks
```

Results:
- Renders current `DataGrid`
- Shows empty state when no result
- Shows row count, filter count, modified count

Messages:
- Query errors
- Connection errors
- Warnings and informational logs

Explain:
- Future query plan visualization
- For now can show text output from EXPLAIN commands

History:
- Query history list or compact view
- Can reuse `HistoryPanel` rendering after extracting a `show_in_ui()` method

Tasks:
- Active progress manager tasks
- Cancel controls

Rules:
- Query success opens Results if `auto_open_on_query`.
- Query failure opens Messages.
- Explain action opens Explain tab.
- Escape may return focus to EditorArea, not necessarily hide BottomPanel.

### RightInspector

Width:

```text
default = 320
min = 260
max = 480
```

Tabs:

```text
Properties
Schema
Row
Cell
ErSelection
Connection
```

Rules:
- It is contextual and non-blocking.
- Inspect table schema should open Schema tab.
- Selecting a result row can update Row tab.
- Selecting a cell can update Cell tab with full value.
- Selecting ER node can update ErSelection tab.

Initial migration:
- Implemented: route `show_table_schema` into Schema tab, route ER selection into ErSelection tab, and render Row/Cell details from current Results selection.
- Keep old Help/History/Keybindings dialog fallback until panel adapters are extracted.

### StatusBar

Height:

```text
Comfortable: 26
Compact: 24
Dense: 22
```

Content priority:

```text
Left: active connection/database/table
Middle: row count, selected cell, result state
Right: query time, focus region, editor mode, tasks
```

Rules:
- Use existing `ActionContext::status_line()` for first implementation.
- Avoid long text overflow; truncate center content first.
- StatusBar is not interactive in the first slice except maybe task cancel indicator.

## Visual System

Add `src/ui/workbench/tokens.rs` or extend `src/ui/styles.rs` with semantic workbench tokens.

### Surface Tokens

```rust
pub struct WorkbenchPalette {
    pub surface_base: Color32,
    pub surface_top_bar: Color32,
    pub surface_activity_bar: Color32,
    pub surface_sidebar: Color32,
    pub surface_editor: Color32,
    pub surface_bottom_panel: Color32,
    pub surface_inspector: Color32,
    pub surface_status_bar: Color32,
    pub border_subtle: Color32,
    pub border_active: Color32,
    pub activity_active: Color32,
    pub tab_active: Color32,
    pub tab_inactive: Color32,
    pub focus_ring: Color32,
}
```

Rules:
- Use 1 px separators for region boundaries.
- Use active accent for focus, selected activity, and active tab.
- Use different surface shades to define structure instead of adding many separators.
- Keep data grid dense and legible.

### Typography

Use existing egui fonts first.

Recommended roles:

```text
chrome label: 12-13 px
sidebar item: 13 px
editor/result text: monospace 13-14 px
status bar: 11-12 px
empty state title: 18-22 px
```

Do not spend the first implementation slice on custom font loading.

### Icon Policy

Rules:
- Core chrome should not rely on emoji-only icons.
- Use simple symbolic text or existing icon abstraction for the first slice.
- Emoji can remain in welcome/help content, but not as the primary workbench navigation language.

## Interaction Rules

### Keyboard

Recommended shortcuts:

```text
Ctrl+B: toggle PrimarySidebar
Ctrl+J: toggle BottomPanel
Ctrl+Shift+E: Explorer activity
Ctrl+Shift+F: Filters activity
Ctrl+Shift+H: History activity
Ctrl+Shift+I: toggle RightInspector
F1: Help activity or Help panel
Ctrl+P/Ctrl+Shift+P: command palette, depending current bindings
```

Implementation rule:
- Define commands in `core/commands.rs`.
- Route to `AppAction`.
- Let local focus scopes consume local commands before global fallbacks.

### Mouse

Rules:
- Dragging region dividers updates runtime state immediately.
- Save layout config after drag stop.
- Double-click divider can reset that region size to default.
- Right-click region headers can show panel-specific context menus later.

### Command Palette

Required commands:

```text
Workbench: Toggle Sidebar
Workbench: Toggle Bottom Panel
Workbench: Toggle Right Inspector
Workbench: Focus Editor
Workbench: Focus Results
Workbench: Open Explorer
Workbench: Open Filters
Workbench: Open History
Workbench: Reset Layout
```

Command palette should use `ActionContext` to boost commands relevant to the current region.

## AppAction Changes

Add semantic actions:

```rust
SetWorkbenchActivity(WorkbenchActivity)
TogglePrimarySidebar
SetPrimarySidebarVisible(bool)
ToggleBottomPanel
SetBottomPanelVisible(bool)
SetBottomPanelTab(BottomPanelTab)
ToggleRightInspector
SetRightInspectorVisible(bool)
SetRightInspectorTab(RightInspectorTab)
ResetWorkbenchLayout
FocusWorkbenchRegion(WorkbenchFocus)
```

Migration rule:
- Keep existing `ToggleSidebar`, `ToggleSqlEditor`, `ToggleErDiagram` during transition.
- Make old actions delegate to new actions when equivalent.

Mapping:

| Old action | New action |
|---|---|
| `ToggleSidebar` | `TogglePrimarySidebar` |
| `OpenFilterWorkspace` | `SetWorkbenchActivity(Filters) + SetPrimarySidebarVisible(true)` |
| `OpenHistoryPanel` | `SetWorkbenchActivity(History)` or `SetBottomPanelTab(History)` |
| `OpenHelpPanel` | `SetWorkbenchActivity(Help)` or Help editor tab |
| `ToggleSqlEditor` | Focus/open active `SqlDocument` |
| `ToggleErDiagram` | Open/close `ErDiagram` editor tab |

## Render Pipeline

Target frame flow:

```rust
pub fn run_frame(&mut self, root_ui: &mut egui::Ui) {
    let ctx = root_ui.ctx().clone();
    self.reconcile_active_dialog_owner();
    self.handle_messages(&ctx);
    self.handle_input_router(&ctx, &mut toolbar_actions);
    self.handle_zoom_shortcuts(&ctx);
    self.tick_notifications_and_config(&ctx);
    self.render_dialogs_and_collect_results(&ctx);
    self.render_workbench(root_ui);
    self.apply_collected_actions(&ctx);
    self.render_command_palette_and_toasts(&ctx);
}
```

Target shell:

```rust
fn render_workbench(&mut self, ui: &mut egui::Ui) {
    WorkbenchShell::show(ui, self.workbench_snapshot(), |region, ui| {
        match region {
            WorkbenchRegion::TopBar => self.render_top_bar(ui),
            WorkbenchRegion::ActivityBar => self.render_activity_bar(ui),
            WorkbenchRegion::PrimarySidebar => self.render_primary_sidebar(ui),
            WorkbenchRegion::EditorArea => self.render_editor_area(ui),
            WorkbenchRegion::BottomPanel => self.render_bottom_panel(ui),
            WorkbenchRegion::RightInspector => self.render_right_inspector(ui),
            WorkbenchRegion::StatusBar => self.render_status_bar(ui),
        }
    });
}
```

First slice can be less generic. The important part is separating shell regions from document content.

## Migration Plan

### Phase 0: Spec And Safety

Tasks:
- Keep this spec synced with design changes.
- Add tests around config defaults before changing rendering.

Acceptance:
- `cargo test config` or equivalent passes.
- Workbench config can deserialize from empty config.

### Phase 1: Config Foundation

Tasks:
- Add `WorkbenchConfig` and related config types to `core/config.rs`.
- Add defaults and normalization tests.
- Add `workbench` field to `AppConfig`.
- Copy legacy `sidebar.edge_transfer` into new config during load.

Acceptance:
- Existing config files load.
- New config files save with `[workbench]`.
- Invalid dimensions are clamped.

### Phase 2: WorkbenchState Foundation

Tasks:
- Add `src/state/workbench.rs`.
- Add `UiState.workbench`.
- Bridge existing `show_sidebar`, `sidebar_width`, `show_sql_editor`, `sql_editor_height`, and `show_er_diagram` to workbench state.

Acceptance:
- No visual behavior changes yet.
- Existing tests pass.

### Phase 3: Shell Wrapper

Tasks:
- Add `src/app/surfaces/workbench.rs`.
- Render TopBar globally.
- Render existing Sidebar in PrimarySidebar region.
- Render existing DockArea in EditorArea region.
- Add StatusBar.

Acceptance:
- Toolbar appears once globally.
- Existing sidebar and dock behavior still work.
- StatusBar shows active context.

### Phase 4: ActivityBar And Sidebar Activities

Tasks:
- Add ActivityBar.
- Add `WorkbenchActivity`.
- Map existing sidebar content into Explorer, Filters, Objects, History, Help, Settings.
- Stop stacking all side panels by default.

Acceptance:
- ActivityBar switches sidebar content.
- Filters activity opens from filter actions.
- Explorer remains default.
- Completed 2026-06-18 as a compatibility adapter over legacy sidebar panels.

### Phase 5: BottomPanel

Tasks:
- Add BottomPanel region and tabs.
- Move data grid rendering into Results tab.
- Move errors/messages into Messages tab.
- Add placeholder Explain and Tasks tabs.

Acceptance:
- Completed 2026-06-18.
- SQL editor remains visible while results update.
- Query error does not replace editor content.
- BottomPanel visibility, active tab, and height persist.

### Phase 6: Editor Tab Semantics

Tasks:
- Refactor `DockTab` variants to document/view names.
- Make SQL document the primary editor tab.
- Make ER diagram a first-class editor document.
- Decide table data tab behavior after BottomPanel proves stable.

Acceptance:
- Multiple SQL documents can be arranged.
- ER diagram can split beside SQL.
- Tab titles and close behavior remain correct.

### Phase 7: RightInspector

Tasks:
- Add RightInspector region.
- Route table schema inspection into Schema tab.
- Add row/cell detail views.
- Add ER selection detail.

Acceptance:
- Completed 2026-06-18.
- Inspect actions do not block or replace editor content.
- Inspector updates from selection context.

### Phase 8: Dialog Reduction

Tasks:
- Extract `HistoryPanel::show_in_ui()` for panel rendering.
- Move Help navigation into activity/editor view.
- Decide whether Keybindings stays workspace dialog or becomes Settings activity detail.

Acceptance:
- History and Help no longer obscure the main workbench by default.
- Modal dialogs remain only for transactional flows.

### Phase 9: Visual Polish

Tasks:
- Add workbench palette tokens.
- Replace inconsistent separators and emoji-only chrome.
- Tune density and sizing.
- Add layout reset command.

Acceptance:
- UI clearly reads as editor-style shell.
- Workbench regions are visually distinct.
- Keyboard and mouse affordances are discoverable.

## Testing Plan

### Unit Tests

Config:
- `WorkbenchConfig::default()` has expected defaults.
- Deserializing missing `[workbench]` creates defaults.
- Invalid widths/heights clamp correctly.
- Legacy `sidebar.edge_transfer` migrates.

State:
- `UiState::default()` initializes workbench state consistently.
- Focus bridge maps existing `FocusArea` correctly.

Actions:
- Activity actions set active activity and reveal sidebar.
- Bottom panel actions set active tab and visibility.
- Inspector actions set active tab and visibility.

### Rendering Tests

Use existing egui test patterns:
- Shell renders all expected regions.
- Hidden sidebar gives EditorArea full width except ActivityBar.
- Hidden bottom panel gives EditorArea full height.
- RightInspector width is clamped.
- TopBar is not rendered inside dock tab content.

### Regression Tests

Must keep working:
- Connect database.
- Select table.
- Run query.
- Edit grid cell.
- Save grid changes.
- Toggle ER diagram.
- Open command palette.
- Open help/history/keybindings.
- Keyboard focus routing.

### Manual Checks

Required:
- 1366x768 compact laptop.
- 1920x1080 standard desktop.
- Narrow window around 900 px width.
- Dark and light theme.
- UI scale 0.8, 1.0, 1.25.

## Acceptance Criteria For The Full Refactor

1. TopBar is global and stable.
2. ActivityBar controls PrimarySidebar content.
3. Sidebar no longer stacks all major panels by default.
4. SQL editing, result viewing, ER diagram, and inspection have distinct regions.
5. Query errors and messages appear in BottomPanel, not as a replacement for the editor.
6. Layout sizes and visibility persist across restart.
7. Help and History can be used without blocking the main workspace.
8. Command palette exposes every major workbench action.
9. Existing database workflows still pass tests.
10. The implementation follows the layer dependency rules.

## Implementation Order Recommendation

Do not start with visual polish. Start with configuration and shell:

```text
1. WorkbenchConfig
2. WorkbenchState
3. WorkbenchShell with global TopBar and StatusBar
4. ActivityBar
5. BottomPanel
6. Editor tab semantic refactor
7. RightInspector
8. Dialog reduction
9. Visual token polish
```

The first practical PR should be small:

```text
feat(ui): add workbench layout config
```

The second practical PR:

```text
refactor(ui): introduce workbench shell
```

The third practical PR:

```text
feat(ui): add activity bar and sidebar activities
```

## Risks

| Risk | Mitigation |
|---|---|
| Borrow conflicts around shell rendering | Keep app-bound orchestration in `src/app/surfaces/workbench.rs`; use snapshots for pure UI widgets |
| Dock tab identity breaks query state | Keep `QueryTabManager` as state owner until editor tab model is stable |
| Config churn creates invalid layouts | Clamp every dimension on load and before render |
| Keyboard routing regressions | Add focus bridge tests before deleting old `FocusArea` paths |
| Too-large PR | Ship phase by phase; do not combine config, shell, bottom panel, and dock refactor in one PR |
| Visual redesign breaks functionality | Preserve existing components first, then restyle after layout is stable |

## Non-Goals

- Multi-window support.
- Plugin system UI.
- Full dock layout serialization in first slice.
- Custom font system in first slice.
- Rewriting SQL editor internals.
- Rewriting data grid internals.

## Documentation Sync

When implementation starts, update:

- `~/.codex/references/workbench-ui-design.md` for high-level decisions.
- `~/.codex/references/workbench-ui-refactor-spec.md` for config or migration changes.
- `~/.codex/rules/ui-egui.md` for final rules after each phase lands.
- `CLAUDE.md` if repository architecture or module map changes.
- `docs/CHANGELOG.md` when user-visible behavior changes.
