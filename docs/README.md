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

### Backend release-acceptance gates

The [PostgreSQL integration workflow](../.github/workflows/postgresql-integration.yml) and [MySQL integration workflow](../.github/workflows/mysql-integration.yml) run typed execution and server-cancellation integration tests on pull requests, pushes to `main`, and `v*` tags; both also retain manual dispatch and weekly scheduled runs. The workflows require non-empty `GRIDIX_TEST_PG_URL` or `GRIDIX_TEST_MYSQL_URL`, respectively, before testing, and run each fixed-table test binary serially with `--nocapture`.

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

These Actions checks are release-acceptance gates, not evidence that a release has been published. Release acceptance still requires a manually observed SQLite GUI journey covering create, edit/save, reopen, and CSV/JSON/SQL export. `gridix-driver` currently supports only `launch`, `key`, `ss`, `quit`, and `help`; it cannot drive the required text entry, pointer actions, dialogs, or wait-for behavior for that journey.

**Code is the source of truth.** `.claude/` stays in sync with code.
