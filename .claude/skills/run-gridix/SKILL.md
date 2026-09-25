---
name: run-gridix
description: Build, run, and drive the Gridix desktop database management app. Use when asked to start, run, launch, build, screenshot, or interact with Gridix.
paths:
  - src/**/*.rs
  - Cargo.toml
---

Gridix is an egui/eframe desktop GUI app. All paths relative to repo root.

## Prerequisites

```bash
# Ubuntu/Debian
sudo apt-get update && sudo apt-get install -y build-essential pkg-config libgtk-3-dev imagemagick xauth xdotool xvfb

# Arch
sudo pacman -S --needed base-devel pkgconf gtk3 imagemagick xdotool xorg-server-xvfb xorg-xauth

# Fedora
sudo dnf install gtk3-devel ImageMagick xdotool xorg-x11-server-Xvfb xorg-x11-xauth
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
cargo run --bin gridix-driver -- launch
cargo run --bin gridix-driver -- key Ctrl+N
cargo run --bin gridix-driver -- ss landing
cargo run --bin gridix-driver -- quit
```
### Commands

| cmd | does |
|---|---|
| `launch [--detach]` | start a private-cookie Xvfb + Gridix session and wait for its window; `--detach` returns while the tracked session stays alive |
| `key <keys>` | send a keystroke, e.g. `key Ctrl+N`, `key F1`, `key Escape` |
| `type <text>` | type text into the active Gridix window |
| `move <x> <y>` | move the pointer relative to the Gridix window |
| `click <x> <y> [button]` | click at window-relative coordinates |
| `wait window [timeout]` / `wait-window [timeout]` | wait for the Gridix window, bounded to 300 seconds |
| `wait file <path> [timeout]` / `wait-file <path> [timeout]` | wait for a non-empty file, bounded to 300 seconds |
| `ss [name]` | capture `/tmp/shots/<name>.png` (or `GRIDIX_SHOT_DIR`) |
| `assert-file <path> [bytes]` | require a regular, non-empty artifact |
| `assert-export <format> <path> [expected...]` | validate CSV/JSON/SQL and expected text |
| `assert-reopened <db> <table> <column> <value|NULL>` | verify a persisted SQLite value read-only |
| `quit` | stop only the tracked Gridix/Xvfb PIDs and remove the private Xauthority |
| `help` | show the command list |

The driver does **not** operate native file dialogs or decide semantic widget state. Use
`launch --detach` for non-interactive smoke orchestration and call `quit` after the screenshot;
if regular `launch` receives stdin EOF, it also leaves the tracked session alive. Self-managed
Xvfb uses a per-run Xauthority cookie in a private 0700 temporary directory; `GRIDIX_DISPLAY`
selects the display used by Xvfb and all driver actions. Set `XVFB_MANAGED=1` only when that
display is managed outside the driver.

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
cargo test --workspace --all-features
cargo test --test grid_tests
```

Backend integration uses complete per-backend connection URLs:

```bash
GRIDIX_TEST_MYSQL_URL='mysql://user:password@127.0.0.1:3306/database' \
cargo test --test mysql_cancel_integration -- --nocapture --test-threads=1
```

The PostgreSQL equivalent uses `GRIDIX_TEST_PG_URL`. Missing URLs may locally skip integration behavior; CI release-acceptance workflows preflight URLs and do not accept such skips.

## Gotchas

- **Wayland**: driver sets `WINIT_UNIX_BACKEND=x11` so xdotool can find the window
- **Arch**: Xvfb package is `xorg-server-xvfb`, not `xvfb`
- **Build needs gtk3 dev headers** — `libgtk-3-dev` not just `libgtk-3-0`
- **Keymap warnings on startup** are non-fatal — scope conflict diagnostics, deeper scope wins
- **First build**: ~200 crates, ~2GB in `target/`
- **Window activation**: the driver's own Xvfb has no window manager, so `windowactivate`
  fails there and `key`/`type`/`move`/`click` fall back to `xdotool windowfocus`
  (`XSetInputFocus`). That fallback line goes to stderr; a successful `key:`/`typed …`
  line on stdout means the request was accepted, not that the window holds focus.
- **Typing**: `type` targets the focused window (no `xdotool --window`), because synthetic
  events are ignored by egui/winit. Send `Ctrl+P` (or click) first when the target widget
  may not have focus — text typed in Helix-normal mode is discarded.
