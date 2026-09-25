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
- [x] Historical cleanup: dead code, duplicate tests, and obsolete dependencies removed
- [x] SQLite driver coverage and application error typing
- [x] database → data rename
- [x] 3 cross-audit fixes (handler guards, layer imports, state consistency)
- [x] Typed runtime query and metadata cutover: `TaskRegistry` + `RuntimeEvent` with cooperative cancellation; trigger/routine completions no longer duplicate legacy messages
- [x] Dialog ownership moved from `UiState` to `DbManagerApp`, removing the state → app dependency
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
- Four request-id "stale guards" in `src/app/runtime/handler.rs` (lines ~184, ~247, ~615, ~666) compare a request id against the same value they were dispatched with, so they are always true. The effective stale protection is `TaskRegistry::is_current()` plus `metadata_context_matches_current`; the `pending_{triggers,routines}_request` id fields are inert and should be collapsed.
- `cancel_queries_for_connection(&str)` ignores its connection name and cancels every active query, because `OperationKey::Query` carries only the document. Disconnecting one connection therefore cancels in-flight queries of other connections; rename to reflect the real scope, or add the connection to the key.
- Trigger/routine metadata tasks are registered but never attached, so `cancel_by_key` has no handle: supersede/connection switches only end them through the internal timeout instead of cooperative cancellation.
- `SchemaSnapshot`/`SchemaDiff` and `load_schema_snapshot` are library API with unit tests only; no app/ui/bin consumer exists, so the Schema diff roadmap item stays open until a surface or CLI entry point consumes them.
- Driver and grid-filter coverage remains uneven outside the typed integration paths.
- `src/app/input/input_router.rs` (3558 行, 54% 是测试) 已按行为领域拆为 `input_router/{mod,scopes,actions,context,resolve}.rs` + `tests/{er_diagram,er_focus,keymap,workspace,editor,grid,dialogs,misc}.rs` (最大 512 行)，`src/ui/panels/sidebar/` 的按键处理已拆到 `keyboard.rs`。仍偏大的模块只有覆盖率保护较弱的部分：`src/ui/dialogs/keybindings_dialog.rs` (3733 行)、`src/core/keybindings.rs` (2549 行)、`src/app/runtime/handler.rs` (1870 行)、`src/ui/panels/sidebar/mod.rs` (1474 行)。
- Historical migration residue may remain in legacy connection/database/table/import message/request-ID types; new query and metadata runtime work must use `TaskRegistry` + `RuntimeEvent`, not pending-query maps or duplicate legacy sends.
- Release acceptance is partially captured (2026-09-25 driven Xvfb run): connection create, query, Grid edit/save, and reopen persistence passed, including `gridix-driver assert-reopened … items name after` against the file on disk. The CSV/JSON/SQL export evidence is still missing because export needs a native save dialog that the driven session cannot present; see `docs/LIMITATIONS.md`.
- MySQL cancellation evidence currently covers direct `mysql:8.4` with observer/KILL permissions only; TLS, SSH tunnel, execution-pool capacity pressure, and reuse of the exact cancelled `Conn` are not covered.
- Session fields all pub (single-crate project, no practical risk)
