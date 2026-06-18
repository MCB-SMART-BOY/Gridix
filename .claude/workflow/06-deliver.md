# Stage 7: Deliver

## Entry Criteria
- [ ] All quality gates pass (clippy, fmt, test, check-doc-links)
- [ ] Stage 6 (Verify) complete — behavior confirmed
- [ ] No unresolved review findings

## Activities

### 1. Summarize the change

Write a concise summary of:
- What changed and why
- What was considered but rejected (if non-obvious)
- Any follow-up work or known limitations

### 2. Update the harness

Every code change that affects architecture, conventions, APIs, or behavior must be reflected in the knowledge base:

| change type | update this |
|---|---|
| New module / moved file | `CLAUDE.md` module map, relevant `rules/` paths |
| New AppAction / command | `CLAUDE.md` variant counts, `skills/keybindings/SKILL.md` |
| Workbench layout change | `.claude/references/workbench-ui-design.md`, `.claude/references/workbench-ui-refactor-spec.md`, `.claude/references/dockable-workbench-v2.md` |
| Database / pool / query change | `.claude/rules/database.md`, `.claude/references/query-execution.md` |
| Session / connection change | `.claude/rules/session.md`, `.claude/references/core-flows.md` |
| New invariant / changed flow | `.claude/references/core-flows.md` |
| Bug fixed / new observation | `.claude/references/bug-ledger.md` |
| Tech debt found / resolved | `.claude/references/tech-debt.md` |
| Roadmap item completed | `.claude/references/roadmap.md` |
| Dialog / UI pattern change | `.claude/references/dialog-audit.md`, `.claude/rules/ui-egui.md` |
| ER diagram / metadata change | `.claude/references/er-contracts.md` |
| Config field / env var changed | `CLAUDE.md` config/env sections |
| Build dependency / version bump | `CLAUDE.md` quick commands section |

After updating `.claude/`, sync to `~/.codex/` if the change is relevant there:
```bash
rsync -av .claude/{references,rules,templates,workflow,skills,memory}/ ~/.codex/{references,rules,templates,workflow,skills,memory}/
```

### 3. Update project memory

If the change establishes a new constraint, preference, or project state:
- Write to `.claude/memory/` (and `~/.codex/memory/`)
- Update `MEMORY.md` index with the new entry

### 4. Commit

- One logical change per commit
- Message format: `type(scope): description`
- Types: `feat`, `fix`, `refactor`, `docs`, `chore`, `test`
- Scopes: `session`, `state`, `data`, `ui`, `core`, `config`, `grid`, `editor`, `workbench`
- NO `Co-Authored-By: Claude` in commit messages

Example:
```
fix(session): guard stale query responses with request_id check
```

### 5. If this is a release

Run `skills/release/SKILL.md` — version bump, changelog, tag, publish.

### 6. If this closes an issue or PR

- Reference the issue/PR number in the commit message body
- Update issue status if applicable

## Exit Criteria
- [ ] Change summary written
- [ ] Knowledge base files updated (`.claude/` + `~/.codex/` as needed)
- [ ] Project memory updated if new constraints established
- [ ] Committed with proper message format
- [ ] Release published (if applicable)
- [ ] No stale documentation left behind

## Quick Reference

| Task | Tool / Skill |
|------|-------------|
| Sync .claude → .codex | `rsync -av .claude/{references,rules,templates,workflow,skills,memory}/ ~/.codex/{...}/` |
| Sync .codex → .claude | `rsync -av ~/.codex/{references,rules,templates,workflow,skills,memory}/ .claude/{...}/` |
| Commit | `git commit -m "type(scope): description"` |
| Release | `skills/release/SKILL.md` |
| Update memory | Write to `.claude/memory/<slug>.md` → update `MEMORY.md` index |
