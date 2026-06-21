# Stage 0-1: Intake And Discovery

## Entry Criteria
- [ ] User has stated a goal or problem
- [ ] Existing code changes have been checked and will not be overwritten

## Activities

1. **Classify work**
   - Feature, bug fix, refactor, optimization, test hardening, migration, or release
   - Define success criteria and non-goals
   - Identify risk level and affected users/code paths

2. **Read workflow references**
   - General: `references/modern-software-engineering-workflow.md`
   - Rust: `references/rust-modern-engineering-playbook.md` when applicable
   - Project rules: roadmap, tech-debt, architecture decisions, local rules
   - Gridix refactors: `references/project-refactor-execution-plan.md`

3. **Explore codebase**
   - Locate entry points and ownership boundaries
   - Find existing patterns and tests
   - Identify invariants and high-risk flows
   - For bugs, reproduce or document why reproduction is not possible
   - For optimization, capture baseline measurement

4. **Decide next step**
   - Direct implementation for small low-risk tasks
   - Written plan for cross-cutting or high-risk work
   - Safety-net tests before refactor/bug fix when practical

## Exit Criteria
- [ ] Work type and success criteria are known
- [ ] Affected modules identified
- [ ] Risk areas noted
- [ ] Verification method identified

## Artifacts
- Short plan in conversation or issue
- For large changes: ADR, execution plan, or design note
