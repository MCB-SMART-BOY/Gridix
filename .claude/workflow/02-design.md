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
| Dialog completeness | Is the new DialogId handled in ALL match arms in host.rs? |
| Config persistence | Is save_config_debounced() called (not direct save_config)? |
| Session init | Are new Session fields initialized in Session::new()? |

### 5. Design Decision Record

For architectural changes, create an ADR in `references/architecture/decisions.md`:
```markdown
## ADR-XXX: Title
**Date:** YYYY-MM
**Status:** Proposed
**Context:** ...
**Decision:** ...
**Rationale:** ...
```

## Exit Criteria
- [ ] Layer impact documented
- [ ] Architecture rules verified
- [ ] Risk assessment complete
- [ ] ADR created (if architectural change)
