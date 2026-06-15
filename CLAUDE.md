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
cargo test                     # all pass
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
| Change database code | `src/data/` + `.claude/rules/database.md` |
| Change session/connection code | `src/session/` + `.claude/rules/session.md` |
| Change UI code | `src/ui/` + `.claude/rules/ui-egui.md` |
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
├── types.rs             # Layer -1: shared data types (ConnectionConfig, DbError, DatabaseType, QueryResult, SshError)
├── core/                # Layer 0: pure functions, no side effects, no egui
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
│   ├── notification.rs  # Toast: Info(3s)/Success(3s)/Warning(5s)/Error(8s), max 5 visible
│   ├── progress.rs      # ProgressTask with Arc<AtomicBool> cancel token
│   └── constants.rs     # All magic numbers (pool sizes, timeouts, scale limits, cache sizes)
├── data/                # Layer 1: database operations (was database/)
│   ├── config.rs        # Low-level connection config (encryption, keyring, password migration)
│   ├── connection.rs    # ConnectionManager — HashMap registry, active tracking
│   ├── pool.rs          # Manual pooling: MySQL pools (idle TTL + LRU eviction), PG clients (health-check), SQLite none
│   ├── ssh_tunnel.rs    # russh-based SSH port forwarding, known_hosts verification, tunnel reuse by name
│   ├── error.rs         # DbError (thiserror, 2 active variants)
│   └── query/           # mod.rs orchestrator + sqlite.rs (sync), postgres.rs, mysql.rs (async)
├── session/             # Layer 2: connection lifecycle + tab management + async dispatch (NEW)
│   ├── mod.rs           # Session struct — runtime, mpsc channel, request ID tracking
│   ├── database.rs      # connect, execute (with cancel+timeout), disconnect
│   ├── handler.rs       # poll_messages() → FrameEffects, dispatch, stale-request guard
│   ├── message.rs       # Message enum (13 variants, ALL carry request_id: u64)
│   ├── lifecycle.rs     # ID generation, cancel via oneshot, task tracking
│   ├── tab.rs           # QueryTab (pure data), QueryTabManager (tabs + active index)
│   ├── er_diagram.rs    # ER data loading, relationship inference (heuristic: _id suffix matching)
│   └── metadata.rs      # Sidebar triggers/routines loading
├── state/               # Layer 3: UI state — all rendering state, no DB logic (NEW)
│   ├── mod.rs           # UiState struct
│   ├── focus.rs         # FocusArea, focus_cycle
│   ├── sidebar.rs       # SidebarPanelState, sidebar visibility/width
│   ├── editor.rs        # Editor visibility, autocomplete state
│   ├── dialogs.rs       # DialogId (17 variants), active_dialog_owner
│   ├── grid.rs          # DataGridState, GridWorkspaceStore
│   ├── er_diagram.rs    # ERDiagramState (visual state — table positions, selection, viewport)
│   └── theme.rs         # ThemeManager, scale, highlight_colors
└── ui/                  # Layer 4: rendering — eframe App impl, widgets, components
    ├── mod.rs           # DbManagerApp (4 fields: session, state, config, keybindings)
    ├── dock_tabs.rs     # egui_dock integration — DockTab enum, WorkspaceViewer, sync_all()
    ├── styles.rs        # SUCCESS/DANGER/GRAY/MUTED helpers from egui Visuals
    ├── shortcut_tooltip.rs  # LocalShortcut (141 variants), config_key() paths, runtime overrides
    ├── surfaces/
    │   ├── render.rs    # run_frame() main loop: poll → apply effects → render
    │   ├── dialogs.rs   # render_dialogs() + handle_dialog_results()
    │   └── preferences.rs  # set_ui_scale (0.5–2.0 clamp), set_theme, save_config
    ├── input/           # Keyboard routing (8-stage pipeline)
    │   ├── input_router.rs  # resolve_input_action_with() — 8-stage dispatch
    │   ├── owner.rs     # InputOwner: Recording|Modal|TextEntry|Select|Command|Disabled
    │   └── keyboard.rs  # focus_cycle_areas, zoom shortcuts
    └── components/      # grid (10 files), sql_editor, toolbar (4 files),
        │                   query_tab_bar, welcome, er_diagram (render part), notifications, progress_indicator
        ├── dialogs/     # connection (~1243 lines), export, import (3 files), help (4 files),
        │                   ddl, keybindings (~3560 lines), about, create_db, create_user, picker_shell
        └── panels/      # sidebar (8 files, ~4300 lines), history_panel
```

## Architecture

**4-layer unidirectional dependency:**
```
src/types.rs  (Layer -1) ← shared by all layers
     ↑
src/core/     (Layer 0)  ← pure functions, no side effects
     ↑
src/data/     (Layer 1)  ← database operations, match db_type dispatch
     ↑
src/session/  (Layer 2)  ← connection lifecycle, query lifecycle, tab mgmt, mpsc, async
     ↑
src/state/    (Layer 3)  ← UI state structs, FrameEffects application
     ↑
