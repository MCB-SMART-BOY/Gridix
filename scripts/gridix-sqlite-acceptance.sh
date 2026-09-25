#!/usr/bin/env bash
# Run the manual SQLite GUI acceptance journey with isolated Xvfb state.
# Native file dialogs and semantic widget operations remain manual.
set -Eeuo pipefail
umask 077

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
readonly START_TIMEOUT_SECONDS=60
readonly POLL_SECONDS=1
readonly SCREENSHOTS=(initial-result.png saved-result.png reopened-result.png)
readonly EXPORTS=(acceptance.csv acceptance.json acceptance.sql)

launch_pid=""
fifo_path=""
driver_fd_open=0

fail() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: scripts/gridix-sqlite-acceptance.sh [SHA] [SQLITE_DB]

Starts an isolated Xvfb/Gridix session (unless XVFB_MANAGED=1 is set), writes
a runbook into the release artifact directory, and waits for a human operator
to complete the SQLite journey. The script then validates the screenshots,
database value, and CSV/JSON/SQL exports without automating native file dialogs.

Environment overrides:
  GRIDIX_ACCEPTANCE_DIR   complete artifact directory override
  GRIDIX_ACCEPTANCE_ROOT  artifact root (default: /tmp/gridix-release-acceptance)
  GRIDIX_ACCEPTANCE_SHA   SHA when the first argument is omitted
  GRIDIX_ACCEPTANCE_DB    SQLite database path when the second argument is omitted
  GRIDIX_DRIVER            gridix-driver binary path
  GRIDIX_BIN               gridix binary path
  GRIDIX_DISPLAY           X11 display (default: :99, or DISPLAY when XVFB_MANAGED=1)
  XVFB_MANAGED=1          use an externally managed X11 display
EOF
}

