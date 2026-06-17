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

1. `CLAUDE.md` — complete project map
2. `.claude/references/architecture/decisions.md` — why we made key choices
3. `.claude/references/core-flows.md` — runtime invariants
4. `.claude/references/workflow.md` — dev workflow

## Common tasks

| Task | Start here |
|------|-----------|
| Understand architecture | `CLAUDE.md` → Architecture section |
| Change database code | `src/data/` + `.claude/rules/database.md` |
| Change UI | `src/ui/` + `.claude/rules/ui-egui.md` |
| Add a dialog | `.claude/references/dialog-audit.md` |
| Add a DB backend | `.claude/references/database-backends.md` |
| Change keybindings | `/keybindings` skill |

## Key conventions

- Chinese `//!` module docs, English identifiers
- `use crate::prelude::*;` for common types
- `// =====...=====` section separators
- `thiserror` for error types
- Commit: `type(scope): description` format
