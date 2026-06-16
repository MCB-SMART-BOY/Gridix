# Future improvement roadmap

## Done

### Refactoring (v6.2.0)
- [x] Phase A: `src/types.rs` — shared types (DatabaseType, SslModes, QueryResult)
- [x] Phase B: `src/session/` — Session struct, QueryTab, Message, FrameEffects
- [x] Phase C: `src/state/` — UiState struct (25 fields)
- [x] Phase D: Eliminate `self.sql` dual source
- [x] Session: runtime, tx, rx, manager, tab_manager, autocomplete, notifications, progress, query_history, command_history migrated
- [x] State: theme_manager, highlight_colors, ui_scale, base_pixels_per_point migrated
- [x] `database/` → `data/` rename
- [x] 25 duplicate fields removed from DbManagerApp (~100 → ~47)
- [x] 12 clippy errors → 0

### Dependency health
- [x] syntect replaced with custom tokenizer
- [x] once_cell → std::sync::LazyLock
- [x] lazy_static → std::sync::LazyLock
- [x] parking_lot::Mutex → std::sync::Mutex (RwLock kept)

### CI hardening
- [x] cargo audit in CI
- [x] rust-toolchain.toml
- [x] Quality gate (fmt+clippy+test) in release workflow
- [x] PostgreSQL service container for CI
- [x] cargo-tarpaulin coverage workflow

### Test infrastructure
- [x] tests/common/mod.rs shared utilities
- [x] Duplicate test files removed

### Security
- [x] SSL defaults: PG Prefer, MySQL Preferred
- [x] SSL Required modes validate certificates
- [x] SSH password skip_serializing
- [x] pub(crate) mod app (closed public API exposure)
- [x] Mutex poison handling
- [x] SSH host key fingerprint in errors
- [x] Dead DbError variants removed

## In progress

### Architecture
- [ ] Session field encapsulation (fields currently all `pub`)
- [ ] Complete UiState migration (20+ duplicate fields still on DbManagerApp)
- [ ] Wire FrameEffects → poll_messages() → apply_frame_effects()
- [ ] Config save throttling (15 call sites, no dedup)
- [ ] Unified error types (25 files use `Result<_, String>`)

### Code quality
- [ ] Split oversized files: keybindings_dialog.rs (3560), input_router.rs (3369), keybindings.rs (2448)
- [ ] Remove legacy password migration (v4 → OS keyring, add deprecation window)
- [ ] Remove legacy keybindings field from AppConfig (read-only since v4)

## Medium-term

### Features
- [ ] Query plan visualization (EXPLAIN rendered as tree/table)
- [ ] Database schema diff/compare
- [ ] Dark/light theme auto-switch
- [ ] Export to Parquet/Excel

### Performance
- [ ] Virtual scrolling for >100K row results
- [ ] Config save throttling (batch writes)
- [ ] Async metadata pre-fetching

## Long-term

- [ ] Plugin system for DB drivers
- [ ] WebAssembly build
- [ ] Multi-window support
- [ ] Accessibility (screen reader, high-contrast)
