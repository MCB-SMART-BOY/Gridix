# Stage 1: Plan

## Entry Criteria
- [ ] User goal and acceptance criteria are understood
- [ ] Existing changes and relevant evidence are accounted for

## Activities
1. Identify affected user flow, modules, callers, and existing tests/workflows.
2. Define non-goals and observable success evidence; distinguish local convenience from CI or release evidence.
3. For database changes, determine backend-specific behavior. Typed APIs are `execute_typed` and `execute_typed_cancellable`; SQLite does not promise in-flight cancellation.
4. For release-sensitive work, plan PostgreSQL/MySQL Actions evidence and any required manual RA2 GUI artifacts. Do not infer acceptance from a passing unit suite.
5. Request approval only for material trade-offs, irreversible actions, or expanded scope.

## Exit Criteria
- [ ] Smallest coherent change and affected modules identified
- [ ] Risks and verification method stated
- [ ] Required approval obtained when needed
