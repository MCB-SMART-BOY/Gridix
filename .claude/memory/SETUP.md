# Memory System

Gridix uses persistent memory notes. Primary store: `~/.codex/memory/` (Codex). Mirror: `.claude/memory/` (Claude Code).

## How it works

- Memory files persist between sessions
- Each file holds one fact with frontmatter (name, description, metadata)
- `MEMORY.md` serves as the index
- Memories can link to each other with `[[name]]` syntax

## Types

| type | purpose |
|------|---------|
| `user` | Who the user is, their preferences, expertise |
| `feedback` | Guidance on how the AI agent should work, corrections |
| `project` | Ongoing work, goals, constraints |
| `reference` | External resources, URLs, documentation links |

## When to save

- User explicitly asks to remember something
- User provides feedback on agent behavior
- A significant architectural decision is made
- A new constraint or rule is established
