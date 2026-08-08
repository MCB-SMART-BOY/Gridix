---
name: troubleshoot
description: Diagnose and fix Gridix build, launch, and test failures. Use when builds fail, the app won't start, tests break, or dependencies are missing.
paths:
  - Cargo.toml
  - Cargo.lock
---

# Troubleshooting

## Build failures

### Missing gtk3 headers
```
pkg-config: can't find gtk+-3.0
```
→ `sudo apt-get install -y libgtk-3-dev` (Ubuntu) / `sudo pacman -S gtk3` (Arch) / `sudo dnf install gtk3-devel` (Fedora)

### Missing xdo headers
```
pkg-config: can't find xdo
```
→ `sudo apt-get install -y libxdo-dev` (Ubuntu) / `sudo pacman -S xdotool` (Arch)

### Rust not installed
→ `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source "$HOME/.cargo/env"`

## App won't start

### No X server (headless)
→ Use driver: `cargo run --bin gridix-driver -- launch`
Or manually: `Xvfb :99 -screen 0 1920x1080x24 &; export DISPLAY=:99; cargo run --release`

### Stale Xvfb lock
→ `rm -f /tmp/.X99-lock /tmp/.X11-unix/X99 && pkill Xvfb`

### xdotool can't find window (Wayland)
→ `export WINIT_UNIX_BACKEND=x11` before launching

### Missing runtime libs
→ `sudo apt-get install -y libgtk-3-0 xdotool imagemagick` (Ubuntu)

## Test failures

### PostgreSQL/MySQL integration is not exercising a backend

Local integration tests use complete URLs:

```bash
GRIDIX_TEST_MYSQL_URL='mysql://user:password@127.0.0.1:3306/database' \
cargo test --test mysql_cancel_integration -- --nocapture --test-threads=1

GRIDIX_TEST_PG_URL='postgres://user:password@127.0.0.1:5432/database' \
cargo test --test postgres_cancel_integration -- --nocapture --test-threads=1
```

An absent URL may make the test return locally; it is not a passing backend check. CI preflights these variables and supplies service containers. For cancellation failures, verify the observer can see the marker and that MySQL has the permissions needed for `KILL QUERY`; do not diagnose them as ignored tests.

### input_router tests fail after keybinding changes
Check: did you change a key another test expects? Is the scope path correct? Is `TextEntryGuard` blocking the new shortcut?

### Tooling missing
→ `sudo apt-get install -y imagemagick` (screenshots) / `sudo pacman -S imagemagick`

## Config corruption

Backup then remove:
```bash
cp -r ~/.config/gridix ~/.config/gridix.bak
rm ~/.config/gridix/config.toml ~/.config/gridix/keymap.toml
```
Relaunch → fresh config created.

## Keybindings in config.toml ignored

The `keybindings` field in `config.toml` is legacy read-only. Active keymap: `~/.config/gridix/keymap.toml`. See `/keybindings` skill.

## PostgreSQL/MySQL not detected

Check service installed + running. Ports: 5432 (PG), 3306 (MySQL). Use welcome page "Recheck" button or `Ctrl+N` to create connection manually.
