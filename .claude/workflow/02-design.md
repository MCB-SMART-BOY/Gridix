# Stage 2: Design

## Entry Criteria
- [ ] Approved plan from Stage 1
- [ ] Affected modules identified

## Activities

### 1. Layer Impact Analysis

Determine which layers are affected:
```
types(-1) → core(0) → data(1) → session(2) → state(3) → ui/app(4)
```

IF change spans multiple layers, document dependency order.

### 2. Architecture Check

- [ ] No new cross-layer imports introduced
- [ ] core/ does not import from data/ (except documented `config.rs` exception)
- [ ] data/ does not import from session/ or ui/
- [ ] session/ does not import from state/ or ui/
- [ ] state/ does not import from app/

### 3. Pattern Selection

Check `references/architecture/decisions.md` for applicable ADRs:
- ADR-001: 6-layer dependency direction
- ADR-002: match db_type dispatch (no trait)
- ADR-003: Single process (no IPC)
- ADR-004: QueryTab.sql sole authority (no dual source)
- ADR-005: needs_repaint decoupling

### 4. Risk Assessment

| Risk | Check |
|------|-------|
| State inconsistency | Will mirror fields stay in sync with canonical tab state? |
| Stale response | Does the handler have a request_id guard? |
| Dialog completeness | Is the new DialogId handled in all host match arms? |
| Config persistence | Is `save_config_debounced()` used? |
| Backend cancellation | Does the backend retain and finish the original query after requesting server cancellation? |
| Release evidence | Are CI backend gates or RA2 artifacts explicitly planned rather than assumed? |

### 5. Backend-specific design

- Preserve `match db_type` dispatch; do not add a generic driver trait.
- PostgreSQL cancellation uses `CancelToken`; MySQL uses the execution `Conn::id()` with a separate TLS-configured control connection and `KILL QUERY`.
- SQLite executes synchronously and has no supported in-flight cancellation contract.
- PostgreSQL `NUMERIC` must preserve exact `DbValue::Decimal` values on parameter/result paths. MySQL temporal input rejects nanoseconds of one second or greater.

### 6. Design Decision Record

Record architectural changes in `references/architecture/decisions.md`; otherwise keep the design in the task plan.

## Exit Criteria
- [ ] Layer impact documented
- [ ] Backend, state, and security constraints considered where applicable
- [ ] Verification and release-evidence requirements are explicit
