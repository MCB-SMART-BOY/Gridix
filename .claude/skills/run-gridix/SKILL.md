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
sudo apt-get update && sudo apt-get install -y build-essential pkg-config libgtk-3-dev xvfb

# Arch
sudo pacman -S --needed base-devel pkgconf gtk3 xorg-server-xvfb

# Fedora
sudo dnf install gtk3-devel xorg-x11-server-Xvfb
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
| `launch` | start Xvfb + Gridix and wait for its window |
| `key <keys>` | send a keystroke, e.g. `key Ctrl+N`, `key F1`, `key Escape` |
| `ss [name]` | capture `/tmp/shots/<name>.png` (or `GRIDIX_SHOT_DIR`) |
| `quit` | stop Gridix and Xvfb |
| `help` | show the command list |

The driver does **not** type text, wait for widgets, click controls, or operate native file dialogs. Use it for launch/key/screenshot/quit smoke paths only.

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