absolute_path() {
    case "$1" in
        /*) printf '%s\n' "$1" ;;
        *) printf '%s/%s\n' "$PWD" "$1" ;;
    esac
}

has_state_file() {
    [[ -s "$state_file" ]]
}

write_operator_files() {
    {
        printf 'export GRIDIX_DRIVER=%q\n' "$driver"
        printf 'export GRIDIX_BIN=%q\n' "$gridix_bin"
        printf 'export GRIDIX_DISPLAY=%q\n' "$display"
        printf 'export XVFB_MANAGED=%q\n' "${XVFB_MANAGED:-0}"
        printf 'export GRIDIX_SHOT_DIR=%q\n' "$artifact_dir"
    } > "$env_file"
    chmod 600 "$env_file"

    {
        printf '# SQLite GUI acceptance commands\n'
        printf '# Native file dialogs and semantic widget actions are manual.\n'
        printf 'source %q\n\n' "$env_file"
        printf '# Capture after the initial query result is visible.\n'
        printf '"$GRIDIX_DRIVER" ss initial-result\n'
        printf '"$GRIDIX_DRIVER" assert-file %q\n' "$artifact_dir/initial-result.png"
        printf '# After editing and saving the Grid cell, capture the saved result.\n'
        printf '"$GRIDIX_DRIVER" ss saved-result\n'
        printf '"$GRIDIX_DRIVER" assert-file %q\n' "$artifact_dir/saved-result.png"
        printf '# After closing/reopening the database and confirming persistence.\n'
        printf '"$GRIDIX_DRIVER" ss reopened-result\n'
        printf '"$GRIDIX_DRIVER" assert-file %q\n' "$artifact_dir/reopened-result.png"
        printf '# Export files into this artifact directory, then return to the runner.\n'
        printf '"$GRIDIX_DRIVER" assert-reopened %q items name after\n' "$db_path"
        printf '"$GRIDIX_DRIVER" assert-export csv %q after\n' "$artifact_dir/acceptance.csv"
        printf '"$GRIDIX_DRIVER" assert-export json %q %q\n' "$artifact_dir/acceptance.json" '"name":"after"'
        printf '"$GRIDIX_DRIVER" assert-export sql %q %q NULL\n' "$artifact_dir/acceptance.sql" "'after'"
    } > "$commands_file"
    chmod 600 "$commands_file"
}

cleanup() {
    local exit_status=$?
    trap - EXIT
    if [[ -n "$launch_pid" ]] && kill -0 "$launch_pid" 2>/dev/null; then
        "$driver" quit >> "$driver_log" 2>&1 || true
        if (( driver_fd_open )); then
            exec 9>&-
            driver_fd_open=0
        fi
        wait "$launch_pid" || true
    elif (( driver_fd_open )); then
        exec 9>&-
        driver_fd_open=0
    fi
    if [[ -n "$fifo_path" ]]; then
        rm -f "$fifo_path"
    fi
    exit "$exit_status"
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

sha="${1:-${GRIDIX_ACCEPTANCE_SHA:-}}"
if [[ -z "$sha" ]] && command -v git >/dev/null 2>&1; then
    sha="$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || true)"
fi
[[ -n "$sha" ]] || fail 'provide SHA or set GRIDIX_ACCEPTANCE_SHA'
[[ "$sha" =~ ^[A-Za-z0-9._-]+$ ]] || fail 'SHA must be a simple path component'

artifact_root="${GRIDIX_ACCEPTANCE_ROOT:-/tmp/gridix-release-acceptance}"
artifact_dir="${GRIDIX_ACCEPTANCE_DIR:-$artifact_root/$sha}"
artifact_dir="$(absolute_path "$artifact_dir")"
mkdir -m 700 -p "$artifact_dir"
chmod 700 "$artifact_dir"

for artifact in "${SCREENSHOTS[@]}" "${EXPORTS[@]}"; do
    [[ ! -e "$artifact_dir/$artifact" ]] || fail "artifact already exists: $artifact_dir/$artifact"
done

driver="${GRIDIX_DRIVER:-$REPO_ROOT/target/release/gridix-driver}"
gridix_bin="${GRIDIX_BIN:-$REPO_ROOT/target/release/gridix}"
[[ -x "$driver" ]] || fail "gridix-driver not found or not executable: $driver (build it first)"
[[ -x "$gridix_bin" ]] || fail "gridix binary not found or not executable: $gridix_bin (build it first)"

db_path="${2:-${GRIDIX_ACCEPTANCE_DB:-$artifact_dir/acceptance.db}}"
db_path="$(absolute_path "$db_path")"
if [[ -n "${GRIDIX_DISPLAY:-}" ]]; then
    display="$GRIDIX_DISPLAY"
elif [[ "${XVFB_MANAGED:-0}" == "1" && -n "${DISPLAY:-}" ]]; then
    display="$DISPLAY"
else
    display=":99"
fi
state_file="$artifact_dir/driver-state.json"
env_file="$artifact_dir/driver.env"
commands_file="$artifact_dir/operator-commands.txt"
driver_log="$artifact_dir/driver.log"
fifo_path="$artifact_dir/.driver-stdin.$$"

export GRIDIX_BIN="$gridix_bin"
export GRIDIX_DISPLAY="$display"
export XVFB_MANAGED="${XVFB_MANAGED:-0}"
export GRIDIX_DRIVER_STATE="$state_file"
export GRIDIX_SHOT_DIR="$artifact_dir"

write_operator_files
trap cleanup EXIT
mkfifo "$fifo_path"
exec 9<> "$fifo_path"
driver_fd_open=1

printf 'Starting Gridix acceptance session on %s...\n' "$display"
"$driver" launch <&9 > "$driver_log" 2>&1 &
launch_pid=$!

deadline=$((SECONDS + START_TIMEOUT_SECONDS))
until has_state_file; do
    if ! kill -0 "$launch_pid" 2>/dev/null; then
        printf '%s\n' "$(cat "$driver_log" 2>/dev/null || true)" >&2
        fail 'gridix-driver exited before the X11 window became ready'
    fi
    if (( SECONDS >= deadline )); then
        printf '%s\n' "$(cat "$driver_log" 2>/dev/null || true)" >&2
        fail "timed out waiting for driver state after ${START_TIMEOUT_SECONDS}s"
    fi
    sleep "$POLL_SECONDS"
done

printf 'Session ready. Run the commands in:\n  %s\n' "$commands_file"
printf 'Complete the manual journey in another terminal, then press Enter here.\n'
[[ -t 0 ]] || fail 'manual acceptance requires an interactive terminal'
read -r -p 'Press Enter after all screenshots and exports are ready: ' _

for screenshot in "${SCREENSHOTS[@]}"; do
    "$driver" assert-file "$artifact_dir/$screenshot" >> "$driver_log" 2>&1 \
        || fail "missing or empty screenshot: $artifact_dir/$screenshot"
done
"$driver" assert-reopened "$db_path" items name after >> "$driver_log" 2>&1 \
    || fail "SQLite persistence assertion failed: $db_path"
"$driver" assert-export csv "$artifact_dir/acceptance.csv" after >> "$driver_log" 2>&1 \
    || fail 'CSV export assertion failed'
"$driver" assert-export json "$artifact_dir/acceptance.json" '"name":"after"' >> "$driver_log" 2>&1 \
    || fail 'JSON export assertion failed; use compact JSON output'
"$driver" assert-export sql "$artifact_dir/acceptance.sql" "'after'" NULL >> "$driver_log" 2>&1 \
    || fail 'SQL export assertion failed'

manifest="$artifact_dir/manifest.txt"
{
    printf 'sha=%s\n' "$sha"
    printf 'display=%s\n' "$display"
    printf 'sqlite_db=%s\n' "$db_path"
    printf 'driver=%s\n' "$driver"
    printf 'gridix=%s\n' "$gridix_bin"
    for artifact in "${SCREENSHOTS[@]}" "${EXPORTS[@]}"; do
        printf 'artifact=%s\n' "$artifact_dir/$artifact"
    done
} > "$manifest"

printf 'SQLite GUI acceptance artifacts validated under:\n  %s\n' "$artifact_dir"
