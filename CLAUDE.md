# Gridix

Keyboard-first cross-platform database management desktop app.
Rust + eframe/egui 0.34.1. SQLite, PostgreSQL, MySQL.
Tokio async runtime. Helix-inspired modal editing throughout.

**Deps:** russh 0.61, tokio-postgres 0.7.18, rusqlite 0.39, mysql_async 0.36, egui_dock 0.19.
**Toolchain:** rust-toolchain.toml (stable), cargo-audit in CI.
**Binaries:** `gridix` (GUI), `check-doc-links` (link validator), `gridix-driver` (headless driver).
**Code is the source of truth.** When docs and code disagree, code wins. Update `.claude/` after code changes (`.claude/rules/sync-claude.md`).

## Quick commands

```bash
cargo build --release          # ~90s → target/release/gridix
cargo build                    # ~30s debug
cargo test                     # ~620 tests, all pass
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test && cargo run --bin check-doc-links  # full pre-PR
```

## Task navigation

| you want to… | start here |
|---|---|
| Build, launch, screenshot the app | `/run-gridix` skill |
| Change a keyboard shortcut | `/keybindings` skill |
| Prepare a PR / run all checks | `/pr-prep` skill |
| Publish a release | `/release` skill |
| Fix a build/startup/test error | `/troubleshoot` skill |
| Understand architecture | this file → module map + architecture sections |
| Change a dialog | "Common change recipes" below + `.claude/references/dialog-audit.md` |
| Change the ER diagram | `.claude/references/er-contracts.md` |
| Change database code | `.claude/references/query-execution.md` + `.claude/rules/database.md` |
| Change UI code | `.claude/rules/ui-egui.md` |
| Write/modify tests | `.claude/rules/testing.md` |
| Understand invariants | `.claude/references/core-flows.md` |
| Check known bugs | `.claude/references/bug-ledger.md` |
| Grid edit/save changes | `.claude/references/grid-save-isolation.md` |
| Known tech debt / issues | `.claude/references/tech-debt.md` |
| Future improvements | `.claude/references/roadmap.md` |

## Module map

