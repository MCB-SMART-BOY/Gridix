# Stage 5: Test

## Entry Criteria
- [ ] Stage 4 review passed
- [ ] All build checks pass

## Test Strategy by Layer

| Layer | Test type | Location | Dependency |
|-------|-----------|----------|------------|
| core | `#[test]` pure logic | `tests/core_tests.rs` + inline | None |
| data | `#[tokio::test]` SQLite in-memory | inline in `sqlite.rs` | None (in-memory) |
| session | `#[test]` with mock data | inline | No egui Context |
| state | `#[test]` state transitions | inline | No egui Context |
| ui | `#[test]` egui Context::default() | inline + `tests/ui_dialogs_tests.rs` | egui |

## Required evidence

- Add or update deterministic tests for newly observable behavior.
- For PostgreSQL/MySQL typed or cancellation changes, use the configured `GRIDIX_TEST_PG_URL` / `GRIDIX_TEST_MYSQL_URL` and run the relevant integration binary serially with `--nocapture --test-threads=1`. CI preflight makes missing URLs fail.
- Cancellation evidence must show: query marker observed, `DbError::Cancelled`, marker disappearance, and a reusable connection. A task join or fixed sleep is insufficient.
- For a release candidate, RA2 is a manual SQLite GUI journey. Preserve screenshots for initial query, saved edit, and reopened result, plus non-empty CSV/JSON/SQL exports: CSV contains `after`, JSON contains `"name":"after"`, and SQL contains `'after'` and `NULL`. The driver cannot type, wait for widgets, or operate file dialogs; it cannot automate RA2 end-to-end.

## Full validation

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo run --bin check-doc-links
cargo audit
```

## Exit Criteria
- [ ] Changed behavior has direct evidence
- [ ] Full validation passes
- [ ] Required backend Actions gates pass for the candidate SHA
- [ ] RA2 artifacts exist when a release candidate requires GUI acceptance
