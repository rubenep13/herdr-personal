#!/usr/bin/env bash
# Manual QA for the personal fork's Tree sidebar and Git graph, against
# isolated Herdr daemons that this script starts, populates, and stops. It
# never discovers, reads, or stops another daemon, and the GUI gets its own
# HOME and XDG directories, so it is a fresh install that starts with the
# Tree layout.
#
#   scripts/personal-tree-qa.sh [--two-hosts] /usr/bin/herdr [target/debug/herdr-personal-dev]
#
# Agents are declared with `herdr pane report-agent`, so no real agent runs.
# The daemon's config shows the machine and time in `tab_bar_right` and a
# `$model` line under each agent, the way a personal Herdr config might, and
# Tools gets a second tab with no agent in it. API and Tools work in a real
# Git repository with branches, a merge, a tag, a stash, and uncommitted
# changes; Docs is outside any repository.
#
# With --two-hosts, a second daemon appears as the saved SSH host "Build box",
# reached through a fake `ssh` in the sandbox that never connects anywhere: it
# runs the remote command on this machine, in the second daemon's own HOME
# and XDG directories. Local then connects through HERDR_SOCKET_PATH rather
# than --socket, so views that need a scriptable host (orchestrator, Git
# graph) open on both hosts.
#
# A newline on stdin quits the app, stops the daemons, and removes the sandbox.
set -euo pipefail

