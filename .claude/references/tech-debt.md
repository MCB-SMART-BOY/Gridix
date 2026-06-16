# Technical debt & design gaps

Current state as of architecture refactoring (v6.2.0).

## Resolved

- [x] DatabaseDriver trait (dead code) — deleted
- [x] app/state/mod.rs (dead code) — deleted
- [x] self.sql dual source — eliminated, single source = tab_manager
- [x] central_panel_ratio in session.rs — removed
- [x] grid-save pipeline (dead code) — deleted
- [x] 11 clippy errors — fixed
- [x] No shared test utilities — tests/common/mod.rs created
- [x] Duplicate test files — deleted (4 files)
- [x] syntect dead dependency — removed
- [x] once_cell → LazyLock
- [x] lazy_static → LazyLock
- [x] parking_lot::Mutex → std::sync::Mutex
- [x] SSL cert validation — Required modes now validate
- [x] SSH password in config — skip_serializing
- [x] Public API exposure — pub(crate) mod app
- [x] DbManagerApp ~100 flat fields — reduced to ~47 (session + state extracted)

## Active design debt

### Session encapsulation (HIGH)
**`src/session/mod.rs`** — All fields are `pub`. Any code can:
- `session.manager.connections.clear()`
- `session.tx.send(arbitrary_message)`
- `session.next_connect_request_id = 0` (break ID sequencing)

Fix: Make fields `pub(crate)`, expose operations through methods.

### UiState unused duplicates (HIGH)
**UiState** has 25 fields but DbManagerApp retains identical copies. Example:
- `self.show_sidebar` AND `self.state.show_sidebar` — two copies, can diverge
- Same for 20+ other fields

Fix: Complete migration or delete UiState. Half-finished is worse than not started.

### FrameEffects not wired (HIGH)
`session/frame_effects.rs` defines 100 lines of effect types but `handle_messages()` still directly mutates all state. The architectural promise of "Session → FrameEffects → State" is unfulfilled.

### Result\<..., String\> pattern (MEDIUM)
25 files use `Result<_, String>` for errors (import/export/transfer). No error codes, no structured matching, no location context.

### Config save unbounded (MEDIUM)
`save_config()` called from 15 places (connect, disconnect, execute, preferences, onboarding). No throttling — rapid query execution triggers repeated disk writes.

### Legacy password migration (LOW)
`src/data/config.rs` ~lines 148-165: AES-256-GCM decryption for v4 passwords. No deprecation window defined.

### Legacy keybindings field (LOW)
`AppConfig.keybindings: KeyBindings` — read-only since v4, still written to config.toml. Has TODO comment with no deadline.

### Oversized files (LOW)
- `keybindings_dialog.rs` (3560 lines)
- `input_router.rs` (3369 lines) 
- `keybindings.rs` (2448 lines)

### Config no version field (LOW)
`config.toml` has no `version` key — future format migration has no path. Relies on `serde(default)`.

### QueryResult no streaming (LOW)
100K+ row results stored entirely in memory as `Vec<Vec<String>>`. No cursor/pagination.
