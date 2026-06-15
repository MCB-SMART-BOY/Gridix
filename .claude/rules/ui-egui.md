---
paths:
  - src/ui/**/*.rs
  - src/app/surfaces/*.rs
  - src/app/dialogs/*.rs
---

# Gridix UI/egui rules

**Code is the source of truth.** Verify claims against `src/ui/` before relying on them. Update this file when you change UI code.

## egui_dock layout

The main workspace uses `egui_dock::DockArea` (v0.19) instead of manual layout:
- `src/ui/dock_tabs.rs` — `DockTab` enum, `WorkspaceViewer`, `sync_all()`, layout operations
- `src/app/mod.rs` — `dock_state: DockState<DockTab>` field
- `src/app/surfaces/render.rs` — `sync_all()` called before rendering; viewer delegates to app methods
- Panel toggles (Ctrl+R for ER, Ctrl+J for SQL editor) work via dock tab add/remove
- Tab close via dock's X button → `on_dock_tab_close()` cleanup lifecycle
- Never manually manipulate `allocate_ui_with_layout` for main content — use dock tabs

## Framework

- eframe 0.34.1 + egui 0.34.1. glow backend. X11 + Wayland.
- All widgets use immediate-mode egui — no retained widget tree
- State stored in structs with `#[derive(Default)]`, rendered via `ui(&mut self, ui: &mut egui::Ui)` methods

## Dialog shells

4 contracts in `ui/dialogs/common.rs`:
- **Blocking Modal**: confirm dialogs (delete, discard). Background overlay. Must dismiss before any other interaction.
- **Form Dialog Shell**: connection, export, import, DDL, create DB/user. Fixed footer, scrollable content, auto-reveal first validation error.
- **Workspace Dialog Shell**: command palette, help, keybindings, history. Movable, resizable, non-blocking.
- **Utility Overlay**: toolbar menus, theme chooser. Lightweight, click-outside dismiss.

Use the correct shell. Never render a raw `egui::Window` for a form dialog.

## Responsive widths

- Wide (≥720px): side-by-side field pairs
- Medium (560–720px): stacked same-row
- Narrow (<560px): fully stacked
- Test all three widths for new dialogs

## Visual tokens

Use helpers from `ui/styles.rs` — `theme_text()`, `theme_muted_text()`, `theme_accent()`, `theme_selection_fill()`, `theme_subtle_stroke()`.
Never hardcode colors. Derive from `egui::Visuals` for theme compatibility.

## Keyboard integration

Every interactive element needs:
1. A `LocalShortcut` variant in `shortcut_tooltip.rs`
2. A `config_key()` path (dot-notation, e.g., `"dialog.export.format_csv"`)
3. `default_bindings()` for the initial key assignment
4. `consume_local_shortcut()` call in the widget's input handling

## Large files

- `ui/components/grid/keyboard.rs` (1949 lines): normal mode (hjkl/w/b/gg/G/i/a/c/R/v/x/dd/yy/p/o/O), select mode, count prefix, sequences (`:w`, `gg`, `Space+d`), edge transfers
- `ui/dialogs/keybindings_dialog.rs` (3560 lines): full keybinding config UI with scoped command browser
- `ui/panels/sidebar/mod.rs` (1543 lines): 4 panels, draggable dividers, per-section keyboard nav

## ER diagram

ER is a workspace companion pane, NOT a dialog. Visibility: `show_er_diagram` via `set_er_diagram_visible_with_notice()`.
Focus: `FocusArea::ErDiagram`. Keyboard: `j/k`=select table, `h/l`=geometry nav, `Enter`=open, `Esc`=return, `q`=close, `Shift+L`=relayout, `v`=viewport toggle.
Use theme-derived tokens, not private ER colors. See `.claude/references/er-contracts.md`.
