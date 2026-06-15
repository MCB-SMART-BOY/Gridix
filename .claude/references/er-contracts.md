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
- **Lifecycle**: `loading`, `layout_snapshot` (for incremental stability)

## Token map (visual design)

ER uses theme-derived tokens, not private colors:
- **Shared**: backgrounds, text, borders, selection, toolbar chrome → from `ThemeColors`
- **ER-specific** (5 tokens): relation line color, key marker, empty state, loading indicator, canvas background

## Readability standards

1. Primary cluster anchored near center
2. Related tables packed as components
3. Isolated tables edge-placed, not thrown away
4. Dense graphs use stratification, not single-row compaction
5. Edge routing readable (lane separation, geometry-aware anchors)
6. Canvas utilization ≥60% (no wasted space)
7. Default completion state must look intentional, not like raw layout output

## Entry matrix

6 ways to open ER: ToggleErDiagram (`Ctrl+R`), FocusErDiagram (`Alt+R`), toolbar button, Help learning paths with ER, CloseWorkspaceOverlay, refresh while ER open.
