---
name: run-gridix
description: Build, run, and drive the Gridix desktop database management app. Use when asked to start, run, launch, build, screenshot, or interact with Gridix.
paths:
  - src/**/*.rs
  - Cargo.toml
  - .claude/skills/run-gridix/driver.sh
---

Gridix is an egui/eframe desktop GUI app. All paths relative to repo root.

## Prerequisites

```bash
# Ubuntu/Debian
sudo apt-get update && sudo apt-get install -y build-essential pkg-config libgtk-3-dev xdotool imagemagick xvfb

# Arch
sudo pacman -S --needed base-devel pkgconf gtk3 xdotool imagemagick xorg-server-xvfb

# Fedora
sudo dnf install gtk3-devel xdotool ImageMagick xorg-x11-server-Xvfb
```

Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`

Nix (alternative): `nix run github:MCB-SMART-BOY/Gridix` or `nix develop`

## Build

```bash
cargo build --release          # ~90s → target/release/gridix
cargo build                    # ~30s debug
```

## Run (agent path — Rust driver)

```bash
cargo run --bin gridix-driver -- launch &
cargo run --bin gridix-driver -- key Ctrl+N
cargo run --bin gridix-driver -- ss landing
cargo run --bin gridix-driver -- quit
```

The Rust binary replaces `driver.sh`. It manages Xvfb and gridix process lifecycle.
Needs `xdotool` and `imagemagick` (for `import`) on the system.

### Shell driver (fallback)
```bash
source .claude/skills/run-gridix/driver.sh
launch
key Ctrl+N
ss landing
quit
```

### Tmux wrapping

One-liner: source the driver then call `tmux_wrap`.
```bash
source .claude/skills/run-gridix/driver.sh && tmux_wrap
```

Manual setup:
```bash
tmux new-session -d -s gridix -x 200 -y 50
tmux send-keys -t gridix 'source .claude/skills/run-gridix/driver.sh && launch' Enter
timeout 40 bash -c 'until tmux capture-pane -t gridix -p | grep -q "ready"; do sleep 0.3; done'
tmux send-keys -t gridix 'ss landing' Enter
```

### Commands

| cmd | does |
|---|---|
| `launch` | start Xvfb + gridix, wait for window (30s timeout) |
| `key <keys>` | send keystroke (`key Ctrl+N`, `key F1`, `key Escape`) |
| `type <text>` | type text into focused widget |
| `ss [name]` | screenshot → `/tmp/shots/<name>.png` |
| `wait_for <n>` | sleep N seconds |
| `quit` | stop gridix + Xvfb |
| `help` | list commands |

### First-launch flow (onboarding)

The welcome page shows database status cards (SQLite/PostgreSQL/MySQL).
Flow: `Ctrl+N` → choose SQLite → select/create database file → table appears in sidebar → `Ctrl+J` for SQL editor → `Ctrl+Enter` execute → `F1` help.

Learning sample: `F1` → "Learning" tab → ensures SQLite learning DB (8 tables, 100+ rows, e-commerce schema). `F1` auto-creates + connects it.

## Run (human path)

```bash
cargo run --release   # opens window (needs display). Ctrl-C to quit.
```

Useless headless — use driver.

## Test

```bash
cargo test                     # ~620 tests, all pass
cargo test -p gridix --lib     # unit tests only
cargo test --test grid_tests   # grid-specific tests
```

MySQL integration (needs server):
```bash
GRIDIX_IT_MYSQL_HOST=127.0.0.1 GRIDIX_IT_MYSQL_PORT=3306 \
GRIDIX_IT_MYSQL_USER=root GRIDIX_IT_MYSQL_PASSWORD=secret \
GRIDIX_IT_MYSQL_DB=test \
cargo test --test mysql_cancel_integration -- --ignored --nocapture
```

## Gotchas

- **Wayland**: driver sets `WINIT_UNIX_BACKEND=x11` so xdotool can find the window
- **Arch**: Xvfb package is `xorg-server-xvfb`, not `xvfb`
- **Build needs gtk3 dev headers** — `libgtk-3-dev` not just `libgtk-3-0`
- **Keymap warnings on startup** are non-fatal — scope conflict diagnostics, deeper scope wins
- **First build**: ~200 crates, ~2GB in `target/`
