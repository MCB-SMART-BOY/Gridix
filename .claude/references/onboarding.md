# Onboarding Guide

Quick setup for new Gridix developers.

## Prerequisites

```bash
# Ubuntu/Debian
sudo apt-get install -y build-essential pkg-config libgtk-3-dev libxdo-dev

# Arch
sudo pacman -S --needed base-devel pkgconf gtk3 xdotool
```

Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`

## First build

```bash
git clone https://github.com/MCB-SMART-BOY/Gridix.git
cd Gridix
cargo build                           # ~30s debug
cargo test                            # all ~530 tests pass
cargo run --release                   # launch GUI
```

## Project structure

```
src/types.rs            Layer -1: shared types
src/core/               Layer 0:  pure functions, no egui
src/data/               Layer 1:  database operations
src/session/            Layer 2:  async infrastructure, tab management
src/state/              Layer 3:  UI rendering state
src/app/ + src/ui/      Layer 4:  eframe App, rendering, input routing
```

Dependency direction: `types ← core ← data ← session ← state ← ui`

## Read first

1. `~/.codex/references/modern-software-engineering-workflow.md` — default workflow for any software change
2. `~/.codex/references/rust-modern-engineering-playbook.md` — Rust quality gates, refactor, test, async, and CI rules
3. `CLAUDE.md` — complete project map
4. `~/.codex/references/architecture/decisions.md` — why we made key choices
5. `~/.codex/references/core-flows.md` — runtime invariants
6. `~/.codex/references/workflow.md` — Gridix-specific dev workflow overlay
7. `~/.codex/references/workbench-ui-design.md` — target UI shell and migration plan
8. `~/.codex/references/workbench-ui-refactor-spec.md` — workbench config and implementation details
9. `~/.codex/references/project-refactor-execution-plan.md` — phase-by-phase project refactor route

## Common tasks

| Task | Start here |
|------|-----------|
| Plan any software change | `modern-engineering-workflow` skill |
| Define Rust quality gates | `~/.codex/references/rust-modern-engineering-playbook.md` |
| Understand architecture | `CLAUDE.md` → Architecture section |
| Change database code | `src/data/` + `~/.codex/rules/database.md` |
| Change UI | `src/ui/` + `~/.codex/rules/ui-egui.md` |
| Redesign workbench UI | `~/.codex/references/workbench-ui-design.md` |
| Execute project refactor | `~/.codex/references/project-refactor-execution-plan.md` |
| Add a dialog | `~/.codex/references/dialog-audit.md` |
| Add a DB backend | `~/.codex/references/database-backends.md` |
| Change keybindings | `gridix-keybindings` skill |

## Key conventions

- Chinese `//!` module docs, English identifiers
- `use crate::prelude::*;` for common types
- `// =====...=====` section separators
- `thiserror` for error types
- Commit: `type(scope): description` format
