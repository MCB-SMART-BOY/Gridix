---
name: skill-creator
description: Guide for creating effective Claude Code skills. Use when users want to create a new skill (or update an existing skill) that extends Claude Code's capabilities with specialized knowledge, workflows, or tool integrations.
paths:
  - "**/SKILL.md"
  - .claude/settings.json
---

# Skill Creator (Claude Code)

This skill provides guidance for creating effective Claude Code skills.

## About Skills

Skills are modular, self-contained folders that extend Claude Code's capabilities by providing specialized knowledge, workflows, and tools. They transform Claude Code from a general-purpose agent into a specialized agent equipped with procedural knowledge that no model can fully possess.

### What Skills Provide

1. Specialized workflows — Multi-step procedures for specific domains
2. Tool integrations — Instructions for working with specific file formats or APIs
3. Domain expertise — Project-specific knowledge, schemas, business logic
4. Bundled resources — Scripts, references, and assets for complex and repetitive tasks

## Core Principles

### Concise is Key

The context window is a public good. Skills share the context window with everything else Claude Code needs: system prompt, conversation history, other skills' metadata, and the actual user request.

**Default assumption: Claude Code is already very smart.** Only add context Claude doesn't already have. Challenge each piece of information: "Does Claude really need this explanation?" and "Does this paragraph justify its token cost?"

Prefer concise examples over verbose explanations.

### Set Appropriate Degrees of Freedom

Match the level of specificity to the task's fragility and variability:

**High freedom (text-based instructions)**: Use when multiple approaches are valid, decisions depend on context, or heuristics guide the approach.

**Medium freedom (pseudocode or scripts with parameters)**: Use when a preferred pattern exists, some variation is acceptable, or configuration affects behavior.

**Low freedom (specific scripts, few parameters)**: Use when operations are fragile and error-prone, consistency is critical, or a specific sequence must be followed.

### Anatomy of a Skill

Every skill consists of a required SKILL.md file and optional bundled resources:

```
skill-name/
├── SKILL.md (required)
│   ├── YAML frontmatter metadata (required)
│   │   ├── name: (required)
│   │   ├── description: (required)
│   │   └── paths: (optional — glob patterns for when to auto-load)
│   └── Markdown instructions (required)
└── Bundled Resources (optional)
    ├── scripts/          - Executable code (Python/Bash/etc.)
    ├── references/       - Documentation intended to be loaded into context as needed
    └── assets/           - Files used in output (templates, icons, fonts, etc.)
```

#### SKILL.md (required)

Every SKILL.md consists of:

- **Frontmatter** (YAML): Contains `name` and `description` fields. These are the primary fields that determine when the skill gets triggered. Be clear and comprehensive in describing what the skill is and when it should be used.
- **Body** (Markdown): Instructions and guidance for using the skill. Only loaded AFTER the skill triggers.

#### Bundled Resources (optional)

##### Scripts (`scripts/`)

Executable code (Python/Bash/etc.) for tasks that require deterministic reliability or are repeatedly rewritten.

- **When to include**: When the same code is being rewritten repeatedly or deterministic reliability is needed
- **Example**: `scripts/rotate_pdf.py` for PDF rotation tasks
- **Benefits**: Token efficient, deterministic, may be executed without loading into context

##### References (`references/`)

Documentation and reference material intended to be loaded as needed into context.

- **When to include**: For documentation that Claude should reference while working
- **Examples**: `references/schema.md` for database schemas, `references/api_docs.md` for API specifications
- **Benefits**: Keeps SKILL.md lean, loaded only when Claude determines it's needed
- **Best practice**: If files are large (>10k words), include grep search patterns in SKILL.md
- **Avoid duplication**: Information should live in either SKILL.md or references files, not both.

##### Assets (`assets/`)

Files not intended to be loaded into context, but rather used within the output Claude produces.

- **When to include**: When the skill needs files that will be used in the final output
- **Examples**: `assets/logo.png` for brand assets, `assets/template/` for boilerplate code

### Progressive Disclosure Design Principle

Skills use a three-level loading system to manage context efficiently:

