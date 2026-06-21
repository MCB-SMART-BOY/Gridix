# Gridix Workbench UI Design

## Goal

Gridix should feel like a database-focused editor workbench, closer to VS Code/Zed than a traditional dialog-heavy desktop utility.

2026-06-18 design pivot: fixed `ActivityBar | PrimarySidebar | EditorArea | RightInspector | BottomPanel` is now considered a compatibility bridge, not the final model. The next target is Dockable Workbench v2, where Explorer, Filters, Objects, History, Settings, Results, Tables, Inspector, and SQL/ER documents are peer dockable surfaces. See `~/.codex/references/dockable-workbench-v2.md` and `~/.codex/references/gridix-ui-visual-system-v2.md`.

2026-06-18 implementation status: the surface model, unified surface renderer, `DockTab::Surface`, `default_surface_layout()`, `ensure_surface_tab()`, reveal/open wiring, fixed fallback de-duplication, and runtime startup on the surface dock seed are implemented.

The compatibility bridge still appears as:

```text
TopBar
ActivityBar | PrimarySidebar | EditorArea | RightInspector
            |                | BottomPanel |
StatusBar
```

The final target is simpler:

```text
TopBar
Dockable Workspace
StatusBar
```

The current implementation already has useful pieces (`egui_dock`, sidebar panels, command palette, SQL editor, data grid, ER diagram), but the pieces are not assigned to a consistent workbench model. The redesign should mostly reorganize existing capabilities before adding new visual polish.

Implementation-level configuration, state, module, migration, and testing details are specified in `~/.codex/references/workbench-ui-refactor-spec.md`. The new dockable-surface target model is specified in `~/.codex/references/dockable-workbench-v2.md`.

## Design Principles

1. Stable chrome first. TopBar, ActivityBar, PrimarySidebar, EditorArea, BottomPanel, RightInspector, and StatusBar should be predictable and persistent.
2. EditorArea is for primary work documents. SQL documents, table data views, ER diagrams, schema views, and future diff/explain views belong here.
3. BottomPanel is for query output and diagnostics. Results, messages, query history, explain output, import/export logs, and background task output belong here.
4. PrimarySidebar is contextual navigation, not a dumping ground. Only one activity should dominate the sidebar at a time.
5. RightInspector is contextual detail. Selected table, column, row, ER node, result cell, or connection properties belong here.
6. Dialogs are only for blocking or transactional tasks. Connection creation, destructive confirmation, import/export wizard steps can remain dialogs; help/history/keybindings should become panels where practical.
7. Layout must persist. A workbench that forgets split sizes and panel visibility does not feel like an editor.

## Target Regions

### TopBar

Purpose: global command and context strip.

Contents:
- Sidebar/activity toggle
- Command palette trigger
- Active connection and database summary
- Run/cancel status
- Create/action menus
- Theme and scale controls

Rules:
- Do not render TopBar inside a dock tab.
- Keep height compact: 36-44 px.
- Avoid large emoji-only buttons as primary affordances. Use text labels for critical actions or consistent symbolic icons.

Current code state:
- `Toolbar::show_with_focus()` is called from `DbManagerApp::render_top_bar()` and rendered once above the sidebar/editor layout.
- EditorArea dock content must remain document/view content and must not render toolbar chrome.

### ActivityBar

Purpose: narrow vertical switcher for major side workspaces.

Width: 44-52 px.

Activities:
- Explorer: connections, databases, schemas, tables
- Filters: active result filters
- Objects: triggers, routines, users, indexes where available
- History: query history
- Help: learning and tool guide
- Settings: keybindings/theme/preferences

Rules:
- ActivityBar controls what appears in PrimarySidebar.
- Activity state is global and persistent.
- Use command palette for the same actions so keyboard-first use remains strong.

### PrimarySidebar

Purpose: current activity content.

Default width: 280 px.
Min/max: 220-460 px.

Explorer activity:
- Connection list
- Database/schema tree
- Table/view list
- Context actions for connect, refresh, edit, delete

Filters activity:
- Filter clauses for active result only
- Clear/add/toggle controls
- Should be empty-state aware when no result is active

Objects activity:
- Triggers, routines, users, indexes
- Selecting an object opens SQL definition or inspector detail

History activity:
- Query history list
- Selecting a query loads it into active SQL editor

Help activity:
- Learning navigation and help topics
- Large article content can open in EditorArea or RightInspector

Rules:
- Avoid stacking Explorer, Filters, Triggers, and Routines all at once by default.
- Internal vertical splitters should be exceptional, not the normal layout.

Initial code target:
- Replace `SidebarPanelState.show_connections/show_filters/show_triggers/show_routines` as the primary navigation model with a new `WorkbenchActivity`.
- Existing sidebar modules can be reused as activity views.

### EditorArea

Purpose: primary work documents managed by dock tabs.

Tab kinds:
- `SqlDocument`: SQL editor for a query tab
- `TableData`: editable result/table data grid
- `ErDiagram`: ER canvas
- `SchemaObject`: table/view/procedure definition
- `Welcome`: onboarding and empty state

