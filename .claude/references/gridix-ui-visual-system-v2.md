# Gridix UI Visual And Interaction System v2

## Purpose

Gridix should reach the interaction quality of VS Code and Zed, but remain database-native. The goal is not to copy their pixels; it is to copy the discipline:

- simple default layout
- strong visual hierarchy
- icons for repeated navigation
- text only where real-time meaning matters
- predictable dockable surfaces
- consistent panel anatomy
- surface-specific power features
- keyboard-first operation with discoverable mouse affordances

This document complements `dockable-workbench-v2.md`. That file defines layout architecture. This file defines the visual and interaction system.

## Implementation Status

Last updated: 2026-06-18.

- Surface foundation exists: `WorkbenchSurfaceKind` descriptors carry icon, title, description, command ID, role, default placement, allowed placements, singleton flag, and persistence key.
- Unified surface shell foundation exists in `src/ui/workbench/surface.rs`: `WorkbenchSurfaceHeader`, `SurfaceAction`, icon-only button helper, and shared tooltip rendering.
- BottomPanel and RightInspector close controls now use the shared icon-only action tooltip contract.
- Phase C bridge exists: fixed-region clicks can set `WorkbenchFocus::Surface`, and editor dock tabs render through the unified surface renderer.
- Phase C seed layout exists: `DockTab::Surface`, `default_surface_layout()`, and `ensure_surface_tab()` can place Explorer/Results/Inspector as peer dock items.
- Reveal/open actions now route into the surface dock for activity, output, inspector, and ER surfaces.
- Fixed-region fallback de-duplication exists: PrimarySidebar, BottomPanel, and RightInspector no longer duplicate layout space when the active equivalent docked surface exists.
- Runtime startup now uses the surface dock seed, so Explorer/SQL/Results/Inspector are peer surfaces by default.
- Next step: migrate fixed-region headers/tabs into the shared surface shell, add a consistent core icon set, and then remove redundant fixed regions.

## Design Diagnosis

Current problems:

1. UI regions exist, but they do not feel like one system.
2. Too many labels compete for attention.
3. Panels have inconsistent purpose: some are navigation, some are output, some are inspectors, but they use similar visual weight.
4. The ActivityBar duplicates concepts already available in TopBar/commands.
5. BottomPanel and RightInspector work, but they are fixed-region implementations rather than user-movable surfaces.
6. The UI is more "utility app with sections" than "editor workbench".

Correct target:

```text
TopBar          global context and commands
SurfaceRail     compact icon launcher, optional and movable later
DockWorkspace   all surfaces as draggable/dockable peers
StatusBar       quiet state line
```

## North Star

Gridix should look calm by default and become rich only when the user asks for depth.

Default impression:

- one visible SQL document
- one visible data/result surface
- one compact Explorer surface
- no duplicate panels
- no permanent text-heavy rails
- no dialog unless the action is transactional

Advanced impression:

- user can split Explorer, Results, Inspector, TableData, ER, History, Help, and Settings into any dock area
- surface-specific controls appear in the surface header
- every icon explains itself through tooltip and command metadata

## Information Hierarchy

### Always Visible

Keep these minimal:

- TopBar: connection/database breadcrumb, command palette, run/cancel state, create/action menu, layout preset, theme/density
- StatusBar: active connection, selected object, query timing, row count, focus/surface
- Dock tab titles: only for surfaces where identity matters

### Visible On Demand

- Explorer
- Filters
- Objects
- History
- Settings
- Inspector
- Messages
- Tasks

### Hidden Until Needed

- Help article body
- keybinding editor detail
- import/export preview internals
- schema column metadata if no schema/ER inspect action happened

## Icon-First Rule

Use icons instead of text labels for repeated chrome:

- SurfaceRail items
- surface move/dock actions
- close/pin/split/maximize actions
- filter add/remove/toggle controls
- result export/save/refresh controls
- inspector tab icons

Use text labels when:

- the value changes in real time and cannot be inferred from icon alone
- destructive action requires clarity
- user is in onboarding or empty state
- table/connection/database names must be read
- command palette/search result lists need readable titles

## Tooltip Contract

Every icon-only control must have a tooltip with:

```text
功能名称
一句话作用
快捷键或命令 ID（如果存在）
```

