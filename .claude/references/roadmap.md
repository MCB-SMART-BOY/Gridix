# Future improvement roadmap

## Done (v6.1.0+)

### Dependency health
- [x] russh 0.58.1→0.61 (yanked crate + 2 HIGH vulns fixed)
- [x] tokio-postgres 0.7→0.7.18 (MEDIUM vuln fixed)
- [x] Consider replacing `syntect` — replaced with custom tokenizer, dependency removed
- [x] Replaced `once_cell` with `std::sync::LazyLock`
- [x] Replaced `lazy_static` with `std::sync::LazyLock`
- [x] Replaced `parking_lot::Mutex` with `std::sync::Mutex` (RwLock kept for re-entrant safety)

### CI hardening
- [x] Add `cargo audit` to CI quality gate
- [x] Add `rust-toolchain.toml` to pin toolchain version
- [x] Add quality checks (fmt+clippy+test) to release workflow before build
- [x] Add PostgreSQL service container for integration tests in CI
- [x] Add `cargo-tarpaulin` for code coverage reporting

### Test infrastructure
- [x] Create `tests/common/mod.rs` for shared test fixtures
- [x] Deduplicate identical test files

### Architecture
- [x] Delete `DatabaseDriver` trait — removed as dead code
- [x] Remove `app/state/mod.rs` dead code
- [x] Remove `core/session.rs` dead code (SessionManager — replaced by session/ layer)
- [x] Delete dead grid-save pipeline (~150 lines)
- [x] Fix 12 clippy errors (0 remaining)

### Security
- [x] PostgreSQL default SSL: Disable → Prefer
- [x] MySQL default SSL: Disabled → Preferred
- [x] Fix Required/Require SSL modes to validate certificates
- [x] SSH password/passphrase `#[serde(skip_serializing)]`
- [x] Close `DbManagerApp` public API exposure (pub mod app → pub(crate))
- [x] Mutex poison handling (`.unwrap()` → `.unwrap_or_else(|e| e.into_inner())`)
- [x] SSH host key SHA-256 fingerprint in error messages

## Near-term (next release)

### Architecture refactoring (in progress)
- [ ] Phase A: Create `src/types.rs` shared types layer
- [ ] Phase B: Reorganize `data/` layer, extract `session/` module
- [ ] Phase C: Extract `UiState` — 4-field `DbManagerApp`
- [ ] Phase D: Eliminate `self.sql` dual source

### Dependency health
- [ ] Monitor `rsa` crate — 1 remaining MEDIUM vuln (no fix available, transitive via russh)
- [ ] Watch `mysql_async` for `lru` unsound fix

### Test infrastructure
- [ ] Add `proptest` or `quickcheck` for property-based testing of SQL parser/formatter
- [ ] Add unit tests for `data/pool.rs` (currently zero coverage)
- [ ] Add unit tests for `data/ssh_tunnel.rs` (config only, no connectivity tests)
- [ ] Add Session tests (poll_messages, FrameEffects)
- [ ] Add PostgreSQL/MySQL driver tests

## Medium-term (v7.0.0)

### Features
- [ ] Query plan visualization (EXPLAIN output rendered as tree/table)
- [ ] Database schema diff/compare tool
- [ ] Dark/light theme auto-switch based on system preference
- [ ] Export to additional formats (Parquet, Excel via calamine)

### Performance
- [ ] Virtual scrolling for large query results (>100K rows)
- [ ] Async metadata pre-fetching (triggers, routines loaded on connection, not on demand)
- [ ] Syntax highlight cache warming for common SQL patterns

## Long-term

- [ ] Plugin system for database extensions (custom drivers, import/export formats)
- [ ] WebAssembly build for browser-based Gridix (egui→eframe web backend)
- [ ] Multi-window support (detach query tabs, ER diagrams as separate windows)
- [ ] Accessibility: screen reader support, high-contrast theme, keyboard-only audit
