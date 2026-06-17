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
- Config version: 2 (with #[serde(default)] for backward compat)
- Config save: 5-second debounce via save_config_debounced()
- Handler repaint: needs_repaint flag replaces ctx.request_repaint()

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

## Build Commands

```bash
cargo test                                     # Full test suite
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```
