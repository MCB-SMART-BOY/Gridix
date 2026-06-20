# Gridix

Keyboard-first cross-platform database management desktop app.
Rust + eframe/egui 0.34.1. SQLite, PostgreSQL, MySQL.
Tokio async runtime. Helix-inspired modal editing throughout.

**Deps:** russh 0.61, tokio-postgres 0.7.18, rusqlite 0.39, mysql_async 0.36, egui_dock 0.19.
**Toolchain:** rust-toolchain.toml (stable), cargo-audit in CI.
**Binaries:** `gridix` (GUI), `check-doc-links` (link validator), `gridix-driver` (headless driver).
**Code is the source of truth.** When docs and code disagree, code wins. Update `~/.codex/` after code changes (`~/.codex/rules/sync-codex.md`).

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
| Build, launch, screenshot the app | `gridix-run` skill |
| Change a keyboard shortcut | `gridix-keybindings` skill |
| Prepare a PR / run all checks | `gridix-pr-prep` skill |
| Publish a release | `gridix-release` skill |
| Fix a build/startup/test error | `gridix-troubleshoot` skill |
| Understand architecture | this file → module map + architecture sections |
| Change a dialog | "Common change recipes" below + `~/.codex/references/dialog-audit.md` |
| Change the ER diagram | `~/.codex/references/er-contracts.md` — preserve schema-canvas visual language |
| Change database code | `src/data/` + `~/.codex/rules/database.md` |
| Change session/connection code | `src/session/` + `~/.codex/rules/session.md` |
| Change UI code | `src/ui/` + `~/.codex/rules/ui-egui.md` |
| Write/modify tests | `~/.codex/rules/testing.md` |
| Understand invariants | `~/.codex/references/core-flows.md` |
| Check known bugs | `~/.codex/references/bug-ledger.md` |
| Grid edit/save changes | `~/.codex/references/grid-save-isolation.md` |
| Known tech debt / issues | `~/.codex/references/tech-debt.md` |
| Future improvements | `~/.codex/references/roadmap.md` |
| New developer onboarding | `~/.codex/references/onboarding.md` |
| Architecture decisions (ADR) | `~/.codex/references/architecture/decisions.md` |
| Add a DB backend | `~/.codex/references/database-backends.md` |
| Development workflow | `~/.codex/references/workflow.md` |
| Testing patterns | `~/.codex/rules/testing.md` |

## Module map

```
src/
├── main.rs              # → bootstrap::run()
├── lib.rs               # public API re-exports
├── bootstrap.rs         # tracing (RUST_LOG, default gridix=info,warn), panic hook, fonts, eframe launch (1200×800)
├── prelude.rs           # use crate::prelude::* — HashMap, Arc, Color32, tokio, serde, thiserror
├── types.rs             # Layer -1: shared types — DatabaseType, PostgresSslMode, MySqlSslMode, QueryResult
├── core/                # Layer 0: pure functions, no side effects
│   ├── config.rs        # AppConfig (TOML, atomic temp-file+rename, Unix 0o600)
│   ├── keybindings.rs   # Action (38 variants), KeyBindings, keymap.toml engine
│   ├── commands.rs      # ~100 ScopedCommand entries with default_bindings
│   ├── theme.rs         # ThemeManager, 18 ThemePresets, ThemeColors
│   ├── syntax.rs        # SQL highlighting — custom tokenizer
│   ├── autocomplete.rs  # SQL completion — keywords, functions, tables, columns
│   ├── export.rs        # CSV/TSV/SQL/JSON export + import parsing
│   ├── transfer.rs      # Unified TransferSession→Plan→Execution pipeline
│   ├── formatter.rs     # Best-effort SQL beautifier (no AST, keyword-based indent)
│   ├── history.rs       # QueryHistory (100 items max, newest first)
│   ├── notification.rs  # Toast: Info(3s)/Success(3s)/Warning(5s)/Error(8s), max 5 visible
│   ├── progress.rs      # ProgressTask with Arc<AtomicBool> cancel token
│   └── constants.rs     # All magic numbers (pool sizes, timeouts, scale limits, cache sizes)
├── data/                # Layer 1: database operations
│   ├── config.rs        # ConnectionConfig — AES-256-GCM, keyring, password migration
│   ├── connection.rs    # ConnectionManager — HashMap registry, active tracking
│   ├── pool.rs          # Manual pooling: MySQL (TTL+LRU), PG (health-check), SQLite none
│   ├── ssh_tunnel.rs    # russh SSH port forwarding, known_hosts + fingerprint verification
│   ├── error.rs         # DbError (2 variants: Connection, Query)
│   └── query/           # mod.rs orchestrator + sqlite.rs (sync), postgres.rs, mysql.rs (async)
├── session/             # Layer 2: connection lifecycle + tab management + async dispatch
│   ├── mod.rs           # Session struct (~30 fields): runtime, mpsc, manager, tab_manager, request tracking
│   ├── message.rs       # Message enum (13 variants, ALL carry request_id: u64)
│   └── tab.rs           # QueryTab (pure data), QueryTabManager (tabs + active index)
│       (pending: migrate app/runtime/{database,handler,lifecycle,metadata,er_diagram}.rs)
├── state/               # Layer 3: UI state — no DB logic
│   ├── mod.rs           # UiState struct (~60 fields): focus, sidebar, editor, dialogs, ER, grid, theme
│   └── workbench.rs     # WorkbenchState, WorkbenchFocus::Surface, ActivityBar/Sidebar/Panel runtime state, WorkbenchSurfaceKind/Role/Placement/Id descriptors
│       (pending: split into focus/sidebar/editor/dialogs/grid/er_diagram submodules)
├── app/                 # Transitional: DbManagerApp — being decomposed into Session + UiState
│   ├── mod.rs           # DbManagerApp (~11 fields, target reached: ~11)
│   ├── action/          # AppAction (58 variants) → AppEffect, command palette, CommandDescriptor registry
│   ├── dialogs/host.rs  # DialogId (17 variants), active_dialog_owner
│   ├── input/           # Keyboard routing (8-stage dispatch pipeline)
│   │   ├── input_router.rs (~3400 lines)
│   │   ├── owner.rs     # InputOwner: Recording|Modal|TextEntry|Select|Command|Disabled
│   │   └── keyboard.rs  # focus_cycle_areas, zoom shortcuts
│   ├── runtime/         # tokio → mpsc → UI thread (gradually migrating to session/)
│   │   ├── database.rs  # connect, execute (cancel+timeout), disconnect
│   │   ├── handler.rs   # handle_messages() — poll with try_recv, dispatch
│   │   ├── message.rs   # → re-exports session::message::Message
│   │   ├── request_lifecycle.rs  # ID generation, cancel, tab sync
│   │   ├── er_diagram.rs  # ER data loading, relationship inference
│   │   └── metadata.rs  # Sidebar triggers/routines loading
│   ├── surfaces/
│   │   ├── render.rs    # run_frame() main loop + surface/fallback layout orchestration
│   │   ├── workbench.rs # TopBar/ActivityBar/PrimarySidebar/BottomPanel/RightInspector app-bound render adapters + unified surface renderer + navigation surface adapter + surface seed/fallback de-dup helpers
│   │   ├── dialogs.rs   # render_dialogs() + handle_dialog_results()
│   │   └── preferences.rs  # set_ui_scale (0.5–2.0), set_theme, save_config
│   └── workflow/        # export, import, help, welcome
└── ui/                  # Layer 4: egui rendering — widgets, components, styling
    ├── dock_tabs.rs     # egui_dock integration — legacy tabs + DockTab::Surface, canonical April-shell seed (Results center, SQL editor bottom, ER right) and split ratios (query/ER 0.73/0.27, results/editor 0.69/0.31), default_surface_layout(), ensure_surface_tab(), has_surface_tab(), WorkspaceViewer, sync_all()
    ├── workbench/       # WorkbenchShell, dormant ActivityBar widget, BottomPanel, RightInspector, StatusBar, SurfaceHeader widgets
    ├── styles.rs        # SUCCESS/DANGER/GRAY/MUTED from egui Visuals
    ├── shortcut_tooltip.rs  # LocalShortcut (138 variants), config_key() paths
    ├── components/      # grid (10 files), sql_editor, toolbar (4 files),
    │   │                   query_tabs (tab bar rendering), welcome, er_diagram (render),
    │   │                   notifications, progress_indicator
    │   ├── dialogs/     # connection, export, import, help, ddl, keybindings,
    │   │                   about, create_db, create_user, picker_shell, toolbar_menu, toolbar_theme
    │   └── panels/      # sidebar (8 files, ~4300 lines), history_panel
    └── surfaces/        # render.rs, dialogs.rs, preferences.rs (thin wrappers delegating to app/)
```

