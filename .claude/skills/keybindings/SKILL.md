---
name: gridix-keybindings
description: Add, modify, or verify Gridix keyboard shortcuts. Use only in the Gridix repository when asked to change a shortcut, fix key routing, add a keybinding, or understand the keyboard system.
paths:
  - src/core/keybindings.rs
  - src/core/commands.rs
  - src/ui/shortcut_tooltip.rs
  - src/app/input/input_router.rs
  - src/app/action/action_system.rs
---

# Keyboard system

Gridix is keyboard-first. The binding system has 4 layers across 5 files.

## Architecture

```
Keypress → input_router.rs (8-stage pipeline)
         → InputContextSnapshot (per-frame state capture)
         → FocusScope → keymap_scope_path()
         → KeyBindings (keymap.toml lookup)
         → AppAction (54 variants) → AppEffect
         → apply_app_effects()
```

## Layer reference

| layer | file | what it defines |
|---|---|---|
| Keymap engine | `core/keybindings.rs` | `Action` (38 variants), `KeyBindings`, `KeyBinding::parse()`, scope_resolution_chain(), conflict detection |
| Command registry | `core/commands.rs` | ~100 `ScopedCommand` entries with `default_bindings` |
| UI shortcuts | `ui/shortcut_tooltip.rs` | `LocalShortcut` (138 variants), `config_key()`, runtime overrides |
| Routing pipeline | `app/input/input_router.rs` | `FocusScope`, `resolve_input_action_with()`, `TextEntryGuard` |
| Action system | `app/action/action_system.rs` | `AppAction` → `AppEffect`, `command_descriptors()`, availability |

## Scope inheritance

Child scopes inherit from parents:
```
editor.insert → editor → workspace routing
dialog.help.scroll_up → dialog.help → dialog.common → workspace routing
sidebar.filters.input → sidebar.filters → sidebar.list → workspace routing
```

Defined in `keybindings.rs::scope_resolution_chain()`.

## Conflict detection (3 levels)

1. **Same-scope**: two commands share a key → Error diagnostic
2. **Parent-shadowing**: parent + child share key → Warning (child wins)
3. **Text-entry conflict**: unmodified alphanumeric in text-entry scope → Error

Diagnostics shown in KeyBindings dialog, never written to disk.

## Persistence

Active keymap: `~/.config/gridix/keymap.toml` (TOML, atomic write, 0o600).
Legacy `config.toml.keybindings` field is read-only for migration.

## Change recipes

### Change a default binding
Find command in `core/commands.rs` → edit `default_bindings` → `cargo test -p gridix --lib keybindings`.

### Add a scoped command
Add `scoped_command!()` entry in `core/commands.rs` with id, description, default_bindings. If it needs a LocalShortcut, add variant in `shortcut_tooltip.rs`.

### Add a global action
Add `AppAction` variant → `CommandDescriptor` → availability → reduction arm in `action_system.rs`. If it needs a global key, add to `Action` enum + `default_bindings()` in `keybindings.rs`.

BottomPanel layout actions currently exist only as command-palette actions unless a key is added: `ToggleBottomPanel`, `SetBottomPanelVisible(bool)`, and `SetBottomPanelTab(BottomPanelTab)`.

### Verify a keybinding works
```bash
source "$HOME/.claude/skills/run-gridix/driver.sh"
launch
key Ctrl+N          # test shortcut
ss result
quit
```

## Doc vs code

**Code is the source of truth.** Always verify keybindings against `input_router.rs::ErDiagramLocalAction` and `core/keybindings.rs` — not against any external docs. When you change a keybinding, update this skill.
