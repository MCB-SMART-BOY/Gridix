# ER diagram design contracts

Merged from `docs/recovery/44,47,48,49,50,51`. These govern ER diagram changes.

## Role

The ER diagram is a **workspace companion pane** — not a dialog, not full-screen, not a sub-app.
Visibility authority: `show_er_diagram` via `set_er_diagram_visible_with_notice()`.
Focus: `FocusArea::ErDiagram` in the `Sidebar → DataGrid → ErDiagram → SqlEditor` cycle.

## Keyboard flow

| key | scope | action |
|---|---|---|
| `j/k` | navigation | PrevTable / NextTable |
| `Shift+J/K` | navigation | PrevRelatedTable / NextRelatedTable |
| `Shift+Arrow` | navigation | PrevRelatedTable / NextRelatedTable (alt) |
| `h/l` | navigation | GeometryLeft / GeometryRight |
| `Enter/Right` | navigation | OpenSelectedTable |
| `Esc/Left` | navigation | ReturnToWorkspace |
| `q` | navigation | CloseDiagram |
| `Shift+L` | navigation | Relayout |
| `f` | navigation | FitView |
| `v` | navigation | ToggleViewportMode |
| `r` | navigation | Refresh |
| `+/-` | navigation | Zoom |
| `hjkl` | viewport | Pan (geometry movement) |
| `Esc` | viewport | ExitViewportMode |

## State ownership

20+ fields in `ERDiagramState`:
- **Loaded graph**: `tables`, `relationships` (produced by `load_er_diagram_data()`)
- **Viewport**: `viewport_offset`, `viewport_zoom` (viewport mode only)
- **Selection**: `selected_table_index`, `interaction_mode` (Focused/Viewport)
- **Lifecycle**: `loading`, `layout_snapshot` (for incremental stability), `load_generation`
- **Stale-guard**: `load_generation` is monotonic, bumped by `begin_loading()`/`clear()`. Async ER fetches (`ERTableColumnsFetched`/`ForeignKeysFetched`) carry the generation; handlers drop mismatched responses so a disconnected/old connection's schema cannot write into a new connection's ER state (audit B6-ER). Disconnect clears `er_diagram_state`, which bumps the generation.

## Token map (visual design)

ER uses theme-derived tokens, not private colors:
- **Shared**: backgrounds, text, borders, selection, toolbar chrome → from `ThemeColors`
- **Schema canvas**: canvas background, edge frame, low-alpha glow, minor/major grid
- **Database object cards**: table body/header, semantic accent rail, selected/related/dimmed border states, shadow, key-row fill, alternating row fill
- **Column affordances**: PK/FK badges, type text, nullable/default markers, hidden-column footer
- **Relationship affordances**: explicit/inferred/selected line colors, halo stroke, endpoint anchors, cardinality label pill

Do not regress ER rendering to plain boxes plus thin lines. `src/ui/components/er_diagram/render.rs` should keep the database-native schema-canvas language unless the user explicitly asks for another visual direction.

## Readability standards

1. Primary cluster anchored near center
2. Related tables packed as components
3. Isolated tables edge-placed, not thrown away
4. Dense graphs use stratification, not single-row compaction
5. Edge routing readable (lane separation, geometry-aware anchors)
6. Canvas utilization ≥60% (no wasted space)
7. Default completion state must look intentional, not like raw layout output
8. Empty/loading states use the same canvas card language as loaded diagrams

## Interaction correctness

1. Click selection must only select/clear; it must not leave `dragging_table` active.
2. Table dragging and canvas panning must use incremental pointer deltas from the previous frame, divided by current zoom.
3. Do not repeatedly add egui's total `Response::drag_delta()` to model coordinates.
4. Hit-testing should match draw order: when table cards overlap, choose the last-drawn/topmost table.
5. Selection clearing must synchronize `selected_table`, `pending_selection_reveal`, and every table's `selected` flag.

## Entry matrix

6 ways to open ER: ToggleErDiagram (`Ctrl+R`), FocusErDiagram (`Alt+R`), toolbar button, Help learning paths with ER, CloseWorkspaceOverlay, refresh while ER open.
