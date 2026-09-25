# Technical debt & design gaps

v7.1.0 historical consolidation context; this ledger tracks current implementation and acceptance gaps rather than claiming all paths or documentation are verified.

## ✅ Resolved

- [x] DbManagerApp: ~100 → ~11 fields (~89 migrated to Session/UiState)
- [x] self.sql dual source — single source = tab_manager
- [x] State consistency — clear_result/clear_search sync mirror + tab
- [x] 6-layer unidirectional architecture
- [x] Config version field, 5s debounce throttle
- [x] needs_repaint handler/egui decoupling
- [x] 11 clippy errors → 0
- [x] Security: SSL cert validation, SSH password, public API, mutex poison
- Historical cleanup: dead code, duplicate tests, and obsolete dependencies removed
- Documentation drift is now machine-checked: `cargo run --bin check-doc-symbols` (CI via `ci.yml`/`docs.yml`) fails on backticked references to removed symbols and on path-qualified file references that no longer exist, and the first pass corrected ~30 stale references across `CLAUDE.md`, `.claude/rules/`, and `.claude/references/`; dated snapshots and design drafts carry explicit file-level exemptions instead of drifting silently
- [x] SQLite driver coverage and application error typing
- [x] database → data rename
- [x] 3 cross-audit fixes (handler guards, layer imports, state consistency)
- [x] Typed runtime query and metadata cutover: `TaskRegistry` + `RuntimeEvent` with cooperative cancellation; trigger/routine completions no longer duplicate legacy messages
- [x] Dialog ownership moved from `UiState` to `DbManagerApp`, removing the state → app dependency
- [x] Request tracking collapsed to presence-only `pending_*` fields: the four always-true request-id guards in `src/app/runtime/handler.rs` and the inert request-id fields are gone. <!-- doc-symbols: ignore: names fields that no longer exist --> Freshness is decided by `TaskRegistry::is_current()` at the event entry; the authoritative connection/database gate is the event arm itself (nothing is applied unless the event's connection id and database match the pending request), `handle_triggers_fetched`/`handle_routines_fetched` treat the pending field only as "in flight", and `metadata_context_matches_current` re-checks the active connection before touching the sidebar (`triggers_event_without_pending_request_does_not_replace_list`, `triggers_event_for_inactive_connection_does_not_replace_list`).
- [x] Query cancellation is connection-scoped: `OperationKey::Query { connection, document }`, `cancel_queries_for_connection` cancels only the disconnecting connection's in-flight queries, `cancel_queries_for_document` covers the tab-scoped path (registry tests `cancel_queries_for_connection_only_cancels_that_connection`, `cancel_queries_for_document_ignores_connection`).
- [x] Trigger/routine metadata tasks attach their handles and race the request against the cancellation token, so supersede and connection switches end them cooperatively instead of via the internal timeout (`src/app/runtime/metadata.rs`).
- [x] `load_schema_snapshot` (`src/data/query/mod.rs`) and its SQLite-only helper `load_snapshot` had no in-tree consumer and were removed; <!-- doc-symbols: ignore: both symbols were deleted --> the snapshot diff path now reads `SchemaSnapshot::from_catalog` directly (the dialog has used that since the Schema diff cutover).
- [x] Dialog fixed-width overflow (G41-B007) is closed for the remaining surfaces: `CreateDbDialog`, `CreateUserDialog`, and `ExportDialog` rows go through `src/ui/dialogs/responsive.rs` / `ui.horizontal_wrapped`, with headless width measurements as proof.

## Critical logic paths (current design)

| Path | Status |
|------|--------|
| needs_repaint lifecycle (set → check → clear → init) | ✅ |
| clear_result/clear_search mirror ↔ tab sync | ✅ |
| Config save debounce + tick + on_exit flush | ✅ |
| Task completion stale guard | `RuntimeEvent { TaskId, OperationKey }` is filtered by `TaskRegistry::is_current()` |
| Query cancellation | Query tokens request cooperative server-side cancellation; query handles are not force-aborted |
| Connection pending_connect_requests guard | ✅ |

## Remaining (non-critical)

- FrameEffects types defined, not wired (needs_repaint works as minimal decoupling)
- Driver and grid-filter coverage remains uneven outside the typed integration paths.
- `src/app/input/input_router/` (原 3558 行, 54% 是测试) 已按行为领域拆为 `input_router/{mod,scopes,actions,context,resolve}.rs` + `tests/{er_diagram,er_focus,keymap,workspace,editor,grid,dialogs,misc}.rs` (最大 526 行)，`src/ui/panels/sidebar/` 的按键处理已拆到 `keyboard.rs`。仍偏大的模块只有覆盖率保护较弱的部分：`src/ui/dialogs/keybindings_dialog.rs` (约 3.7k 行)、`src/core/keybindings.rs` (约 2.6k 行)、`src/app/runtime/handler.rs` (约 2.1k 行)、`src/ui/panels/sidebar/mod.rs` (约 1.5k 行)。行数取整避免随每次改动过期。
- Historical migration residue may remain in legacy connection/database/table/import message/request-ID types; new query and metadata runtime work must use `TaskRegistry` + `RuntimeEvent`, not pending-query maps or duplicate legacy sends.
- Release acceptance is partially captured (2026-09-25 driven Xvfb run): connection create, query, Grid edit/save, and reopen persistence passed, including `gridix-driver assert-reopened … items name after` against the file on disk. The CSV/JSON/SQL export evidence is still missing because export needs a native save dialog that the driven session cannot present; see `docs/LIMITATIONS.md`.
- MySQL cancellation evidence covers direct `mysql:8.4` with observer/KILL permissions (`mysql-integration.yml` plus the `mysql-acceptance` job in `ci.yml`, re-run locally 2026-09-25); MySQL cancellation over TLS or an SSH tunnel is not covered. TLS acceptance itself runs in `tls-acceptance.yml` (pull requests, `main`, tags, weekly, manual); the SSH tunnel suite runs only through the manually dispatched `ssh-acceptance.yml`, which needs repository SSH secrets. Execution-pool capacity pressure, `remove_pool` recycling, and post-`clear_all` usability are covered by `tests/mysql_pool_acceptance.rs` and `tests/postgres_pool_acceptance.rs`, each needing only its own backend URL: `mysql-integration.yml` / `postgresql-integration.yml` run their whole suite, and the `ci.yml` acceptance jobs run theirs in a dedicated step with `GRIDIX_ACCEPTANCE=1` (verified locally 2026-09-25 against `mysql:8.4` and `postgres:16` with the other backend's URL deliberately unset). Pool behaviour over TLS or an SSH tunnel, and the exact cancelled `Conn` identity, still have no observable handle.
- Session fields all pub (single-crate project, no practical risk)
