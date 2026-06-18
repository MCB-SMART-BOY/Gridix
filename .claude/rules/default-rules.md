---
paths:
  - .claude/rules/default.rules
  - .claude/settings.json
description: Explains the dual permission system. default.rules is Codex format (prefix_rule); settings.json is Claude Code format (permissions.allow). When adding new approved commands, update both.
---

# Permission Rules — Dual Format

Gridix maintains two permission files in parallel:

| File | Format | Consumed by | Purpose |
|------|--------|-------------|---------|
| `default.rules` | Codex `prefix_rule(pattern=[...], decision="allow")` | Codex | Bash command allowlisting |
| `../settings.json` | Claude Code `permissions.allow` | Claude Code | Bash command allowlisting |

## How to add a new permission

1. **Add to `default.rules`**: Append `prefix_rule(pattern=["<command>", "<arg1>", ...], decision="allow")`
2. **Add to `settings.json`**: Add the command pattern to `permissions.allow` array

## Key permissions mapped

| Operation | `default.rules` pattern | `settings.json` equivalent |
|-----------|------------------------|---------------------------|
| Build check | `["cargo", "check"]` | `"cargo check"` |
| Git push | `["git", "push", "origin"]` | `"git push origin"` |
| GH release | `["gh", "release", "create"]` | `"gh release create *"` |
| GH PR comment | `["gh", "pr", "comment"]` | `"gh pr comment *"` |
| Curl downloads | `["curl", "-L"]` | `"curl -L *"` |
| Nix builds | `["nix-build", "-A"]` | `"nix-build -A *"` |

## Note

`default.rules` is synced from `~/.codex/rules/default.rules` — don't edit it manually without also updating the Codex original. `settings.json` is Claude Code-specific and NOT synced to `~/.codex/` (it lives only in `.claude/`).
