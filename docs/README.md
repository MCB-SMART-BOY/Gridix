# Gridix Docs

## User docs

- [CHANGELOG.md](CHANGELOG.md) — version history
- [LEARNING_CURRICULUM.md](LEARNING_CURRICULUM.md) — in-app learning content spec
- [LIMITATIONS.md](LIMITATIONS.md) — current product limitations and verification boundaries

## Developer reference

All engineering knowledge lives in `.claude/`:
- `CLAUDE.md` — architecture, module map, conventions, task navigation
- `.claude/skills/` — executable workflows (build/run, keybindings, PR prep, release, troubleshoot)
- `.claude/rules/` — domain rules auto-loaded when editing matching files
- `.claude/references/` — engineering ledgers (invariants, contracts, bug ledger, query trace)

### Typed query runtime

The typed execution surface is `execute_typed` and `execute_typed_cancellable`. PostgreSQL preserves `NUMERIC` values as exact `DbValue::Decimal` text for parameter binding and result decoding. MySQL temporal input requires microsecond precision and rejects nanoseconds at or above one second.

Cancellation is backend-specific. PostgreSQL sends a driver `CancelToken` request and waits for the original query to return; MySQL uses the execution connection ID with a separately opened TLS-configured control connection that issues `KILL QUERY` and then waits for the original query. SQLite's synchronous `rusqlite` execution cannot safely interrupt a statement that has already started, so the cancellable API does not promise in-flight SQLite cancellation.

### Backend and GUI release-acceptance gates

The PostgreSQL and MySQL acceptance jobs in the main CI workflow are required dependencies of the tag release job. They run typed execution and server-cancellation integration tests before a GitHub Release can be published. The standalone PostgreSQL/MySQL workflows remain useful for scheduled, manual, PR, and `main` diagnostics.

The main CI workflow also runs a clean Xvfb/X11 GUI smoke job on `main`, tags, and manual dispatch. It uploads a manifest and initial screenshot, but records `ra2_status=manual-required`; this smoke does not claim that native dialogs or the full SQLite journey succeeded.

Run the same binaries locally against disposable databases:

```bash
GRIDIX_TEST_PG_URL='<PostgreSQL test URL>' \
  cargo test --test postgres_typed_e2e -- --nocapture --test-threads=1
GRIDIX_TEST_PG_URL='<PostgreSQL test URL>' \
  cargo test --test postgres_cancel_integration -- --nocapture --test-threads=1

GRIDIX_TEST_MYSQL_URL='<MySQL test URL>' \
  cargo test --test mysql_typed_e2e -- --nocapture --test-threads=1
GRIDIX_TEST_MYSQL_URL='<MySQL test URL>' \
  cargo test --test mysql_cancel_integration -- --nocapture --test-threads=1
```

The CI acceptance jobs are release-acceptance gates, not evidence that a release has been published. Release acceptance still requires a manually observed SQLite GUI journey covering create, edit/save, reopen, and CSV/JSON/SQL export. Run `scripts/gridix-sqlite-acceptance.sh <SHA>` after building the release binaries; it creates an isolated Xvfb session, an operator runbook, and validates the retained screenshots, database value, and exports. `gridix-driver` supports launch, keyboard, text, pointer, wait, screenshot, and artifact assertions, but native dialogs and semantic GUI actions remain manual.

**Code is the source of truth.** `.claude/` stays in sync with code.