```
src/
├── main.rs              # → bootstrap::run()
├── lib.rs               # public API re-exports
├── bootstrap.rs         # tracing (RUST_LOG, default gridix=info,warn), panic hook, fonts, eframe launch (1200×800)
├── prelude.rs           # use crate::prelude::* — HashMap, Arc, Color32, tokio, serde, thiserror
├── app/
│   ├── mod.rs           # DbManagerApp (838 lines) — central eframe::App, GridWorkspaceStore
│   ├── action/          # AppAction (44 variants) → AppEffect, command palette, CommandDescriptor registry
│   ├── dialogs/host.rs  # DialogId (17 variants), active_dialog_owner — at most ONE dialog owns input per frame
│   ├── input/           # Keyboard routing
│   │   ├── input_router.rs (3370 lines)  # resolve_input_action_with() — 8-stage dispatch pipeline
│   │   ├── owner.rs     # InputOwner: Recording|Modal|TextEntry|Select|Command|Disabled
│   │   └── keyboard.rs  # focus_cycle_areas (Sidebar→DataGrid→ErDiagram→SqlEditor), zoom shortcuts
│   ├── runtime/         # tokio → mpsc channel → UI thread
│   │   ├── database.rs  # connect, execute (with cancel+timeout), disconnect, grid save
│   │   ├── handler.rs   # handle_messages() — poll with try_recv, dispatch, stale-request guard
│   │   ├── message.rs   # Message enum (16 variants, ALL carry request_id: u64)
│   │   ├── request_lifecycle.rs  # ID generation, cancel via oneshot, task tracking
│   │   ├── er_diagram.rs  # ER data loading, relationship inference (heuristic: _id suffix matching)
│   │   └── metadata.rs  # Sidebar triggers/routines loading
│   ├── surfaces/
│   │   ├── render.rs (1831 lines)  # run_frame() main loop: reconcile owner → messages → input → dialogs → panels
│   │   ├── dialogs.rs   # render_dialogs() + handle_dialog_results()
│   │   └── preferences.rs  # set_ui_scale (0.5–2.0 clamp), set_theme, save_config
│   └── workflow/        # export, import (rfd::FileDialog), help (learning sample DB, 8 tables/100+ rows), welcome
├── core/                # framework-agnostic
│   ├── config.rs        # AppConfig (TOML, atomic temp-file+rename, Unix 0o600)
│   ├── keybindings.rs   # Action (35 variants), KeyBindings, keymap.toml engine, scope_resolution_chain()
│   ├── commands.rs      # ~100 ScopedCommand entries with default_bindings
│   ├── theme.rs         # ThemeManager, 18 ThemePresets (default: TokyoNightStorm dark, TokyoNightLight light)
│   ├── syntax.rs        # SQL highlighting — custom tokenizer (110 keywords, 85 functions)
│   ├── autocomplete.rs  # SQL completion — keywords, functions, tables, columns, WHERE-context aware
│   ├── export.rs        # CSV/TSV/SQL/JSON export + import parsing (csv crate, manual JSON parser)
│   ├── transfer.rs      # Unified TransferSession→Plan→Execution pipeline (wraps export.rs)
│   ├── formatter.rs     # Best-effort SQL beautifier (no AST, keyword-based indent)
│   ├── history.rs       # QueryHistory (100 items max, newest first)
│   ├── session.rs       # SessionManager — tabs, layout, auto-save (60s tick + Drop)
│   ├── notification.rs  # Toast: Info(3s)/Success(3s)/Warning(5s)/Error(8s), max 5 visible
│   ├── progress.rs      # ProgressTask with Arc<AtomicBool> cancel token
│   └── constants.rs     # All magic numbers (pool sizes, timeouts, scale limits, cache sizes)
├── database/
│   ├── config.rs        # ConnectionConfig — AES-256-GCM encrypted passwords, keyring via password_ref UUID
│   ├── connection.rs    # ConnectionManager — HashMap registry, active tracking
│   ├── pool.rs          # Manual pooling: MySQL pools (idle TTL + LRU eviction), PG clients (health-check), SQLite none
│   ├── ssh_tunnel.rs    # russh-based SSH port forwarding, known_hosts verification, tunnel reuse by name
│   ├── error.rs         # DbError (thiserror, 5 variants, SQL truncated at 200 chars in context)
│   ├── types.rs         # QueryResult with null_flags: Vec<Vec<bool>> — distinguishes SQL NULL from ""
│   └── query/           # mod.rs orchestrator + sqlite.rs (sync), postgres.rs, mysql.rs (async)
└── ui/
    ├── dock_tabs.rs     # egui_dock integration — DockTab enum, WorkspaceViewer, sync_all()
    ├── styles.rs        # SUCCESS/DANGER/GRAY/MUTED helpers from egui Visuals
    ├── shortcut_tooltip.rs  # LocalShortcut (141 variants), config_key() paths, runtime overrides
    └── components/      # grid (10 files, 1949-line keyboard.rs), sql_editor, toolbar (4 files),
        │                   query_tabs, welcome, er_diagram (4 files), notifications, progress_indicator
        ├── dialogs/     # connection (~1243 lines), export, import (3 files), help (4 files, ~3076 lines),
        │                   ddl, keybindings (~3560 lines), about, create_db, create_user, picker_shell, toolbar_menu, toolbar_theme
        └── panels/      # sidebar (8 files, ~4300 lines), history_panel
```

## Architecture

**Runtime:** UI thread (egui frames) + Tokio multi-thread (async DB). Communication via `std::sync::mpsc`.

**Layout (egui_dock):** The main workspace uses `DockArea` with resizable panels. `DockTab` variants: `QueryData` (data grid), `SqlEditor`, `ErDiagram`, `AuxPanel`. `sync_all()` runs each frame to synchronize dock tabs with app state. Layout ratios are managed by egui_dock (replaces old manual `allocate_ui_with_layout`). Sidebar, toolbar, and dialogs are outside the dock.

**Keyboard routing (8-stage pipeline, `input_router.rs`):**
1. True-global fallback (zoom: Ctrl+=/Ctrl+-/Ctrl+0) → 2. Recording mode → 3. Dialog shortcuts → 4. ER diagram → 5. Modal dialog stops here → 6. Scoped keymap dispatch → 7. Workspace fallback → 8. Minimal global (F1/Ctrl+N/Ctrl+P)

**Focus cycle:** `Sidebar → DataGrid → ErDiagram → SqlEditor` via Tab/Shift+Tab.

**Scope inheritance:** child scopes inherit from parents — `dialog.help.scroll_up → dialog.help → dialog.common → workspace routing`. Defined in `keybindings.rs::scope_resolution_chain()`.

