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
- [x] Typed runtime query cutover: `TaskRegistry` + `RuntimeEvent` with cooperative cancellation

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
- Several UI/input modules remain oversized and should be split only with behavior-preserving characterization coverage.
- Historical migration residue may remain in legacy message/request-ID types, but new query runtime work must use `TaskRegistry` + `RuntimeEvent`, not pending-query maps or `QueryDone`.
- Release acceptance remains incomplete: the required manual SQLite GUI create/query/edit/save/reopen/export evidence has not been captured.
- MySQL cancellation evidence currently covers direct `mysql:8.4` with observer/KILL permissions only; TLS, SSH tunnel, execution-pool capacity pressure, and reuse of the exact cancelled `Conn` are not covered.
- Session fields all pub (single-crate project, no practical risk)
