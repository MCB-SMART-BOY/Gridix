---
paths:
  - src/ui/**/*.rs
  - src/state/**/*.rs
---

# Gridix UI/egui rules

**Code is the source of truth.** Verify claims against `src/ui/` and `src/state/` before relying on them. Update this file when you change UI code.

## Layer architecture

```
src/state/  (Layer 3) — UiState struct, all rendering state
     ↑
src/ui/     (Layer 4) — DbManagerApp (~47 fields), rendering, input routing
     ↑
src/app/    (Layer 4) — input routing, surfaces, workflow, runtime
```

State owns the data; UI reads State and mutates it. Session is read-only during rendering.

## DbManagerApp (~47 fields)

```rust
pub struct DbManagerApp {
    session: Session,      // All DB + async state
    state: UiState,        // Theme + scale (partially migrated)
    // ~45 remaining fields: dialogs, grid, search, config, ER, dock, keybindings
}
```

**Migration status:** ~53 fields migrated to Session/State, ~47 remain. Target: 4 fields.

## egui_dock layout

The main workspace uses `egui_dock::DockArea` (v0.19):
- `src/ui/dock_tabs.rs` — `DockTab` enum, `WorkspaceViewer`, `sync_all()`
- `sync_all()` copies tab state from `self.session.tab_manager` to dock tree
- Tab close → calls session methods

## Framework

- eframe 0.34.1 + egui 0.34.1. glow backend. X11 + Wayland.
- All widgets use immediate-mode egui
- State stored in structs with `#[derive(Default)]`, rendered via `ui(&mut self, ui: &mut egui::Ui)` methods

## Dialog shells

4 contracts in `ui/components/dialogs/common.rs`:
- **Blocking Modal**: confirm dialogs (delete, discard)
- **Form Dialog Shell**: connection, export, import, DDL, create DB/user
- **Workspace Dialog Shell**: command palette, help, keybindings, history
- **Utility Overlay**: toolbar menus, theme chooser

## Borrow checker pattern

During rendering, Session is accessed as `&self.session` for reading. State accessed as `&mut self.state`. **Never take `&mut self.session` during rendering** — all Session mutations happen before or after the render pass.