## Architecture

**6-layer unidirectional dependency (final):**
```
src/types.rs    (Layer -1) — shared types
     ↑
src/core/       (Layer 0)  — pure functions, no side effects
     ↑
src/data/       (Layer 1)  — database operations, match db_type dispatch
     ↑
src/session/    (Layer 2)  — connection lifecycle, async dispatch (~30 fields)
     ↑
src/state/      (Layer 3)  — UI rendering state (~60 fields)
     ↑
src/app/ + ui/  (Layer 4)  — eframe App impl, rendering, input routing (DbManagerApp: ~11 fields)
```

**Refactoring complete (v6.3.0):**
- ✅ DbManagerApp: ~100 → ~11 fields (~89 migrated to Session/UiState)
- ✅ Session: ~30 fields with request ID privacy, needs_repaint decoupling
- ✅ UiState: ~60 fields with all UI rendering state
- ✅ self.sql dual source eliminated
- ✅ database/ → data/ renamed
- ✅ Config versioning, throttling, security fixes
- ✅ SQLite driver tests, AppError types, 3 audit fixes
- ✅ 0 clippy errors, 0 compiler warnings, 0 test failures
- ✅ 6 critical logic paths verified (needs_repaint, mirror sync, config debounce, handler guards, tab switch, connection guards)

## Architecture of `~/.codex/`

| directory | when loaded | purpose |
|---|---|---|
| `CLAUDE.md` | every session | project-wide context, module map, architecture |
| `workflow/` | agent follows stage by stage | 8-stage lifecycle: Intake→Discovery→Design→Safety Net→Implement→Review→Verify→Deliver |
| `skills/` | user invokes skill name or `description` matches task | executable workflows: gridix-run, gridix-keybindings, gridix-pr-prep, gridix-release, gridix-troubleshoot |
| `rules/` | automatically when editing files matching `paths:` | domain rules with DO/DON'T/VERIFY patterns: database, session, ui-egui, testing, sync-codex |
| `templates/` | agent uses for consistency | standard formats: commit messages, PR descriptions, feature requests |
| `references/` | agent reads on demand | engineering ledgers (tech-debt, roadmap, bug-ledger), design contracts (core-flows, er-contracts, dialog-audit), architecture ADRs |
| `memory/` | persistent between sessions | project context, user preferences, feedback log |

## Available skills

`gridix-run` — build, launch, screenshot · `gridix-keybindings` — keyboard shortcuts · `gridix-pr-prep` — pre-PR checks · `gridix-release` — version bump → publish · `gridix-troubleshoot` — build/launch/test fixes
