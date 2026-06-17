# Stage 1: Plan

## Entry Criteria
- [ ] User has stated a goal or problem
- [ ] No existing code changes pending

## Activities

1. **Understand scope**
   - Read `CLAUDE.md` for project context
   - Check `references/roadmap.md` for planned features
   - Check `references/tech-debt.md` for related issues

2. **Explore codebase**
   - Identify affected modules using module map
   - Check layer dependencies (`references/architecture/layers.md`)
   - Find existing patterns to follow

3. **Design approach**
   - Choose architectural pattern from `references/architecture/decisions.md`
   - Verify layer direction (types ← core ← data ← session ← state ← ui)
   - Estimate scope: single file / module / cross-cutting

4. **Present plan**
   - State what will change and why
   - Identify risk areas (see `references/tech-debt.md` for known pitfalls)
   - Get user approval before coding

## Exit Criteria
- [ ] Plan approved by user
- [ ] Affected modules identified
- [ ] Risk areas noted

## Artifacts
- Plan description (can be in conversation)
- For large changes: written in `references/architecture/decisions.md` as ADR
