#!/usr/bin/env bash
# Manual QA for the personal fork's Tree sidebar, against an isolated Herdr
# daemon that this script starts, populates, and stops. It never discovers,
# reads, or stops another daemon, and the GUI gets its own HOME and XDG
# directories, so it is a fresh install that starts with the Tree layout.
#
#   scripts/personal-tree-qa.sh /usr/bin/herdr [target/debug/herdr-personal-dev]
#
# Agents are declared with `herdr pane report-agent`, so no real agent runs.
# Press Enter in this terminal to quit the app, stop the daemon, and remove
# the sandbox.
set -euo pipefail

herdr=${1:?usage: personal-tree-qa.sh /absolute/herdr [client]}
client=${2:-target/debug/herdr-personal-dev}
case "$herdr" in /*) ;; *) echo "herdr must be an absolute path" >&2; exit 2 ;; esac
client=$(realpath "$client")
[ -x "$herdr" ] && [ -x "$client" ] || { echo "missing executable" >&2; exit 2; }

sandbox=$(mktemp -d "${TMPDIR:-/tmp}/hq.XXXXXX")
chmod 700 "$sandbox"
daemon_pid=
gui_pid=

# Arrays rather than functions, so a background `env` execs the program and
# `$!` is its own PID, never a subshell's.
isolated=(
  PATH=/usr/bin:/bin HOME="$sandbox" XDG_CONFIG_HOME="$sandbox/config"
  XDG_STATE_HOME="$sandbox/state" XDG_DATA_HOME="$sandbox/data"
  XDG_CACHE_HOME="$sandbox/cache" XDG_RUNTIME_DIR="$sandbox" TMPDIR="$sandbox"
  HERDR_CONFIG_PATH="$sandbox/config.toml" HERDR_SOCKET_PATH="$sandbox/a.sock"
  HERDR_CLIENT_SOCKET_PATH="$sandbox/a-client.sock" SHELL=/bin/sh TERM=xterm-256color
)
gui=(
  PATH=/usr/bin:/bin HOME="$sandbox/gui" XDG_CONFIG_HOME="$sandbox/gui/config"
  XDG_STATE_HOME="$sandbox/gui/state" XDG_CACHE_HOME="$sandbox/gui/cache"
  XDG_DATA_HOME="$sandbox/gui/data" XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-}"
  WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-}" DISPLAY="${DISPLAY:-}"
)

cleanup() {
  # Only the exact children this script started.
  if [ -n "$gui_pid" ] && kill -0 "$gui_pid" 2>/dev/null; then
    env -i "${gui[@]}" "$client" app quit >/dev/null 2>&1 || kill "$gui_pid" 2>/dev/null || :
    wait "$gui_pid" 2>/dev/null || :
  fi
  if [ -n "$daemon_pid" ] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill -TERM "$daemon_pid" 2>/dev/null || :
    wait "$daemon_pid" 2>/dev/null || :
  fi
  rm -rf "$sandbox"
}
trap cleanup EXIT

printf 'onboarding = false\n[terminal]\ndefault_shell = "/bin/sh"\nshell_mode = "non_login"\n' \
  > "$sandbox/config.toml"
env -i "${isolated[@]}" "$herdr" server > "$sandbox/server.log" 2>&1 &
daemon_pid=$!
for _ in $(seq 1 100); do [ -S "$sandbox/a-client.sock" ] && break; sleep 0.1; done
[ -S "$sandbox/a-client.sock" ] || { echo "daemon did not start" >&2; exit 1; }

cli() { env -i "${isolated[@]}" "$herdr" "$@" >/dev/null; }
for label in API Tools Docs; do cli workspace create --cwd "$sandbox" --label "$label" --no-focus; done
cli pane split w1:p1 --direction right --no-focus
cli pane split w1:p1 --direction down --no-focus
cli pane report-agent --source qa --agent claude --state blocked --message "needs approval" w1:p1
cli pane report-agent --source qa --agent codex --state working w1:p2
cli pane report-agent --source qa --agent opencode --state idle w1:p3
cli pane report-agent --source qa --agent claude --state working w2:p1

env -i "${gui[@]}" "$client" --socket "$sandbox/a-client.sock" > "$sandbox/gui.log" 2>&1 &
gui_pid=$!
for _ in $(seq 1 80); do
  env -i "${gui[@]}" "$client" status >/dev/null 2>&1 && break
  kill -0 "$gui_pid" 2>/dev/null || { cat "$sandbox/gui.log" >&2; exit 1; }
  sleep 0.25
done
echo "Isolated daemon $daemon_pid and Herdr Personal Dev $gui_pid are running."
echo "Sandbox: $sandbox"
echo "Try: fold API, drag Tools above API, Move Space Down from the palette,"
echo "search \"codex\", quit and rerun this step to see the order kept."
read -r -p "Press Enter to quit and clean up. " _ || :