**Core invariants:**
- At most one dialog owns keyboard input per frame (`active_dialog_owner` reconciled at frame start)
- Text entry always wins over command keys (`TextEntryGuard`)
- Every async message carries `request_id: u64` — stale responses dropped
- Grid workspace isolated per `(tab_id, connection, database, table)` via `GridWorkspaceStore`
- See `.claude/references/core-flows.md` for the full 7 invariants + 9 core flows

## Config & persistence

**Files (all platforms via `dirs` crate):** `~/.config/gridix/config.toml` (AppConfig), `~/.config/gridix/keymap.toml` (keybindings), `~/.config/gridix/session.toml` (session). All atomic temp-file+rename, Unix `0o600`.

**Password security:** `password_ref` UUID in config.toml → actual secret in OS keyring (`keyring` crate). Legacy AES-256-GCM encrypted passwords auto-migrated. SSL/TLS: PG (Disable/Prefer/Require/VerifyCa/VerifyFull), MySQL (Disabled/Preferred/Required/VerifyCa/VerifyIdentity). SSH: `russh` + `known_hosts`.

## Test infrastructure

**Locations:** `tests/*.rs` (13 files, 108 external tests) + `src/**/mod.rs` `#[cfg(test)]` (56 files, 512 inline tests).

**Patterns:** pure logic (`#[test]`), egui component (`Context::default()` + `begin_pass`), async (`#[tokio::test]`). MySQL integration `#[ignore]`d — needs `GRIDIX_IT_MYSQL_*` env vars. See `.claude/rules/testing.md` for full patterns.

## Code conventions

- `use crate::prelude::*;` for common types · `thiserror` for errors · `// =====...=====` section separators
- Chinese `//!` module docs + `///` field docs, English identifiers
- `#[allow(dead_code)]` on public API with Chinese justification
- Commit: `type(scope): description` — `fix(sql-editor):`, `feat(welcome):`, `docs:`, `refactor:`, `release:`
- Docs: bilingual EN+中文 same page · behavior change → update docs same PR · user-visible → `docs/CHANGELOG.md`

## Common change recipes

**Add a dialog:** DialogId in `dialogs/host.rs` → file in `ui/dialogs/` → render in `surfaces/dialogs.rs` → result in `handle_dialog_results()` → LocalShortcut in `shortcut_tooltip.rs` → scoped commands in `commands.rs`. See `.claude/references/dialog-audit.md` for shell contracts.

**Add a theme:** ThemePreset + ThemeColors + name/all arms in `core/theme.rs` → toolbar entry in `toolbar_theme_dialog.rs`.

**Add a toolbar action:** AppAction + CommandDescriptor + availability + reduction in `action_system.rs` → button in `ui/components/toolbar/`.

**Change a keybinding:** see `/keybindings` skill. Verify with `/run-gridix` driver.

**Change ER diagram:** see `.claude/references/er-contracts.md` — keyboard flow, token map, readability standards.

**Change database code:** see `.claude/rules/database.md` — cancel flow, pooling, null handling, password security.

## Environment variables

| var | default | where |
|---|---|---|
| `RUST_LOG` | `gridix=info,warn` | bootstrap.rs |
| `GRIDIX_IT_MYSQL_HOST/PORT/USER/PASSWORD/DB` | (none) | MySQL integration tests |
| `WINIT_UNIX_BACKEND` | (auto) | Set `x11` for xdotool on Wayland |
| `DISPLAY` | (system) | Driver uses `:99` with Xvfb |

## Docs reliability

Standalone docs (ARCHITECTURE.md, etc.) were consolidated into CLAUDE.md and `.claude/` during v5.0.0 docs consolidation. `.claude/rules/` and `.claude/references/` are the authoritative design documents. CHANGELOG.md is in `docs/CHANGELOG.md`. Code is the source of truth — when docs and code disagree, code wins.

## Architecture of `.claude/`

| directory | when loaded | contains |
|---|---|---|
| `CLAUDE.md` (this file) | every session | project-wide context |
| `skills/` | user invokes `/<name>` or `description` matches task | executable workflows |
| `rules/` | automatically when editing files matching the `paths:` glob | domain-specific rules |
| `references/` | agent reads on demand (linked from task nav + recipes) | engineering ledgers, invariants, design contracts |

## Available skills

`/run-gridix` — build, launch, drive · `/keybindings` — keyboard shortcuts · `/pr-prep` — pre-PR checks · `/release` — version bump → publish · `/troubleshoot` — build/launch/test fixes