1. **Metadata (name + description)** — Always in context (~100 words)
2. **SKILL.md body** — When skill triggers (<5k words)
3. **Bundled resources** — As needed (unlimited, loaded on demand)

Keep SKILL.md body under 500 lines. Split content into separate files when approaching this limit. Reference them from SKILL.md and describe clearly when to read them.

**Pattern 1: High-level guide with references**

```markdown
# PDF Processing

## Quick start
Extract text with pdfplumber: [code example]

## Advanced features
- **Form filling**: See [FORMS.md](FORMS.md)
- **API reference**: See [REFERENCE.md](REFERENCE.md)
```

**Pattern 2: Domain-specific organization**

```
database-skill/
├── SKILL.md (overview and navigation)
└── references/
    ├── postgres.md
    ├── mysql.md
    └── sqlite.md
```

**Pattern 3: Conditional details**

```markdown
# DOCX Processing
## Creating documents
Use docx-js for new documents. See [DOCX-JS.md](DOCX-JS.md).

**For tracked changes**: See [REDLINING.md](REDLINING.md)
**For OOXML details**: See [OOXML.md](OOXML.md)
```

## Skill Creation Process

1. Understand the skill with concrete examples
2. Plan reusable skill contents (scripts, references, assets)
3. Create the skill directory and SKILL.md
4. Edit the skill (implement resources and write SKILL.md)
5. Validate the skill
6. Iterate based on real usage

### Step 1: Understanding with Concrete Examples

Ask clarifying questions:
- "What functionality should the skill support?"
- "Can you give examples of how this skill would be used?"
- "What would a user say that should trigger this skill?"

### Step 2: Planning Reusable Contents

Analyze each example:
1. Consider how to execute from scratch
2. Identify scripts, references, and assets that would help

Example: For a `pdf-editor` skill handling "rotate this PDF":
- Rotating PDFs requires re-writing the same code → add `scripts/rotate_pdf.py`

Example: For a `database-query` skill handling schema questions:
- Querying requires knowing table schemas → add `references/schema.md`

### Step 3: Create the Skill

Create the skill directory structure manually:

```bash
mkdir -p ~/.claude/skills/<skill-name>
```

Or for project-specific skills:

```bash
mkdir -p .claude/skills/<skill-name>
```

### Step 4: Write SKILL.md

**Writing Guidelines:** Use imperative/infinitive form.

**Frontmatter:**
- `name`: kebab-case, under 64 chars
- `description`: Primary triggering mechanism. Include what the skill does AND specific triggers/contexts for when to use it. This is the ONLY place to describe "when to use" — the body loads after triggering.
- `paths` (optional): Glob patterns for auto-loading when editing matching files.

Example:
```yaml
---
name: sql-formatter
description: Format and beautify SQL queries. Use when working with .sql files, writing SQL in code, or cleaning up generated queries. Handles keyword capitalization, indentation, and common style guides.
paths:
  - "**/*.sql"
  - "src/data/query/**/*.rs"
---
```

**Body:** Write instructions for using the skill and its bundled resources.

### Step 5: Validate

Manual validation checklist:
- [ ] Frontmatter has required `name` and `description`
- [ ] Name is kebab-case, under 64 chars
- [ ] Description clearly states what the skill does and when to use it
- [ ] Body is concise and under 500 lines
- [ ] References are one level deep from SKILL.md
- [ ] No extraneous files (README.md, CHANGELOG.md, etc.)

### Step 6: Iterate

After testing, identify improvements and update.

## Skill Naming

- Lowercase letters, digits, and hyphens only
- Under 64 characters
- Prefer short, verb-led phrases: `sql-formatter`, `pr-prep`, `db-migrate`
- Namespace by tool when it improves clarity: `gh-address-comments`, `linear-address-issue`

## Placement

- **User-level** (available across all projects): `~/.claude/skills/<skill-name>/`
- **Project-level** (specific to one repo): `.claude/skills/<skill-name>/`

## What NOT to Include

- README.md, INSTALLATION_GUIDE.md, QUICK_REFERENCE.md, CHANGELOG.md
- Auxiliary context about the process that went into creating the skill
- User-facing documentation separate from the skill instructions
