# Stage 4: Review

## Entry Criteria
- [ ] All implementation complete
- [ ] Fast checks pass

## Self-Review Checklist

### Correctness
- [ ] Solves the stated problem
- [ ] Edge cases handled
- [ ] Errors are explicit and actionable
- [ ] Persisted data/config/API changes are compatible or migrated
- [ ] Existing invariants preserved

### Architecture
- [ ] Boundaries are maintained
- [ ] Side effects stay near boundaries
- [ ] New dependency justified
- [ ] Refactor does not hide behavior changes
- [ ] Workbench surface changes include descriptor metadata, bridge updates, and icon tooltip contract checks

### Tests
- [ ] New or changed behavior has relevant tests
- [ ] Bug fix has regression coverage when practical
- [ ] Refactor has characterization or existing coverage
- [ ] Optimization has measurement

### Documentation
- [ ] Docs/config examples updated if behavior changed
- [ ] ADR/design note updated if architecture changed
- [ ] Changelog updated for user-visible changes
- [ ] Local workflow docs updated if rules changed

## Verification Commands

Rust default:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Workspace Rust:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Exit Criteria
- [ ] All checklist items pass
- [ ] Required verification commands pass or skipped checks are justified
- [ ] No unrelated diff remains
