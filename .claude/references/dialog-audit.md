# Dialog shell contracts & responsive design

From `docs/recovery/20-dialog-layout-audit.md` and `43-dialog-responsive-row-design.md`.

## 4 shell contracts

| shell | use for | behavior |
|---|---|---|
| **Blocking Modal** | Confirm dialogs (delete, discard) | Background overlay, must be dismissed before any other interaction |
| **Form Dialog Shell** | Connection, Export, Import, DDL, CreateDB, CreateUser, SchemaDiff | Fixed footer, scrollable content, auto-reveal first validation error |
| **Workspace Dialog Shell** | CommandPalette, Help, KeyBindings, History | Movable, resizable |
| **Utility Overlay** | Toolbar menus, theme chooser | Lightweight popup, click-outside dismiss |

## Layer blocking contract

`DialogShell::show_blocking` is the blocking path: it lays a full-screen interactive area
above the workbench but below the dialog window (`show_pointer_blocker`) and registers the
dialog layer as the frame's modal layer (`register_modal_layer`), so pointer and keyboard
stop at the dialog. `DialogShell::show` renders a plain window and is reserved for the
popup family (toolbar menus, theme chooser). Confirm dialogs use `DialogWindow::blocking`,
which relies on the `egui::Modal` backdrop and registers the modal layer through
`egui::Modal::show`. Oversized requests are clamped by `DialogStyle::responsive_widths` /
`responsive_heights` and `constrain_to(content_rect)`, so a dialog cannot grow past the
viewport.

egui's widget hit test does not consult the modal layer (`hit_test` filters by `Order`
only), so the modal layer alone cannot stop background clicks; the pointer blocker owns
that half of the contract. `src/ui/dialogs/common.rs` tests
`dialog_shell_blocking_swallows_clicks_on_lower_layers` and
`dialog_shell_without_blocking_lets_lower_layers_receive_clicks` pin both directions.

Escape closes the workspace overlays (Help, History, ER) through
`InputRouterContext::resolve_escape_fallback`; each dialog consumes Escape inside its own
window.

## Responsive row widths

Three width classes for dialog content rows:
- **Wide** (≥720px): side-by-side field pairs
- **Medium** (560–720px): stacked but same-row
- **Narrow** (<560px): fully stacked

High-risk dialogs that were fixed: ConnectionDialog, ImportDialog, DdlDialog.
Low-frequency dialogs to check at narrow widths: CreateDbDialog, CreateUserDialog, ExportDialog.

## Verification checklist

- [ ] Narrow viewport (560px): no horizontal overflow in any dialog
- [ ] Medium viewport (640px): form rows wrap correctly
- [ ] Wide viewport (800px): field pairs render side-by-side
- [ ] Dialog surface: clicks on sidebar/toolbar/workbench do nothing while a dialog is open, dialog controls stay live
- [ ] Escape closes the active dialog in an oversized/late-opened viewport instead of being swallowed
- [ ] Confirm dialogs: backdrop must be dismissed before any other interaction
- [ ] Form footer: stays fixed during scroll
- [ ] First validation error: auto-scrolls into view