Examples:

```text
Explorer
显示连接、数据库和表
Command: workbench.surface.explorer
```

```text
Run Query
执行当前 SQL 文档
Ctrl+Enter
```

```text
Dock Right
将当前 surface 移动到右侧 dock
Command: workbench.surface.moveRight
```

No icon-only button is acceptable without hover text.

## Unified Panel Anatomy

Every dockable panel/surface should share this skeleton:

```text
SurfaceHeader
  left: icon + optional title/context chip
  center: optional search/filter/breadcrumb
  right: scoped actions
SurfaceBody
  content
SurfaceFooter optional
  compact state, count, selected item, errors
```

Rules:

- Header height: 30-34 px compact.
- Header icon is always present.
- Header title can be hidden in compact mode if tab title already shows identity.
- Scoped actions align right and are icon-only with tooltips.
- Body uses the same padding scale across surfaces.
- Empty states use the same visual template.
- Surface-specific controls live in the header, not TopBar.

## Surface Classes

### Document Surfaces

Examples:

- `SqlDocument`
- `TableData`
- `ErDiagram`
- `SchemaObject`
- `HelpArticle`

Visual behavior:

- can live in center dock
- tab title should show identity
- larger body area
- local toolbar is sparse
- prefer no permanent footer unless state matters

### Navigation Surfaces

Examples:

- `Explorer`
- `Objects`
- `History`
- `SettingsIndex`

Visual behavior:

- tree/list density is high
- icons carry object type
- title text is secondary
- selection highlight must be strong
- local search should be one key away

### Output Surfaces

Examples:

- `QueryResult`
- `Messages`
- `Explain`
- `Tasks`

Visual behavior:

- can dock bottom by default, but not limited to bottom
- real-time labels are allowed: row count, duration, affected rows
- grid/result controls use icons plus tooltip
- errors use semantic color, not oversized banners

### Inspector Surfaces

Examples:

- `Inspector::Schema`
- `Inspector::Row`
- `Inspector::Cell`
- `Inspector::Connection`
- `Inspector::ErSelection`

Visual behavior:

- property rows are compact
- value text is selectable
- secret values are never shown
- inspector mode tabs should be icons first
- full names appear in tooltip

### Utility Surfaces

Examples:

- `Settings`
- `Keybindings`
- `Help`

Visual behavior:

- can dock center or side
- should avoid modal behavior
- use two-pane layout only when width allows
- collapse navigation automatically on narrow width

## SurfaceRail

The old ActivityBar should evolve into `SurfaceRail`.

Purpose:

- reveal/focus surfaces
- show active/available surfaces
- provide drag handles for surface placement later

Rules:

- icon-only by default
- 40-44 px wide in compact mode
- labels hidden unless expanded
- no surface content is owned by the rail
- rail can be hidden
- top/bottom placement is optional later

Default icons:

| Surface | Icon intent |
|---|---|
| Explorer | database tree |
| Search/Filters | funnel |
| Objects | connected nodes |
| Results | table/grid |
| Messages | terminal/log |
| ER | graph |
| History | clock |
| Inspector | info/sidebar |
| Settings | gear |
| Help | question mark |

Actual icon set should be consistent. Do not mix emoji with vector-style icons in core chrome.

## Typography

Use a compact, editor-like hierarchy:

- UI text: 12-13 px
- surface header: 12 px semibold
- tab text: 12 px
- data grid: 12-13 px monospace or tabular
- SQL editor: existing editor font
- empty state title: 16 px

Avoid large headings in work surfaces unless the surface is an empty state or onboarding screen.

## Color System

Use semantic tokens, not ad hoc colors:

```text
surface.base
surface.raised
surface.sunken
surface.header
surface.tab.active
surface.tab.inactive
surface.rail
surface.rail.active
border.subtle
border.focus
border.drag_target
text.primary
text.secondary
text.muted
accent.primary
accent.warning
accent.danger
accent.success
data.null
data.modified
data.pk
data.fk
```

Rules:

- one accent color per theme
- active surface gets a focus border or stripe, not a loud background
- dangerous/destructive colors only appear on destructive actions or errors
- data grid change state uses semantic data tokens
- icons inherit text-secondary by default, accent on active/hover

## Density Modes

Support three modes eventually:

