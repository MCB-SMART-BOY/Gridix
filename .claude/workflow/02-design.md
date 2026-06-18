# Stage 2: Design

## Entry Criteria
- [ ] Scope and success criteria are clear
- [ ] Affected modules identified

## Activities

### 1. Impact Analysis

Determine affected areas:
- data/config/API/schema
- domain logic
- async/background work
- UI/transport
- tests/docs/CI
- deployment/release

For layered projects, document dependency order before editing.

### 2. Architecture Check

- [ ] Boundaries stay explicit
- [ ] Public API changes are intentional
- [ ] Side effects stay near boundaries
- [ ] Config/data migrations are backward compatible
- [ ] New dependencies are justified

### 3. Safety Plan

Choose verification before implementation:
- Unit tests for pure logic
- Integration tests for boundaries
- Regression test for bug fixes
- Characterization tests for refactors
- Benchmark/profile for optimization
- Migration tests for persisted data/config

### 4. Implementation Plan

- [ ] Smallest first slice identified
- [ ] Adapter/compatibility bridge considered
- [ ] Rollback path known
- [ ] Tests and commands listed

### 5. Design Decision Record

For architectural changes, create/update an ADR or design note:

```markdown
## ADR-XXX: Title
**Date:** YYYY-MM
**Status:** Proposed
**Context:** ...
**Decision:** ...
**Rationale:** ...
```

## Exit Criteria
- [ ] Impact documented
- [ ] Architecture/risk checked
- [ ] Test plan exists
- [ ] ADR/design note created if needed
