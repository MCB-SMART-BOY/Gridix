# System Skills

These skills provide development infrastructure for the harness itself.

| Skill | Status | Description |
|---|---|---|
| `skill-creator` | **Claude Code adapted** | Guide for creating effective Claude Code skills. Original from Codex, adapted for Claude Code conventions. |
| `openai-docs` | **Claude Code adapted** | OpenAI developer documentation lookup, model selection, and upgrade guidance. Stripped of Codex-specific self-knowledge sections. Complements the built-in `claude-api` skill. |
| `imagegen` | Codex-only | Image generation using Codex built-in tools. Not usable in Claude Code (references Codex-specific `image_gen` tool). |
| `plugin-creator` | Codex-only | Codex plugin scaffolding (`.codex-plugin/plugin.json`). Not applicable to Claude Code. |
| `skill-installer` | Codex-only | Installs skills from `github.com/openai/skills` marketplace. Not applicable to Claude Code. |

## Differences from `.codex/` originals

The `skill-creator` and `openai-docs` SKILL.md files have been adapted:
- Codex-specific tool names replaced with Claude Code equivalents
- `$CODEX_HOME` paths changed to `~/.claude`
- Codex self-knowledge / manual sections removed (openai-docs)
- `agents/openai.yaml` metadata preserved but not actively used by Claude Code

The `imagegen`, `plugin-creator`, and `skill-installer` skills are verbatim copies from `~/.codex/` for reference only.
