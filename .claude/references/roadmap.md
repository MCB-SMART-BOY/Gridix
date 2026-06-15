# Future improvement roadmap

## Near-term (next release)

### Dependency health
- [x] russh 0.58.1→0.61 (yanked crate + 2 HIGH vulns fixed)
- [x] tokio-postgres 0.7→0.7.18 (MEDIUM vuln fixed)
- [ ] Monitor `rsa` crate — 1 remaining MEDIUM vuln (no fix available, transitive via russh)
- [ ] Watch `mysql_async` for `lru` unsound fix
- [ ] Consider replacing `syntect` if it remains unmaintained (uses yaml-rust, bincode)

### CI hardening
- [ ] Add `cargo audit` to CI quality gate
- [ ] Add `rust-toolchain.toml` to pin toolchain version
- [ ] Add quality checks (fmt+clippy+test) to release workflow before build
- [ ] Add PostgreSQL service container for integration tests in CI
- [ ] Add `cargo-tarpaulin` for code coverage reporting

### Test infrastructure
- [ ] Create `tests/common/mod.rs` for shared test fixtures
- [ ] Add `proptest` or `quickcheck` for property-based testing of SQL parser/formatter
- [ ] Deduplicate identical test files (ddl_tests.rs = ddl_dialog_tests.rs)
- [ ] Add unit tests for `database/pool.rs` (currently zero coverage)
- [ ] Add unit tests for `database/ssh_tunnel.rs` (config only, no connectivity tests)

## Medium-term (v7.0.0)

### Architecture
- [ ] Resolve `DatabaseDriver` trait — either implement it across backends or delete it
- [ ] Unify `self.sql` dual-source — make tab the sole authority
- [ ] Complete `app/state/mod.rs` refactoring or remove dead code
- [ ] Extract `GridWorkspaceStore` patterns into a reusable workspace management layer

### Features
- [ ] PostgreSQL integration test suite in CI (matching MySQL's)
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
- [ ] Collaborative editing via CRDT for shared connections
- [ ] Multi-window support (detach query tabs, ER diagrams as separate windows)
- [ ] Accessibility: screen reader support, high-contrast theme, keyboard-only audit