two_hosts=0
if [ "${1:-}" = --two-hosts ]; then two_hosts=1; shift; fi
herdr=${1:?usage: personal-tree-qa.sh [--two-hosts] /absolute/herdr [client]}
client=${2:-target/debug/herdr-personal-dev}
case "$herdr" in /*) ;; *) echo "herdr must be an absolute path" >&2; exit 2 ;; esac
client=$(realpath "$client")
[ -x "$herdr" ] && [ -x "$client" ] || { echo "missing executable" >&2; exit 2; }

sandbox=$(mktemp -d "${TMPDIR:-/tmp}/hq.XXXXXX")
chmod 700 "$sandbox"
remote="$sandbox/r"
daemon_pid=
remote_pid=
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
remote_env=(
  PATH=/usr/bin:/bin HOME="$remote" XDG_CONFIG_HOME="$remote/config"
  XDG_STATE_HOME="$remote/state" XDG_DATA_HOME="$remote/data"
  XDG_CACHE_HOME="$remote/cache" XDG_RUNTIME_DIR="$remote" TMPDIR="$remote"
  SHELL=/bin/sh TERM=xterm-256color
)
gui=(
  PATH=/usr/bin:/bin HOME="$sandbox/gui" XDG_CONFIG_HOME="$sandbox/gui/config"
  XDG_STATE_HOME="$sandbox/gui/state" XDG_CACHE_HOME="$sandbox/gui/cache"
  XDG_DATA_HOME="$sandbox/gui/data" XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-}"
  WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-}" DISPLAY="${DISPLAY:-}"
)
gui_args=(--socket "$sandbox/a-client.sock")
if [ "$two_hosts" = 1 ]; then
  # Local is the first daemon through HERDR_SOCKET_PATH; saved hosts come from
  # the sandbox's state directory; the login shell puts the fake ssh first.
  gui+=(PATH="$sandbox/bin:/usr/bin:/bin" SHELL=/bin/sh HERDR_SOCKET_PATH="$sandbox/a.sock")
  gui_args=()
fi

cleanup() {
  # Only the exact children this script started.
  if [ -n "$gui_pid" ] && kill -0 "$gui_pid" 2>/dev/null; then
    env -i "${gui[@]}" "$client" app quit >/dev/null 2>&1 || kill "$gui_pid" 2>/dev/null || :
    wait "$gui_pid" 2>/dev/null || :
  fi
  for pid in "$daemon_pid" "$remote_pid"; do
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill -TERM "$pid" 2>/dev/null || :
      wait "$pid" 2>/dev/null || :
    fi
  done
  rm -rf "$sandbox"
}
trap cleanup EXIT

# A repository with two branches merged back, an open topic branch, a tag, a
# remote-tracking branch, a stash, and uncommitted changes.
seed_repo() {
  local dir=$1 at=1792152000
  git_qa() {
    env -i PATH=/usr/bin:/bin HOME="$sandbox" GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
      GIT_AUTHOR_DATE="@$at +0000" GIT_COMMITTER_DATE="@$at +0000" \
      git -C "$dir" -c user.name="Ada Lovelace" -c user.email=ada@example.invalid \
      -c commit.gpgsign=false -c tag.gpgsign=false "$@" >/dev/null ||
      { echo "git $* failed in $dir" >&2; return 1; }
  }
  # Each branch writes its own file, so the merge is clean.
  commit() { at=$((at + 3600)); printf '%s\n' "$1" >> "$dir/$2"; git_qa add -A; git_qa commit -qm "$1"; }
  mkdir -p "$dir"
  git_qa init -q -b main
  commit "feat: start the project" notes.txt
  commit "docs: describe the layout" notes.txt
  git_qa switch -qc feature/lanes
  commit "feat: draw lanes" lanes.txt
  commit "fix: keep lanes stable while paging" lanes.txt
  git_qa switch -q main
  commit "chore: bump dependencies" notes.txt
  at=$((at + 3600)); git_qa merge -q --no-ff -m "Merge branch 'feature/lanes'" feature/lanes
  git_qa tag -a v0.1.0 -m "First release"
  git_qa update-ref refs/remotes/origin/main HEAD
  git_qa switch -qc topic/search
  commit "feat: search commits" search.txt
  git_qa switch -q main
  commit "fix: show detached HEAD" notes.txt
  printf 'stashed\n' >> "$dir/notes.txt"; git_qa stash push -q -m "try another colour"
  printf 'uncommitted\n' >> "$dir/notes.txt"; printf 'new\n' > "$dir/scratch.txt"
}

config='onboarding = false
[terminal]
default_shell = "/bin/sh"
shell_mode = "non_login"
[ui]
tab_bar_right = [{ type = "hostname" }, { type = "datetime" }]
[ui.sidebar.agents]
rows = [["state_icon", "agent"], ["$model"]]'
printf '%s\n' "$config" > "$sandbox/config.toml"
seed_repo "$sandbox/repo"
env -i "${isolated[@]}" "$herdr" server > "$sandbox/server.log" 2>&1 &
daemon_pid=$!

if [ "$two_hosts" = 1 ]; then
  mkdir -p "$remote/config/herdr" "$sandbox/bin" "$sandbox/gui/state/herdr/client"
  printf '%s\n' "$config" > "$remote/config/herdr/config.toml"
  seed_repo "$remote/repo"
  # The fake ssh: `-G` answers from nothing, and the one known target runs its
  # command here, in the second daemon's environment.
  cat > "$sandbox/bin/ssh" <<EOF
#!/bin/sh
resolve=0
while [ \$# -gt 0 ]; do
  case "\$1" in
    --) shift; break ;;
    -G) resolve=1; shift ;;
    -[BbcDEeFIiJLlmOopQRSWw]) shift 2 ;;
    -*) shift ;;
    *) break ;;
  esac
done
target=\$1; shift
if [ \$resolve = 1 ]; then
  printf 'user qa\nhostname %s\nport 22\nuserknownhostsfile $sandbox/known_hosts\n' "\$target"
  exit 0
fi
[ "\$target" = buildbox ] || { echo "qa ssh: no such host: \$target" >&2; exit 255; }
exec env -i ${remote_env[*]} /bin/sh -c "\$*"
EOF
  chmod 700 "$sandbox/bin/ssh"
  printf 'PATH="%s/bin:$PATH"; export PATH\n' "$sandbox" > "$sandbox/gui/.profile"
  # Saved hosts are ignored unless their id is 32 hex digits.
  printf '{"version":1,"ssh":[{"id":"b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0","label":"Build box","target":"buildbox","session":"default","enabled":true}]}\n' \
    > "$sandbox/gui/state/herdr/client/endpoints.json"
  chmod 600 "$sandbox/gui/state/herdr/client/endpoints.json"
  env -i "${remote_env[@]}" "$herdr" server > "$sandbox/remote.log" 2>&1 &
  remote_pid=$!
fi

here() { env -i "${isolated[@]}" "$herdr" "$@" >/dev/null; }
there() { env -i "${remote_env[@]}" "$herdr" "$@" >/dev/null; }
for _ in $(seq 1 100); do [ -S "$sandbox/a-client.sock" ] && break; sleep 0.1; done
[ -S "$sandbox/a-client.sock" ] || { echo "daemon did not start" >&2; exit 1; }

here workspace create --cwd "$sandbox/repo" --label API --no-focus
here workspace create --cwd "$sandbox/repo" --label Tools --no-focus
here workspace create --cwd "$sandbox" --label Docs --no-focus
here pane split w1:p1 --direction right --no-focus
here pane split w1:p1 --direction down --no-focus
here pane report-agent --source qa --agent claude --state blocked --message "needs approval" w1:p1
here pane report-agent --source qa --agent codex --state working w1:p2
here pane report-agent --source qa --agent opencode --state idle w1:p3
here pane report-agent --source qa --agent claude --state working w2:p1
here pane report-metadata w1:p1 --source qa --agent claude --token model=opus-4.7
here pane report-metadata w1:p2 --source qa --agent codex --token model=gpt-5.5
here tab create --workspace w2 --cwd "$sandbox/repo" --label shell

if [ "$two_hosts" = 1 ]; then
  for _ in $(seq 1 100); do there workspace list 2>/dev/null && break; sleep 0.1; done
  there workspace list || { cat "$sandbox/remote.log" >&2; exit 1; }
  there workspace create --cwd "$remote/repo" --label web --no-focus
  there workspace create --cwd "$remote" --label infra --no-focus
  there pane split w1:p1 --direction right --no-focus
  there pane report-agent --source qa --agent claude --state working w1:p1
  there pane report-agent --source qa --agent codex --state blocked --message "needs approval" w1:p2
  there pane report-agent --source qa --agent gemini --state idle w2:p1
  there pane report-metadata w1:p1 --source qa --agent claude --token model=sonnet-5.5
  there tab create --workspace w2 --cwd "$remote" --label logs
fi

env -i "${gui[@]}" "$client" ${gui_args[@]+"${gui_args[@]}"} > "$sandbox/gui.log" 2>&1 &
gui_pid=$!
for _ in $(seq 1 80); do
  env -i "${gui[@]}" "$client" status >/dev/null 2>&1 && break
  kill -0 "$gui_pid" 2>/dev/null || { cat "$sandbox/gui.log" >&2; exit 1; }
  sleep 0.25
done
echo "Local daemon $daemon_pid${remote_pid:+, Build box daemon $remote_pid}, Herdr Personal Dev $gui_pid."
echo "Sandbox: $sandbox"
echo "Try: fold API, drag Tools above API, Move Space Down from the palette,"
echo "search \"codex\", quit and rerun this step to see the order kept."
echo "Options: $client config set sidebar.hierarchy indent|block|guide,"
echo "tabs.position subheader|bottom|herdr, tabs.content name|agent|status."
read -r -p "Press Enter to quit and clean up. " _ || :
