# Dialog shell contracts & responsive design

From `docs/recovery/20-dialog-layout-audit.md` and `43-dialog-responsive-row-design.md`.

## 4 shell contracts

| shell | use for | behavior |
|---|---|---|
| **Blocking Modal** | Confirm dialogs (delete, discard) | Background overlay, must be dismissed before any other interaction |
| **Form Dialog Shell** | Connection, Export, Import, DDL, CreateDB, CreateUser | Fixed footer, scrollable content, auto-reveal first validation error |
| **Workspace Dialog Shell** | CommandPalette, Help, KeyBindings, History | Movable, resizable, non-blocking |
| **Utility Overlay** | Toolbar menus, theme chooser | Lightweight popup, click-outside dismiss |

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
- [ ] Blocking modal: background click does not dismiss, Escape does
- [ ] Form footer: stays fixed during scroll
- [ ] First validation error: auto-scrolls into view
