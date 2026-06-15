#!/usr/bin/env bash
# driver.sh — Gridix egui desktop app driver for headless Linux.
# Launches under xvfb, sends keystrokes via xdotool, takes screenshots via import.
# Source this script, then call the functions. Or wrap in tmux for interactive use.
#
# Usage:
#   source driver.sh
#   launch              # → starts gridix under xvfb, waits for window
#   key Ctrl+N          # → send keystroke
#   type "SELECT 1"     # → type text
#   ss my-shot          # → screenshot → /tmp/shots/my-shot.png
#   quit                # → kill the app
#
# Tmux one-liner:
#   tmux_wrap           # → starts tmux session with driver ready

set -uo pipefail

GRIDIX_BIN="${GRIDIX_BIN:-${PWD}/target/release/gridix}"
SHOT_DIR="${SHOT_DIR:-/tmp/shots}"
XVFB_DISPLAY="${XVFB_DISPLAY:-:99}"
export DISPLAY="${XVFB_DISPLAY}"

# For systems where Xvfb is already managed externally, set XVFB_MANAGED=1
XVFB_MANAGED="${XVFB_MANAGED:-0}"
XVFB_PID=""
GRIDIX_PID=""
APP_WID=""

mkdir -p "$SHOT_DIR"

# ── helpers ──────────────────────────────────────────────────────────

_xvfb_up() {
  if [[ "$XVFB_MANAGED" == "1" ]]; then
    echo "[driver] using external Xvfb at $DISPLAY"
    return 0
  fi
  if ! command -v Xvfb &>/dev/null; then
    echo "[driver] ERROR: Xvfb not found."
    echo "[driver] Ubuntu: sudo apt-get install -y xvfb"
    echo "[driver] Arch:   sudo pacman -S xorg-server-xvfb"
    echo "[driver] Fedora: sudo dnf install xorg-x11-server-Xvfb"
    return 1
  fi
  # Clean stale lock
  rm -f "/tmp/.X${XVFB_DISPLAY#:}-lock" "/tmp/.X11-unix/X${XVFB_DISPLAY#:}"
  Xvfb "$XVFB_DISPLAY" -screen 0 1920x1080x24 -ac +extension RANDR &
  XVFB_PID=$!
  sleep 1
  echo "[driver] Xvfb started on $DISPLAY (pid=$XVFB_PID)"
}

_wait_window() {
  local timeout="${1:-30}"
  local elapsed=0
  while [[ $elapsed -lt $timeout ]]; do
    APP_WID=$(xdotool search --onlyvisible --name "Gridix" 2>/dev/null | head -1 || true)
    if [[ -n "$APP_WID" ]]; then
      echo "[driver] window found: $APP_WID (after ${elapsed}s)"
      return 0
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done
  echo "[driver] ERROR: window 'Gridix' not found after ${timeout}s"
  return 1
}

# ── commands ──────────────────────────────────────────────────────────

launch() {
  if [[ -n "$GRIDIX_PID" ]]; then
    echo "[driver] already running (pid=$GRIDIX_PID)"
    return 0
  fi

  if [[ ! -x "$GRIDIX_BIN" ]]; then
    echo "[driver] ERROR: gridix binary not found at '$GRIDIX_BIN'"
    echo "[driver] Build first: cargo build --release"
    return 1
  fi

  _xvfb_up

  # Force X11 backend — needed on Wayland hosts for xdotool compatibility
  export WINIT_UNIX_BACKEND=x11

  "$GRIDIX_BIN" &>/tmp/gridix-driver.log &
  GRIDIX_PID=$!
  echo "[driver] gridix started (pid=$GRIDIX_PID)"

  _wait_window 30 || { quit; return 1; }
  sleep 2  # let first frame fully render
  echo "[driver] ready."
}

key() {
  if [[ -z "$APP_WID" ]]; then echo "[driver] ERROR: launch first"; return 1; fi
  xdotool windowactivate --sync "$APP_WID" 2>/dev/null || true
  xdotool key "$@"
  echo "[driver] key: $*"
}

type() {
  if [[ -z "$APP_WID" ]]; then echo "[driver] ERROR: launch first"; return 1; fi
  xdotool windowactivate --sync "$APP_WID" 2>/dev/null || true
  xdotool type "$@"
  echo "[driver] type: $*"
}

ss() {
  local name="${1:-ss-$(date +%s)}"
  local f="$SHOT_DIR/${name}.png"
  if [[ -z "$APP_WID" ]]; then
    # No app window — screenshot whole screen
    import -window root "$f"
  else
    import -window "$APP_WID" "$f"
  fi
  echo "[driver] screenshot: $f"
}

wait_for() {
  local timeout="${1:-10}"
  sleep "$timeout"
  echo "[driver] waited ${timeout}s"
}

quit() {
  if [[ -n "$GRIDIX_PID" ]]; then
    kill "$GRIDIX_PID" 2>/dev/null || true
    wait "$GRIDIX_PID" 2>/dev/null || true
    GRIDIX_PID=""
    APP_WID=""
    echo "[driver] gridix stopped"
  fi
  if [[ "$XVFB_MANAGED" != "1" && -n "$XVFB_PID" ]]; then
    kill "$XVFB_PID" 2>/dev/null || true
    wait "$XVFB_PID" 2>/dev/null || true
    XVFB_PID=""
    echo "[driver] Xvfb stopped"
  fi
}

help() {
  echo "Gridix driver commands:"
  echo "  launch              — start gridix under xvfb, wait for window"
  echo "  key <keys>          — send keystrokes (e.g. 'key Ctrl+N', 'key F1')"
  echo "  type <text>         — type text string"
  echo "  ss [name]           — screenshot → $SHOT_DIR/<name>.png"
  echo "  wait_for <seconds>  — sleep N seconds"
  echo "  quit                — stop gridix and xvfb"
  echo "  tmux_wrap           — start a fresh tmux session with driver loaded"
  echo ""
  echo "Env vars:"
  echo "  GRIDIX_BIN          — path to gridix binary (default: ./target/release/gridix)"
  echo "  SHOT_DIR            — screenshot directory (default: /tmp/shots)"
  echo "  XVFB_DISPLAY        — X display for Xvfb (default: :99)"
  echo "  XVFB_MANAGED        — set to 1 if Xvfb is already running externally"
}

tmux_wrap() {
  local session="${1:-gridix}"
  local driver_path
  driver_path="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/driver.sh"
  tmux kill-session -t "$session" 2>/dev/null || true
  tmux new-session -d -s "$session" -x 200 -y 50
  tmux send-keys -t "$session" "source '$driver_path' && launch" Enter
  echo "[driver] tmux session '$session' started. Attach: tmux attach -t $session"
  echo "[driver] send commands: tmux send-keys -t $session '<command>' Enter"
  echo "[driver] capture output: tmux capture-pane -t $session -p"
}

# If executed directly (not sourced), show help
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  help
fi
