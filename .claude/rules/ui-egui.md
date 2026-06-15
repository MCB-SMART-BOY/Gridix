---
paths:
  - src/ui/**/*.rs
  - src/state/**/*.rs
  - src/ui/surfaces/*.rs
---

# Gridix UI/egui rules

**Code is the source of truth.** Verify claims against `src/ui/` and `src/state/` before relying on them. Update this file when you change UI code.

## Layer architecture

```
src/state/  (Layer 3) — UiState struct, all rendering state, apply_frame_effects()
     ↑
src/ui/     (Layer 4) — DbManagerApp (4 fields), rendering, input routing
```

State owns the data; UI reads State (immutable) and mutates State (for UI-specific changes). Session is read-only during rendering.

## DbManagerApp (4 fields)

```rust
pub struct DbManagerApp {
    session: Session,
    state: UiState,
    config: AppConfig,
    keybindings: KeyBindings,
}
```

Down from ~100 fields. Session → FrameEffects → State → render.

## egui_dock layout

The main workspace uses `egui_dock::DockArea` (v0.19):
- `src/ui/dock_tabs.rs` — `DockTab` enum, `WorkspaceViewer`, `sync_all()`, layout operations
- `sync_all()` copies tab state from `self.session.tab_manager` to dock tree
- Tab close via dock's X button → calls `self.session.close_tab()` 
- Panel toggles (Ctrl+R for ER, Ctrl+J for SQL editor) work via dock tab add/remove

## Framework

- eframe 0.34.1 + egui 0.34.1. glow backend. X11 + Wayland.
- All widgets use immediate-mode egui — no retained widget tree
- State stored in structs with `#[derive(Default)]`, rendered via `ui(&mut self, ui: &mut egui::Ui)` methods

## Dialog shells

4 contracts in `ui/components/dialogs/common.rs`:
- **Blocking Modal**: confirm dialogs (delete, discard). Background overlay. Must dismiss before any other interaction.
- **Form Dialog Shell**: connection, export, import, DDL, create DB/user. Fixed footer, scrollable content, auto-reveal first validation error.
- **Workspace Dialog Shell**: command palette, help, keybindings, history. Movable, resizable, non-blocking.
- **Utility Overlay**: toolbar menus, theme chooser. Lightweight, click-outside dismiss.

Use the correct shell. Never render a raw `egui::Window` for a form dialog.

## Responsive widths

- Wide (≥720px): side-by-side field pairs
- Medium (560–720px): stacked same-row
- Narrow (<560px): fully stacked

## Visual tokens

Use helpers from `ui/styles.rs` — `theme_text()`, `theme_muted_text()`, `theme_accent()`, `theme_selection_fill()`, `theme_subtle_stroke()`.
Never hardcode colors. Derive from `egui::Visuals` for theme compatibility.

## Keyboard integration

Every interactive element needs:
1. A `LocalShortcut` variant in `shortcut_tooltip.rs`
2. A `config_key()` path (dot-notation, e.g., `"dialog.export.format_csv"`)
3. `default_bindings()` for the initial key assignment
4. `consume_local_shortcut()` call in the widget's input handling

## ER diagram

ER is a workspace companion pane, NOT a dialog. Visibility: `show_er_diagram` via `set_er_diagram_visible_with_notice()`.
Focus: `FocusArea::ErDiagram` in the `Sidebar → DataGrid → ErDiagram → SqlEditor` cycle.
Keyboard: `j/k`=select table, `h/l`=geometry nav, `Enter`=open, `Esc`=return, `q`=close, `Shift+L`=relayout, `v`=viewport toggle.
Use theme-derived tokens, not private ER colors. See `.claude/references/er-contracts.md`.

## Borrow checker pattern

During rendering, Session is accessed as `&self.session` for reading. State is accessed as `&mut self.state` for UI mutations. These are guaranteed disjoint by Rust's borrow checker (different fields of DbManagerApp). **Never take `&mut self.session` during rendering** — all Session mutations happen via methods that run before or after the render pass.
