# Technical debt & logic issues

Code-verified issues found during audit. Ordered by severity.

## Architectural debt

### ~~DatabaseDriver trait is dead code~~ FIXED
Deleted `database/driver.rs` — trait had 0 implementors. Actual dispatch: `match db_type` in `query/mod.rs`.

### ~~state/mod.rs is dead code~~ FIXED
Deleted `app/state/mod.rs` — 8 unused structs, all fields already inlined in `DbManagerApp`.

### Dual-source self.sql (medium)
Both `DbManagerApp::sql` and `active_tab().sql` exist — can diverge. Known structural cost from recovery docs. Fix: make tab the sole authority, remove the app-level mirror.

### ~~No shared test utilities~~ FIXED
`tests/common/mod.rs` provides `begin_key_pass()` and `focus_text_input()`. Duplicate test files removed (ddl_dialog_tests.rs, autocomplete_tests.rs, formatter_tests.rs, syntax_tests.rs).

## Logic bugs & limitations

### ~~SQL formatter: byte-index panic~~ FIXED
`core/formatter.rs` — `i` tracked char position but was used as byte index into `&upper[i..]`, causing panic on non-ASCII identifiers. Fixed by using `chars` Vec comparison.

### ~~Autocomplete: false positives~~ FIXED
### ~~MySQL get_columns: backslash injection~~ FIXED
### ~~grid actions: "null" → NULL data loss~~ FIXED
### ~~keybindings: mac_cmd ignored in matches()~~ FIXED
### ~~disconnect: pending_triggers_request cleared for wrong connection~~ FIXED
### ~~grid save SQL: missing identifier quoting~~ FIXED
`generate_save_sql` used `escape_identifier` (validation only) without `quote_identifier` (quoting). Fixed by using `quote_identifier` with database-aware quoting style. Added `db_type` parameter.
### ~~query/mod.rs: dollar-quote in skip_balanced_parens~~ FIXED
`skip_balanced_parens` didn't handle PostgreSQL dollar-quoted strings, causing wrong bracket depth. Fixed with dollar-quote tag detection.
### SQL formatter: nested subqueries not handled (low)
`core/formatter.rs` — keyword-based indent, no AST. Nested `SELECT` within parentheses not re-indented. Known limitation.
### SSH: confusing error on known_hosts mismatch (low)
### ~~MySQL pool race~~ FIXED
Health check + removal now atomic under write lock.
### ~~PG pool race~~ FIXED
Same pattern — health check and removal atomic under write lock.
### ~~SSH tunnel race~~ FIXED
`get_or_create` re-checks existence under write lock, discards duplicate tunnel.
### ~~Grid keyboard: 0 key with prefix~~ FIXED
Pressing `0` while prefix is active now clears prefix and correctly triggers JumpLineStart.

## CI gaps

### No PostgreSQL integration tests in CI
Only MySQL has scheduled CI testing. PostgreSQL tests would need a service container like MySQL's.

### No test coverage measurement
No `cargo-tarpaulin`, `grcov`, or Codecov. Unknown what % of code is exercised by tests.

### Release workflow skips quality gate
`release.yml` builds artifacts without running `cargo fmt`/`clippy`/`test`. A release could go out with broken tests if the branch wasn't pushed to master first.

### ~~Unpinned Rust toolchain~~ FIXED
`rust-toolchain.toml` created. CI pinned to stable channel.

### ~~Manual layout → egui_dock~~ MIGRATED
`central_panel_ratio` field removed. DockArea manages split ratios.
### ~~Dock: tab index drift after close~~ FIXED
Closing a dock tab left remaining tabs with stale indices. Fixed: reindex all QueryData tabs + remove out-of-range tabs in `sync_query_tabs()`.
### ~~Dock: new tab pushed to wrong leaf~~ FIXED
`push_to_focused_leaf` could push new tabs to ER/SQL editor leaf. Fixed: find QueryData leaf explicitly before pushing.
### ~~Dock: welcome page not rendered~~ FIXED
`render_workspace_content` had no Welcome surface branch. Fixed: added Welcome::show() + handle_welcome_action() inline.
### ~~SqlEditor actions discarded in dock tab~~ FIXED
`render_sql_editor_in_ui` returns `SqlEditorActions` but dock viewer discarded it. Fixed: capture + `handle_sql_editor_actions()`.
### ~~remove_tabs index instability~~ FIXED
Multi-leaf removal collected pre-removal indices. Fixed: sort descending before removal.
### ~~Dead clones in run_frame~~ FIXED
5 unused connection-data clones removed from CentralPanel. Cleanup: dead welcome_action/sql_editor_actions vars and their handlers removed.
### ~~Double toolbar_actions handling~~ DOCUMENTED
Two sources (input router + toolbar widget) use separate ToolbarActions instances — no double-processing. By design.
### ~~Session: central_panel_ratio persists but unused~~ FIXED
Removed `central_panel_ratio` field, `default_central_panel_ratio()`, and `record_layout()` from `session.rs`.