Rules:
- `SqlDocument` should be a first-class editor tab, not a bottom scratchpad.
- Opening a table from Explorer should open a `TableData` tab or reuse the current table data tab based on user setting.
- ER diagram should be an editor tab that can split right/left like code editors.
- Welcome should be an editor tab/empty editor surface, not mixed into the data grid tab.

Recommended default:

```text
EditorArea tabs:
  [SQL: Query 1] [Table: users] [ER Diagram]
```

Initial code target:
- Implemented conservatively with `DockTab::SqlDocument { index, title }`, `TableData`, `ErDiagram`, `SchemaObject`, `Welcome`, and `AuxPanel`.
- Keep existing `QueryTabManager` as SQL document state owner until document identity is ready to move away from query-tab indices.

### BottomPanel

Purpose: output for the active editor/workflow.

Default height: 260 px.
Min/max: 140-55% viewport height.

Tabs:
- Results
- Messages
- Explain
- History
- Tasks

Rules:
- Query execution writes to Results and Messages.
- Explain output should not replace normal result data unless explicitly requested.
- BottomPanel can be hidden, maximized, or focused via keyboard.
- Result filters can live in PrimarySidebar but the filtered data is rendered in Results.

Initial code target:
- Move `DataGrid::show_editable()` out of QueryData editor content into `BottomPanel::Results`, while preserving grid state and keyboard focus.
- `WorkspaceSurface::QueryError` becomes `BottomPanel::Messages` content.

### RightInspector

Purpose: contextual metadata and properties.

Default width: 320 px.
Min/max: 260-480 px.

Views:
- Table schema
- Column metadata
- Row detail
- Cell full value
- ER node details
- Connection properties

Rules:
- RightInspector opens automatically for inspect actions, but should be easy to hide.
- It should not block the editor.
- It should follow active selection and show an empty state when no inspectable object exists.

Initial code target:
- Implemented through `WorkbenchRightInspector` and `RightInspectorState`.
- `show_table_schema` reveals Schema, ER node selection reveals ErSelection, Row/Cell read current Results selection, and Connection/Properties show non-sensitive context.

### StatusBar

Purpose: low-noise runtime status.

Height: 22-26 px.

Contents:
- Active connection/database
- Current table/object
- Row count / selected cell
- Query elapsed time
- Focus area and editor mode
- Background task status

Rules:
- StatusBar should replace scattered contextual labels where possible.
- It should stay visible unless the user enters a distraction-free mode.

Initial code target:
- Use existing `ActionContext::status_line()` as a first implementation source.

## Layout Persistence

Add a persisted workbench section to `AppConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchLayoutConfig {
    pub active_activity: WorkbenchActivity,
    pub primary_sidebar_visible: bool,
    pub primary_sidebar_width: f32,
    pub bottom_panel_visible: bool,
    pub bottom_panel_height: f32,
    pub bottom_panel_active_tab: BottomPanelTab,
    pub right_inspector_visible: bool,
    pub right_inspector_width: f32,
    pub right_inspector_active_tab: RightInspectorTab,
}
```

Recommended defaults:
- `active_activity = Explorer`
- `primary_sidebar_visible = true`
- `primary_sidebar_width = 280.0`
- `bottom_panel_visible = true`
- `bottom_panel_height = 260.0`
- `bottom_panel_active_tab = Results`
- `right_inspector_visible = false`
- `right_inspector_width = 320.0`

Dock layout persistence can come later. `egui_dock` serialization will likely require enabling its `serde` feature in `Cargo.toml`; do that only after the logical tab model is stable.

## State Model

Add workbench-specific UI state in `src/state/mod.rs`:

```rust
pub enum WorkbenchActivity {
    Explorer,
    Filters,
    Objects,
    History,
    Help,
    Settings,
}

pub enum BottomPanelTab {
    Results,
    Messages,
    Explain,
    History,
    Tasks,
}

pub enum RightInspectorTab {
    Properties,
    Schema,
    Row,
    Cell,
    ErSelection,
}
```

Keep the first implementation conservative:
- Do not remove existing sidebar panel state immediately.
- Introduce the workbench state and map existing UI into it.
- Remove old booleans only after behavior is covered by tests.

## Navigation Model

Keyboard focus order should become:

```text
TopBar -> ActivityBar -> PrimarySidebar -> EditorArea -> BottomPanel -> RightInspector -> StatusBar/none
```

Shortcut recommendations:
- `Ctrl+B`: toggle PrimarySidebar
- `Ctrl+J`: toggle BottomPanel
- `Ctrl+Shift+E`: Explorer activity
- `Ctrl+Shift+F`: Filters activity
- `Ctrl+Shift+H`: History activity
- `Ctrl+Shift+I`: RightInspector
- `F1` or command palette: Help activity/panel

Focus state should refer to workbench regions first, then local scopes inside each region.

## Visual Direction

Gridix should use an editor-like dense dark/light UI, not dialog-card-heavy spacing in the main shell.

Tokens to add:
- `surface_base`
- `surface_panel`
- `surface_sidebar`
- `surface_editor`
- `surface_bottom`
- `border_subtle`
- `border_active`
- `tab_active`
- `tab_inactive`
- `activity_active`
- `status_bar_bg`

