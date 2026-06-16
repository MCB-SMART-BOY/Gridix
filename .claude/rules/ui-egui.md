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

## Dialog shells

4 contracts in `ui/components/dialogs/common.rs`: Blocking Modal, Form Dialog Shell, Workspace Dialog Shell, Utility Overlay.

## Borrow checker pattern

During rendering: `&self.session` (read-only) + `&mut self.state` (UI mutations). Disjoint fields — guaranteed safe by Rust.
