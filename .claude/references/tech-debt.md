# Technical debt & design gaps

v6.3.0 — architecture migration complete.

## ✅ All Resolved

- [x] DbManagerApp: ~100 → ~11 fields (~89 migrated)
- [x] self.sql dual source — eliminated
- [x] State consistency — clear_result/clear_search sync
- [x] 6-layer unidirectional architecture
- [x] 11 clippy errors — 0 remaining
- [x] Security: SSL, SSH, public API, mutex poison
- [x] Dead code: ~800 lines removed
- [x] Dependencies: syntect, once_cell, lazy_static removed
- [x] Config: version field, 5s debounce throttle
- [x] Tests: SQLite driver (7 tests), tests/common/mod.rs
- [x] AppError + ErrorKind types
- [x] needs_repaint handler/egui decoupling
- [x] Database → data rename
- [x] All docs synchronized

## Remaining (non-critical)

- FrameEffects defined but not wired (needs_repaint provides minimal decoupling)
- 3 oversized files (keybindings_dialog 3560L, input_router 3369L, keybindings 2448L)
- 72 source files with zero test coverage
- Session fields all pub (single-crate project, no practical risk)
- Legacy v4 password migration code (no deprecation window)
