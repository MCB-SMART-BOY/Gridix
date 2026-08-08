---
name: project-context
description: Current project state, architecture, and active constraints
metadata:
  type: project
---

# Gridix Project Context

## Current State

- **Version**: 7.1.0
- **Branch**: `main` (sole branch — `dev`, `EDU`, `master` consolidated and deleted)
- **TLS**: rustls 0.23 only — zero native-tls/openssl in dependency tree
- **Verification state**: typed-runtime and backend integration workflows are configured; do not infer a release result from configuration. RA2 manual SQLite GUI evidence remains outstanding.
- **Merge**: v6.1.0 core (rustls, state migration) + v7.0.0 features (workbench, ER rewrite, design tokens, 36 audit fixes)


## Architecture State

- 6-layer unidirectional architecture: types(-1) ← core(0) ← data(1) ← session(2) ← state(3) ← ui/app(4)
- DbManagerApp: ~11 fields (from ~100). 89 migrated to Session(~30) + UiState(~60).
- self.sql dual source: ELIMINATED. Sole authority = QueryTab.sql via active_sql()/set_active_sql()
- Config version: 2 (with #[serde(default)] for backward compat)
- Config save: 5-second debounce via save_config_debounced()
- Handler repaint: needs_repaint flag replaces ctx.request_repaint()
- Typed domain layer: `src/domain/` (`DbValue`, `ResultSet`, `SchemaCatalog`, `MutationBatch`) is used by SQLite, PostgreSQL, and MySQL typed execution, mutations, and catalog loading.
- Query runtime: `TaskRegistry` + `RuntimeEvent` own `OperationKey` deduplication, stale-event filtering, and cooperative `CancellationToken` cancellation.
- Public query APIs: `execute_typed()` and `execute_typed_cancellable()`. PostgreSQL uses `CancelToken`; MySQL uses execution `Conn::id()` plus a separately opened TLS-configured control connection running `KILL QUERY`; SQLite does not promise cancellation of a running synchronous statement.
- Type boundaries: PostgreSQL preserves `DbValue::Decimal` for `NUMERIC` binding and decoding; MySQL temporal input rejects invalid values, including nanoseconds at or above one second.

## Key Constraints

- NO batch sed for field migration — causes cross-struct corruption
- NO trait objects for DB backends — use match db_type
- NO new cross-layer imports without documenting in architecture/decisions.md
- EVERY DialogId variant must be handled in ALL match arms in host.rs
- Field migration: add to target struct FIRST, then migrate ONE ref at a time

## Active Tech Debt

- FrameEffects defined in `session/frame_effects.rs` but not wired
- Driver and grid-filter coverage remains uneven outside typed integration paths
- Several UI/input modules remain oversized
- RA2 manual SQLite GUI create/query/edit/save/reopen/export evidence is still required for release acceptance
- MySQL cancellation CI currently covers direct `mysql:8.4` with observer/KILL privileges, not TLS, SSH tunnel, execution-pool pressure, or reuse of the exact cancelled `Conn`
- Session fields are all pub (acceptable for single-crate project)

## Build Commands

```bash
cargo test --workspace --all-features
cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features
cargo doc --workspace --no-deps
```
