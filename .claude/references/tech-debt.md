# Technical debt & design gaps

v6.3.0 — architecture migration complete. All logic paths verified.

## ✅ Resolved

- [x] DbManagerApp: ~100 → ~11 fields (~89 migrated to Session/UiState)
- [x] self.sql dual source — single source = tab_manager
- [x] State consistency — clear_result/clear_search sync mirror + tab
- [x] 6-layer unidirectional architecture
- [x] Config version field, 5s debounce throttle
- [x] needs_repaint handler/egui decoupling
- [x] 11 clippy errors → 0
- [x] Security: SSL cert validation, SSH password, public API, mutex poison
- [x] Dead code: ~800 lines, 4 duplicate tests, syntect/once_cell/lazy_static
- [x] SQLite driver tests (7 schema tests), AppError + ErrorKind
- [x] database → data rename
- [x] 3 cross-audit fixes (handler guards, layer imports, state consistency)
- [x] All docs synchronized

## Critical logic paths (verified)

| Path | Status |
|------|--------|
| needs_repaint lifecycle (set → check → clear → init) | ✅ |
| clear_result/clear_search mirror ↔ tab sync | ✅ |
| Config save debounce + tick + on_exit flush | ✅ |
| Handler request_id stale guards (6 guarded / 6 idempotent) | ✅ |
| Tab switch persist → switch → sync_from | ✅ |
| Connection pending_connect_requests guard | ✅ |

## Remaining (non-critical)

- FrameEffects types defined, not wired (needs_repaint works as minimal decoupling)
- 72 source files zero test coverage (data/query drivers, grid filter)
- 3 oversized files (keybindings_dialog 3560L, input_router 3369L, keybindings 2448L)
- Session fields all pub (single-crate project, no practical risk)
