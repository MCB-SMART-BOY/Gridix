# Gridix Harness Engineering Knowledge Base

This directory is the complete AI-assisted software engineering harness for the Gridix project. It provides Claude Code with domain knowledge, workflows, rules, skills, and reference material needed to work effectively on this codebase.

Primary source of truth: `~/.codex/` (actively maintained by Codex sessions). This `.claude/` mirror is kept in sync with verbatim copies of shared content, plus Claude Code-specific enhancements (`paths:` frontmatter, `sync-claude.md`, adapted system skills).

## Directory Map

```
.claude/
├── README.md                 ← you are here
├── settings.json             ← Claude Code permissions (allow/deny rules)
├── .gitignore                ← excludes settings.json from git
│
├── references/               ← Engineering ledgers & design contracts (21 files)
│   ├── architecture/         ← ADR: architecture/decisions.md
│   ├── *-ledger              ← bug-ledger.md, tech-debt.md
│   ├── *-spec                ← workbench-ui-refactor-spec.md
│   ├── *-design              ← workbench-ui-design.md, dockable-workbench-v2.md
│   ├── *-contracts           ← er-contracts.md, dialog-audit.md, core-flows.md
│   ├── *-guide               ← testing-guide.md, onboarding.md, workflow.md
│   ├── *-plan                ← project-refactor-execution-plan.md
│   ├── *-playbook            ← rust-modern-engineering-playbook.md
│   └── roadmap.md            ← planned features & milestones
│
├── rules/                    ← Domain rules — DO/DON'T/VERIFY patterns (9 files)
│   ├── database.md           ← data layer: match db_type, no traits
│   ├── session.md            ← session layer: async, needs_repaint
│   ├── ui-egui.md            ← UI layer: workbench shell, dialogs
│   ├── testing.md            ← test patterns, Rust gates, layer-specific
│   ├── security.md           ← security rules
│   ├── sync-codex.md         ← when to update ~/.codex/ after code changes
│   ├── sync-claude.md        ← when to update .claude/ after code changes
│   ├── default.rules         ← Codex-format bash permission prefix rules
│   └── default-rules.md      ← explains dual permission system (Codex + Claude Code)
│
├── workflow/                 ← 8-stage lifecycle (7 files)
│   ├── README.md             ← overview, quality gates, project overlays
│   ├── 01-plan.md            ← Stages 0+1: Intake → Plan
│   ├── 02-design.md          ← Stage 2: Design approach
│   ├── 03-implement.md       ← Stages 3+4: Safety Net + Implementation
│   ├── 04-review.md          ← Stage 5: Self-review
│   ├── 05-test.md            ← Stage 6: Verification
│   └── 06-deliver.md         ← Stage 7: Summarize, document, commit, update harness
│
├── skills/                   ← Executable workflows (7 user + 5 system)
│   ├── code-review/          ← Gridix code review checklist
│   ├── keybindings/          ← Keyboard shortcut management
│   ├── pr-prep/              ← Pre-PR quality gates
│   ├── release/              ← Version bump → publish
│   ├── run-gridix/           ← Build, launch, screenshot
│   ├── troubleshoot/         ← Fix build/startup/test errors
│   ├── modern-engineering-workflow/ ← Cross-project engineering process
│   └── .system/              ← System skills (some Codex-only)
│       ├── skill-creator/    ← Claude Code skill creation guide
│       ├── openai-docs/      ← OpenAI API documentation (Claude Code adapted)
│       ├── imagegen/         ← Codex-only: image generation
│       ├── plugin-creator/   ← Codex-only: plugin scaffolding
│       └── skill-installer/  ← Codex-only: skill marketplace installer
│
├── templates/                ← Standard formats (3 files)
│   ├── commit-message.md     ← Conventional commit template
│   ├── pr-description.md     ← PR body template
│   └── feature-request.md    ← Feature specification template
│
└── memory/                   ← Persistent project memory (3 files)
    ├── MEMORY.md             ← Index of all memories
    ├── project-context.md    ← Architecture state, constraints, tech debt
    └── SETUP.md              ← Memory system usage guide
```

## Task Navigation — Where to Start

| You want to… | Start here |
|---|---|
| Build, launch, screenshot | `skills/run-gridix/SKILL.md` |
| Change a keyboard shortcut | `skills/keybindings/SKILL.md` |
| Prepare a PR / run checks | `skills/pr-prep/SKILL.md` |
| Publish a release | `skills/release/SKILL.md` |
| Fix a build/startup error | `skills/troubleshoot/SKILL.md` |
| Review code | `skills/code-review/SKILL.md` |
| Understand architecture | `CLAUDE.md` (project root) + `references/architecture/decisions.md` |
| Change a dialog | `references/dialog-audit.md` + `rules/ui-egui.md` |
| Change the ER diagram | `references/er-contracts.md` |
| Change database code | `rules/database.md` |
| Change session/connection code | `rules/session.md` |
| Change UI code | `rules/ui-egui.md` |
| Write/modify tests | `rules/testing.md` |
| Understand invariants | `references/core-flows.md` |
| Check known bugs | `references/bug-ledger.md` |
| Known tech debt / issues | `references/tech-debt.md` |
| Future improvements | `references/roadmap.md` |
| New developer onboarding | `references/onboarding.md` |
| Workbench UI design | `references/workbench-ui-design.md` → `references/dockable-workbench-v2.md` |
| Workbench UI implementation | `references/workbench-ui-refactor-spec.md` |
| Project-wide refactor | `references/project-refactor-execution-plan.md` |
| General engineering process | `workflow/README.md` → `skills/modern-engineering-workflow/SKILL.md` |
| Rust-specific quality | `references/rust-modern-engineering-playbook.md` |
| Understand development workflow | `references/workflow.md` |
| Add a DB backend | `references/database-backends.md` |
| Query execution & isolation | `references/query-execution.md` |
| Grid save isolation | `references/grid-save-isolation.md` |
| Visual system (design tokens) | `references/gridix-ui-visual-system-v2.md` |
| Testing patterns & guide | `references/testing-guide.md` |
| Historical egui_dock plan | `references/egui-dock-plan.md` |
| After making code changes | `rules/sync-claude.md` (`.claude/`) + `rules/sync-codex.md` (`~/.codex/`) |

## Architecture

**6-layer unidirectional dependency:**
```
src/types.rs    (Layer -1) ← shared types
src/core/       (Layer 0)  ← pure functions, no side effects
src/data/       (Layer 1)  ← database operations
src/session/    (Layer 2)  ← connection lifecycle, async dispatch
src/state/      (Layer 3)  ← UI rendering state
src/app/ + ui/  (Layer 4)  ← eframe App, rendering, input routing
```

**Key constraints:**
- NO trait objects for DB backends — use `match db_type`
- Respect layer dependency direction
- `needs_repaint` pattern for async → UI
- Config save uses `save_config_debounced()` (5s throttle)
- Workbench UI: fixed regions are compatibility adapters; target is Dockable Workbench v2 peer surfaces

## Maintenance

1. **After code changes**: update relevant files per `rules/sync-claude.md`
2. **Sync from Codex**: `rsync -av ~/.codex/{references,rules,templates,workflow,skills,memory}/ .claude/{references,rules,templates,workflow,skills,memory}/`
3. **Don't let drift**: the knowledge base IS part of the project — stale docs are worse than no docs
