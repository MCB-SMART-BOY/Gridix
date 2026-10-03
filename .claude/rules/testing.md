---
paths:
  - tests/**/*.rs
  - src/**/mod.rs
---

# Testing rules with Gridix overlay

Use `~/.codex/references/modern-software-engineering-workflow.md` for the cross-project testing strategy and `~/.codex/references/rust-modern-engineering-playbook.md` for Rust-specific gates.

**Code is the source of truth.** Verify patterns against existing tests in `tests/` and `src/`. Update this file when test infrastructure changes.

## Universal policy

- Test the risk, not the implementation detail.
- Put pure logic tests at the lowest layer that can express the behavior.
- Add regression tests for fixed bugs when practical.
- Use characterization tests before high-risk refactors.
- Use measurements, not intuition, for optimization claims.
- Keep tests deterministic and independent of external services by default.

## Rust gates

Fast local loop:

```bash
cargo check
cargo test -p <package> <test_name>
```

Pre-merge / CI-quality validation:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo run --bin check-doc-links
cargo run --bin check-doc-symbols
cargo audit
```

Optional when installed/configured:

```bash
cargo nextest run
cargo llvm-cov nextest --workspace --all-features
cargo audit
cargo deny check
cargo fuzz run <target>
cargo bench
```

## Test locations

- External integration: `tests/*.rs` — use `use gridix::*;`, no `#[cfg(test)]` wrapper
- Inline unit: `#[cfg(test)] mod tests { use super::*; ... }` in source files
- Shared test utilities: `tests/common/mod.rs` provides `begin_key_pass()` and `focus_text_input()`
- egui 0.36 `RawInput` has no `modifiers` field. Inject `Event::ModifiersChanged(...)` before a key event and keep `Event::Key.modifiers` consistent; prefer `begin_key_pass()` for shared integration tests.
- egui 0.36 panics if a headless test drops an `end_pass()` output with unapplied texture deltas. Test helpers that do not own a renderer must call `ctx.end_pass().textures_delta.clear()`; production renderers must apply the delta instead.

## Patterns

**Pure logic** (most common):
```rust
#[test]
fn test_something() {
    let result = some_pure_function(input);
    assert_eq!(result, expected);
}
```

**Session test** (new — no egui Context needed):
```rust
#[test]
fn test_session_execute() {
    let mut session = Session::new_with_test_runtime();
    session.connect("test".to_string());
    let effects = session.poll_messages();
    assert!(effects.connections.len() > 0);
}
```

**egui component** (for widget behavior — uses real egui Context, no GPU needed):
```rust
#[test]
fn test_widget() {
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::Enter,
            pressed: true,
            modifiers: egui::Modifiers::default(),
            repeat: false,
            physical_key: None,
        }],
        ..Default::default()
    });
    egui::Area::new("test".into()).show(&ctx, |ui| {
        widget.ui(ui);
    });
    assert!(widget.some_property);
}
```

**Async** (for DB operations):
```rust
#[tokio::test]
async fn test_query() {
    let result = execute_query(&config, "SELECT 1").await;
    assert!(result.is_ok());
}
```

## Rules

- Run the narrowest affected test while iterating, then `cargo test --workspace --all-features` before merge.
- If testing egui keyboard behavior, use `egui::Event::Key` with `Key::Character` or `Key::Named`. <!-- doc-symbols: ignore: egui enum variants, not project symbols -->
- PostgreSQL/MySQL typed and cancellation integration tests read `GRIDIX_TEST_PG_URL` / `GRIDIX_TEST_MYSQL_URL`. Without a URL they may return locally; CI must preflight a non-empty URL and run them serially with `--nocapture --test-threads=1`.
- Fixture-bound acceptance suites (`tests/tls_acceptance.rs`, `tests/ssh_acceptance.rs`) run only with `GRIDIX_ACCEPTANCE=1`, which the dedicated acceptance workflows set. Without the flag they skip with a printed reason so `cargo test --workspace --all-features` stays green; with it, `common::required_env` keeps a missing fixture a hard failure.
- Server-side cancellation acceptance must observe a unique query marker, cancel it, observe `DbError::Cancelled`, confirm the marker disappears, then prove the connection remains usable. Do not replace this with fixed sleeps or task-abort assertions.
- `tests/postgres_tab_sessions.rs` exercises real per-Tab PostgreSQL transaction isolation, error/cancel-aborted state, and close/disconnect rollback. An unset `GRIDIX_TEST_PG_URL` skips locally and is not acceptance evidence. For an already-dispatched `COMMIT`, cancellation/Tab close reports an unknown transaction outcome; acceptance must verify queued SQL was discarded, not assert that the first write definitely rolled back.
- `ci.yml` and the standalone PostgreSQL integration workflow must run `postgres_tab_sessions` with a real `GRIDIX_TEST_PG_URL`; a passing no-URL local run only checks skip behavior. The tag release job also waits for both TLS fixture jobs via the reusable `tls-acceptance.yml` workflow. AppImage verification must execute Gridix through `AppRun`, not just check its runtime version. <!-- doc-symbols: ignore: AppRun is a generated AppDir symlink in CI, not a Rust symbol -->
- Session tests should not require `egui::Context`.
- Data layer tests should not require `Session`.
- `src/app/runtime/grid_identity_tests.rs` covers connection/database draft isolation, async deletion and Tab result provenance with no external database. For wrapped-import transaction controls, pair the core lexer tests with real PostgreSQL/MySQL integration tests and assert no preceding write persisted.

## Layer-specific testing

| Layer | Test type | Example |
|-------|-----------|---------|
| `core/` | Pure unit tests | `#[test] fn test_format_sql()` |
| `data/` | Async integration (SQLite in-memory for single connection; temp file for multi-connection metadata) | `#[tokio::test] async fn test_connect()` |
| `session/` | Unit with mock runtime | `#[test] fn test_poll_messages()` |
| `state/` | Pure unit tests | `#[test] fn test_apply_effects()` |
| `ui/` | egui Context tests | `#[test] fn test_dialog_rendering()` |

## Release-acceptance boundary

- Backend Actions workflows—not an unconfigured local test run—are the PostgreSQL/MySQL and TLS acceptance gates for PRs, `main`, and `v*` tags. The CI tag release job waits for their successful results; the standalone TLS workflow also supports scheduled and manual runs.
- RA2 remains a manual SQLite GUI journey: it requires initial, saved, and reopened-result screenshots plus non-empty CSV/JSON/SQL exports. CSV must contain `after`, JSON `"name":"after"`, and SQL `'after'` with `NULL`; `gridix --ci-check` and driver screenshots alone do not prove this journey.
- Status 2026-10-03: the 2026-09-25 screenshots and persisted-value assertion cover create/edit/reopen; a separate Xvfb + GTK portal session produced GUI-driven CSV, JSON and SQL files with `after` and verified them via `gridix-driver assert-export`. The export used a pre-seeded database and the screenshots/files are local temporary artifacts: one continuous same-database journey and release evidence retention remain outstanding; see `docs/LIMITATIONS.md`.
- Do not report a release or RA2 as accepted without the corresponding observed evidence.