Rules:
- Use separators sparingly; rely on panel backgrounds and 1 px borders.
- Use a single accent color per theme for active focus/selection.
- Reduce emoji usage in core chrome. Emojis can remain in welcome/help content.
- Keep row/grid density high, because database work is information-dense.

## Migration Plan

Current implementation status:
- Shell foundation, persisted workbench config, StatusBar, global TopBar, ActivityBar-driven PrimarySidebar activity switching, BottomPanel result/message routing, EditorArea document/view dock tab semantics, and contextual RightInspector are implemented.
- Explorer/Filters/Objects currently adapt legacy sidebar panels; History/Help/Settings are placeholder activities that open existing panels/dialogs.
- Next major UI migration is dialog reduction: Help/History/Settings should move toward panel/activity surfaces.

### Phase 1: Shell Foundation

Goal: introduce stable workbench shell without moving major features yet.

Tasks:
- Create `WorkbenchShell` renderer in `src/app/surfaces/workbench.rs` or similar.
- Move TopBar out of dock document content.
- Add ActivityBar placeholder and StatusBar. Completed historically; after the topbar-first correction, the ActivityBar widget is dormant and is not rendered in the default runtime layout.
- Keep existing Sidebar and DockArea behavior mounted in the shell.
- Persist sidebar width and bottom panel visibility/height in `AppConfig`.

Acceptance:
- Current workflows still work.
- Toolbar appears once globally.
- Restart preserves sidebar width and bottom panel state.

### Phase 2: Sidebar Activities

Goal: make PrimarySidebar mode-driven.

Tasks:
- Add `WorkbenchActivity`.
- Map existing connection/table view to Explorer.
- Move filters into Filters activity.
- Move triggers/routines into Objects activity.
- Add empty states for activities with no active result/connection.

Acceptance:
- Sidebar no longer stacks all panels by default.
- Activity switching works by mouse and keyboard.
- Existing sidebar keyboard workflow remains usable within an activity.

Status: implemented as a compatibility adapter over legacy sidebar panels.

### Phase 3: BottomPanel Results

Goal: make result output a bottom panel.

Tasks:
- Add `BottomPanelTab`.
- Move data grid rendering to `BottomPanel::Results`.
- Move query errors/messages to `BottomPanel::Messages`.
- Prepare `Explain` tab for future query plan visualization.

Acceptance:
- SQL editor can remain visible while results update below it.
- Running a query opens/focuses Results unless user pinned another tab.
- Errors no longer replace the main editor surface.

Status: implemented as a compatibility adapter. Query output no longer renders in the editor document surface; `BottomPanel::Results` owns DataGrid and `BottomPanel::Messages` owns query errors/status.

### Phase 4: Editor Tab Model

Goal: make dock tabs represent documents/views.

Tasks:
- Rename/rework `DockTab` variants around document semantics.
- Create `SqlDocument` tabs from `QueryTabManager`.
- Open table data, ER diagram, and schema views as editor tabs.
- Remove the old query-output/editor split ambiguity.

Acceptance:
- Multiple SQL documents can be opened and arranged.
- ER diagram can be split beside SQL or table data.
- Table data tabs have stable identity by connection/database/table.

Status: implemented for SQL document semantics. `SqlDocument` replaced the old query-output/editor split, ER remains an EditorArea view, and `TableData`/`SchemaObject`/`Welcome` target variants are reserved for later wiring.

### Phase 5: RightInspector

Goal: add contextual property panel.

Tasks:
- Add `RightInspectorState`.
- Route table schema, selected row/cell detail, and ER selection detail into inspector.
- Keep blocking dialogs only for transactional actions.

Acceptance:
- Inspecting a table does not overwrite SQL or result surfaces.
- Selecting cells/ER nodes updates inspector context.

### Phase 6: Panel Migration

Goal: reduce dialog-heavy UX.

Tasks:
- Move History into BottomPanel or PrimarySidebar activity.
- Move Help into Help activity/editor document.
- Keep Keybindings as a workspace-style panel or large dialog depending on editing complexity.

Acceptance:
- Help/history no longer obscure the main workspace by default.
- Command palette can open every panel and activity.

## First Implementation Slice

The safest first slice is:

1. Add `WorkbenchActivity`, `BottomPanelTab`, and layout config defaults.
2. Add a `WorkbenchShell` function around existing content.
3. Move Toolbar out of `render_workspace_content()`.
4. Add StatusBar using existing `ActionContext::status_line()`.
5. Persist sidebar width and SQL/bottom panel height.

Do not move DataGrid or SQL editor in the first slice. That keeps risk low and creates the foundation for later phases.

## Non-Goals For First Slice

- Do not redesign every component visually.
- Do not replace `egui_dock`.
- Do not rewrite `QueryTabManager`.
- Do not move Help/History immediately.
- Do not add multi-window support.

## Open Decisions

- Whether table data should open as an EditorArea tab or always render in BottomPanel Results.
- Whether History belongs primarily in PrimarySidebar or BottomPanel.
- Whether RightInspector should open by default on first table selection.
- Whether dock layout persistence should serialize full `DockState` or use a simpler custom layout model first.