```text
Compact      default for editor-style use
Comfortable  larger touch/mouse targets
Dense        high-information database work
```

Initial implementation should only tune Compact, but code should not hard-code one-off sizes.

## Motion

Motion should be functional:

- surface reveal: 80-120 ms fade/slide if egui supports it cleanly
- drag target highlight: immediate
- tab reorder/drop feedback: immediate
- no decorative animation for query execution

If animation makes layout feel unstable, remove it.

## Panel-Specific Features

Unified skeleton does not mean identical content. Each surface should have one or two feature-specific controls:

- Explorer: connection scope, refresh, new connection, reveal active table
- Filters: add rule, clear all, AND/OR toggle, focus value
- Objects: object type filter, load definition, open in SQL
- Results: export, save changes, refresh, filter, pin result, open as table surface
- Messages: copy log, clear, severity filter
- Inspector: pin context, follow selection, copy value
- History: replay, copy SQL, pin favorite, filter by connection
- Settings: search, open keybindings, reset layout
- ER: fit view, relayout, density, edge mode

Keep these local to the surface header.

## Layout Presets

Offer simple presets instead of forcing users to build from scratch:

1. `Classic`: Explorer left, SQL center, Results bottom, Inspector right.
2. `Minimal`: SQL center, Results hidden until query, SurfaceRail hidden.
3. `Data Review`: Explorer left, TableData center, Inspector right, Messages bottom.
4. `Schema`: Explorer left, ER center, Inspector right, Objects bottom/side.
5. `Laptop`: single column center, surfaces grouped as tabs.

Presets are commands, not separate UI modes.

## Interaction Rules

### Dragging

- Drag a surface tab to split center/left/right/bottom.
- Drag a surface icon from SurfaceRail to create/reveal placement.
- Dragging should show a clear drop target overlay.
- Dropping Explorer/Inspector/Results into center is valid.

### Pinning

Surfaces can be:

- transient: follows current selection
- pinned: keeps its context

Examples:

- Inspector follows selected cell by default.
- Pinning Inspector freezes it to the current cell/table.
- QueryResult may be pinned to keep old query output while running new SQL.

### Focus

Focus should be surface-based, not fixed-region-based:

```rust
WorkbenchFocus::Surface(WorkbenchSurfaceId)
```

Legacy `FocusArea` should become an adapter only.

### Commands

Every UI action should be reachable by command palette:

- reveal/focus surface
- move surface to placement
- close surface
- pin/unpin surface
- reset layout
- apply layout preset

## Anti-Patterns

Do not:

- add another permanent side panel for a new feature
- put feature-specific controls in TopBar
- show text labels for every repeated icon
- duplicate the same feature in TopBar, rail, sidebar, and panel
- use dialogs for read-only or navigational information
- let every panel invent its own header layout
- mix emoji icons with vector-style core chrome
- make the default layout show every available surface

## Implementation Priorities

### Priority 1: Surface Foundations

- `WorkbenchSurfaceKind`
- `WorkbenchSurfaceRole`
- `WorkbenchPlacement`
- `WorkbenchSurfaceId`
- surface descriptor registry
- icon key and tooltip metadata

### Priority 2: Unified Surface Shell

- `WorkbenchSurfaceHeader`
- icon-only action buttons
- tooltip contract
- shared empty state
- shared property row / list row styles

### Priority 3: Dockable Workspace

- one dock tree for workspace surfaces
- fixed regions become default dock placements
- Explorer/Results/Inspector become surfaces

### Priority 4: Visual Cleanup

- replace text-heavy ActivityBar with SurfaceRail
- remove redundant labels
- add consistent icons
- move surface-specific controls into headers

### Priority 5: Dialog Reduction

- History surface
- Help surface
- Settings/keybindings surface

Do not start Priority 5 before Priority 1-2 are in place.

## Acceptance Criteria

The UI is not done until:

- default screen has no redundant feature duplication
- every icon-only control has tooltip + command/shortcut metadata
- Explorer can conceptually dock anywhere
- Results/TableData can conceptually dock anywhere
- Inspector can dock anywhere
- all panels share one shell anatomy
- panel-specific features live in local headers
- TopBar is only global context/command chrome
- layout can be reset to presets
- dialogs are reserved for transactional flows
