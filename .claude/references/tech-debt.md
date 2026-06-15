# Technical debt & logic issues

Code-verified issues found during audit. Ordered by severity.

## Resolved

### ~~DatabaseDriver trait is dead code~~ FIXED
Deleted `database/driver.rs` — trait had 0 implementors. Actual dispatch: `match db_type` in `data/query/mod.rs`.

### ~~state/mod.rs is dead code~~ FIXED
Deleted `app/state/mod.rs` — 8 unused structs, all fields already inlined in `DbManagerApp`.

### ~~No shared test utilities~~ FIXED
`tests/common/mod.rs` provides `begin_key_pass()` and `focus_text_input()`. Duplicate test files removed.

### ~~SQL formatter: byte-index panic~~ FIXED
### ~~Autocomplete: false positives~~ FIXED
### ~~MySQL get_columns: backslash injection~~ FIXED
### ~~grid actions: "null" → NULL data loss~~ FIXED
### ~~keybindings: mac_cmd ignored in matches()~~ FIXED
### ~~disconnect: pending_triggers_request cleared for wrong connection~~ FIXED
### ~~grid save SQL: missing identifier quoting~~ FIXED
### ~~Dollar-quote in skip_balanced_parens~~ FIXED
### ~~MySQL pool race~~ FIXED
### ~~PG pool race~~ FIXED
### ~~SSH tunnel race~~ FIXED
### ~~Grid keyboard: 0 key with prefix~~ FIXED
### ~~Manual layout → egui_dock~~ MIGRATED
### ~~Dock: tab index drift after close~~ FIXED
### ~~Dock: new tab pushed to wrong leaf~~ FIXED
### ~~Dock: welcome page not rendered~~ FIXED
### ~~SqlEditor actions discarded in dock tab~~ FIXED
### ~~remove_tabs index instability~~ FIXED
### ~~Dead clones in run_frame~~ FIXED
### ~~Double toolbar_actions handling~~ DOCUMENTED
### ~~Session: central_panel_ratio persists but unused~~ FIXED
### ~~Grid-save pipeline dead code~~ FIXED (deleted ~150 lines)
### ~~11 clippy errors~~ FIXED (0 remaining)

## Active

### Self.sql dual-source (medium)
`DbManagerApp::sql` and `QueryTab::sql` exist independently — can diverge. 13 code paths set `self.sql` directly. Fix: eliminate `self.sql`, route all access through `session.active_sql()` (Phase D of architecture refactoring).

### SQL formatter: nested subqueries not handled (low)
`core/formatter.rs` — keyword-based indent, no AST. Nested `SELECT` within parentheses not re-indented. Known limitation.

### SSH: confusing error on known_hosts mismatch (low)
Current UX requires manual `ssh` connection before Gridix can connect. Custom TOFU dialog would improve UX. Severity lowered from previous rating after adding SHA-256 fingerprint to error messages.

### Ci gaps (medium)
- No PostgreSQL integration tests in CI (test binary exists but no Docker service)
- No test coverage measurement (tarpaulin workflow created, needs CODECOV_TOKEN)

### Architecture debt (medium)
- `DbManagerApp` ~100 fields — plan: 4-field struct after architecture refactoring
- `database/` → `data/` rename pending (Phase A)
- `session/` and `state/` layers not yet created (Phase B-C)
