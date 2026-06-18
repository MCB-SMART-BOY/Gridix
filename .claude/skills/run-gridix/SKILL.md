---
name: gridix-run
description: Build, run, and drive the Gridix desktop database management app. Use only in the Gridix repository when asked to start, run, launch, build, screenshot, or interact with Gridix.
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
source "$HOME/.claude/skills/run-gridix/driver.sh"
launch
key Ctrl+N
ss landing
quit
```

### Tmux wrapping

One-liner: source the driver then call `tmux_wrap`.
```bash
source "$HOME/.claude/skills/run-gridix/driver.sh" && tmux_wrap
```

Manual setup:
```bash
tmux new-session -d -s gridix -x 200 -y 50
tmux send-keys -t gridix 'source "$HOME/.claude/skills/run-gridix/driver.sh" && launch' Enter
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

Workbench shell note: current builds include the compatibility shell, StatusBar, global TopBar, 4月式 stable PrimarySidebar visibility, BottomPanel result/message routing, EditorArea document/view dock tabs, RightInspector detail tabs, Dockable Workbench v2 surface descriptors, `WorkbenchFocus::Surface`, `DockTab::Surface`, runtime startup on `default_surface_layout()` with Results center / SQL editor bottom / ER right and named split ratios locked to the user-approved 2026-06-19 screenshot (`280px` fixed PrimarySidebar, query/ER `0.73/0.27`, results/editor `0.69/0.31`), `ensure_surface_tab()` for explicit surface reveal, reveal/open action wiring into the surface dock, the unified surface renderer bridge, the shared surface action/header foundation, fixed fallback de-duplication for docked-equivalent PrimarySidebar/BottomPanel/RightInspector surfaces, and real Explorer/Filters/Objects surface rendering through the Sidebar adapter. The duplicate left ActivityBar/SurfaceRail is not rendered in the default runtime layout. Successful queries reveal Results; failed queries reveal Messages; schema/ER inspect paths reveal RightInspector tabs; SQL editing happens in bottom `SqlDocument` tabs. Default Explorer/Filters/Objects content belongs to the fixed PrimarySidebar; explicitly docked navigation surfaces remain possible but must not be created/destroyed by the simple sidebar expand/collapse button.

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
