# Commit Message Template

## Format

```
type(scope): brief description

- Bullet point of what changed
- Bullet point of why
- Impact summary
```

## Types

| Type | Use when |
|------|----------|
| `feat` | New feature |
| `fix` | Bug fix |
| `refactor` | Code change that neither fixes nor adds |
| `docs` | Documentation only |
| `chore` | Maintenance, dependencies |
| `test` | Adding or updating tests |

## Scopes

| Scope | Module |
|-------|--------|
| `session` | src/session/ |
| `state` | src/state/ |
| `data` | src/data/ |
| `ui` | src/ui/ |
| `core` | src/core/ |
| `config` | src/core/config.rs |
| `grid` | src/ui/components/grid/ |
| `editor` | src/ui/components/sql_editor.rs |

## Examples

```
feat(saved-queries): add save/load for frequently used SQL

- Add SavedQuery {name, sql, created_at} to AppConfig
- Add toolbar button + popup dialog for saved queries
- Persist via existing config save mechanism
- Zero test failures
```

```
fix(session): prevent stale response from clearing other tab's grid

- Add workspace_id check in handle_grid_save_done
- Verify tab_id matches before clearing grid state
- Add test for cross-tab save isolation
```