src/ui/       (Layer 4)  ← eframe App impl, rendering, input routing
```

Each layer depends only on layers below it. Layers above never import from layers that are higher.

**Key architectural decisions:**
- **No trait for DB backends.** `match db_type` is the correct pattern for three backends with fundamentally different execution models (SQLite sync/spawn_blocking, PG async, MySQL async pooled). This was verified against a previous attempt (`DatabaseDriver` trait, deleted as dead code).
- **Session returns FrameEffects, not direct UI mutations.** `poll_messages()` processes async results internally (tab state, history, autocomplete) and emits structured effects for the UI layer to apply.
- **One truth source per data.** `QueryTab.sql` is the sole authority; `self.sql` mirror eliminated.
- **No process separation.** The serialization tax on QueryResult is too high; SSH tunnel management across processes adds unnecessary complexity.

**Runtime:** UI thread (egui frames) + Tokio multi-thread (async DB). Communication via `std::sync::mpsc::channel`.

**Data flow (per frame):**
```
1. session.poll_messages() → FrameEffects
2. state.apply_frame_effects(effects)
3. ui.render(ctx) — reads session (immutable) + state (mutable for UI state)
```

**Layout (egui_dock):** The main workspace uses `DockArea` with resizable panels. `DockTab` variants: `QueryData` (data grid), `SqlEditor`, `ErDiagram`, `AuxPanel`. `sync_all()` runs each frame to synchronize dock tabs with session tab state.

**Keyboard routing (8-stage pipeline, `input_router.rs`):**
1. True-global fallback (zoom: Ctrl+=/Ctrl+-/Ctrl+0) → 2. Recording mode → 3. Dialog shortcuts → 4. ER diagram → 5. Modal dialog stops here → 6. Scoped keymap dispatch → 7. Workspace fallback → 8. Minimal global (F1/Ctrl+N/Ctrl+P)

**Focus cycle:** `Sidebar → DataGrid → ErDiagram → SqlEditor` via Tab/Shift+Tab.

## Core invariants

- At most one dialog owns keyboard input per frame (`active_dialog_owner` reconciled at frame start)
- Text entry always wins over command keys (`TextEntryGuard`)
- Every async message carries `request_id: u64` — stale responses dropped
- Grid workspace isolated per `(tab_id, connection, database, table)` via `GridWorkspaceStore`
- `Session` owns all async infrastructure; `UiState` is never touched by async code
- `poll_messages()` returns `FrameEffects`; handlers never directly mutate State

## Config & persistence

**Files (all platforms via `dirs` crate):** `~/.config/gridix/config.toml` (AppConfig), `~/.config/gridix/keymap.toml` (keybindings). All atomic temp-file+rename, Unix `0o600`.

**Config ownership:** `SessionConfig` (connections, history) lives in Session. `UiConfig` (theme, scale, onboarding) lives in State. `AppConfig` is the persistence format, composed from both on save.

**Password security:** `password_ref` UUID in config.toml → actual secret in OS keyring (`keyring` crate). Legacy AES-256-GCM encrypted passwords auto-migrated. SSL/TLS: PG default Prefer, MySQL default Preferred. Both Required modes validate certificates. SSH: `russh` + `known_hosts` + key fingerprint logging. SSH passwords `#[serde(skip_serializing)]`.

## Test infrastructure

**Locations:** `tests/*.rs` + `src/**/mod.rs` `#[cfg(test)]`. Shared utilities in `tests/common/mod.rs`.

**Patterns:** pure logic (`#[test]`), egui component (`Context::default()` + `begin_pass`), async (`#[tokio::test]`). MySQL integration `#[ignore]`d — needs `GRIDIX_IT_MYSQL_*` env vars. Session can be tested without egui Context.

## Code conventions

- `use crate::prelude::*;` for common types · `thiserror` for errors · `// =====...=====` section separators
- Chinese `//!` module docs + `///` field docs, English identifiers
- Commit: `type(scope): description` — `fix:`, `feat:`, `docs:`, `refactor:`, `release:`

## Common change recipes

**Add a dialog:** DialogId in `state/dialogs.rs` → file in `ui/components/dialogs/` → render in `ui/surfaces/dialogs.rs` → result in `handle_dialog_results()` → LocalShortcut in `shortcut_tooltip.rs` → scoped commands in `commands.rs`.

**Add a database backend:** Add `src/data/query/<backend>.rs` → register in `src/data/query/mod.rs` dispatch → no other files need changes.

**Add a session operation:** Add method on `Session` in `src/session/` → if produces new effect, add variant to `FrameEffects` → handle in `state/mod.rs`.

**Change a keybinding:** see `/keybindings` skill.

**Change database code:** see `.claude/rules/database.md` — cancel flow, pooling, null handling.

**Change session code:** see `.claude/rules/session.md` — Message variants, request lifecycle, tab management.

## Environment variables

| var | default | where |
|---|---|---|
| `RUST_LOG` | `gridix=info,warn` | bootstrap.rs |
| `GRIDIX_IT_MYSQL_HOST/PORT/USER/PASSWORD/DB` | (none) | MySQL integration tests |
| `WINIT_UNIX_BACKEND` | (auto) | Set `x11` for xdotool on Wayland |
| `DISPLAY` | (system) | Driver uses `:99` with Xvfb |

## Docs reliability

`.claude/rules/` and `.claude/references/` are the authoritative design documents. Code is the source of truth — when docs and code disagree, code wins.

## Architecture of `.claude/`

| directory | when loaded | contains |
|---|---|---|
| `CLAUDE.md` (this file) | every session | project-wide context |
| `skills/` | user invokes `/<name>` or `description` matches task | executable workflows |
| `rules/` | automatically when editing files matching the `paths:` glob | domain-specific rules |
| `references/` | agent reads on demand (linked from task nav + recipes) | engineering ledgers, invariants, design contracts |

## Available skills

`/run-gridix` — build, launch, drive · `/keybindings` — keyboard shortcuts · `/pr-prep` — pre-PR checks · `/release` — version bump → publish · `/troubleshoot` — build/launch/test fixes
