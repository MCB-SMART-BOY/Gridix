---
name: project-context
description: Current project state, architecture, and active constraints
metadata:
  type: project
---

# Gridix Project Context

## Architecture State

- 6-layer unidirectional architecture: types(-1) ← core(0) ← data(1) ← session(2) ← state(3) ← ui/app(4)
- DbManagerApp: ~11 fields (from ~100). 89 migrated to Session(~30) + UiState(~60).
- self.sql dual source: ELIMINATED. Sole authority = QueryTab.sql via active_sql()/set_active_sql()
- database/ renamed to data/
- Config version: 3 (additive `AppConfig.workbench`, with `#[serde(default)]` for backward compat)
- Config save: 5-second debounce via save_config_debounced()
- Handler repaint: needs_repaint flag replaces ctx.request_repaint()
- Workbench shell foundation exists: runtime state in `src/state/workbench.rs`, shell widgets in `src/ui/workbench/`, app adapters in `src/app/surfaces/workbench.rs`, global TopBar rendering, ActivityBar-driven PrimarySidebar activity switching, BottomPanel result/message routing, EditorArea document/view dock tabs, contextual RightInspector, Dockable Workbench v2 surface descriptors, `WorkbenchFocus::Surface`, `DockTab::Surface`, `default_surface_layout()`, `ensure_surface_tab()`, unified workbench surface rendering, and shared surface header/action chrome

## Key Constraints

- NO batch sed for field migration — causes cross-struct corruption
- NO trait objects for DB backends — use match db_type
- NO new cross-layer imports without documenting in architecture/decisions.md
- EVERY DialogId variant must be handled in ALL match arms in host.rs
- Field migration: add to target struct FIRST, then migrate ONE ref at a time

## Active Tech Debt

- FrameEffects defined in session/frame_effects.rs but not wired
- 72 source files with zero test coverage (data/query drivers, grid filter)
- 3 oversized files: keybindings_dialog(3560L), input_router(3369L), keybindings(2448L)
- Session fields are all pub (acceptable for single-crate project)
- Workbench migration is partial and has a design pivot: fixed ActivityBar/PrimarySidebar/BottomPanel/RightInspector are compatibility adapters, not the final model. Target is Dockable Workbench v2 (`references/dockable-workbench-v2.md`) plus UI Visual System v2 (`references/gridix-ui-visual-system-v2.md`): Explorer/Filters/Objects/History/Settings/Results/Tables/Inspector/SQL/ER become peer dockable surfaces with a shared icon-first panel shell. Surface model types, descriptor metadata, `DockTab::surface_kind()` bridge, shared surface action/header chrome, unified surface renderer bridge, runtime surface dock seed, reveal/open action wiring into `ensure_surface_tab()`, and fixed-region fallback de-duplication are implemented; next UI slice should migrate remaining fixed-region chrome into the shared surface shell before continuing dialog reduction.

## Build Commands

```bash
cargo test                                     # Full test suite
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test && cargo run --bin check-doc-links
```
