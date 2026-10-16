//! Usage text for every control command.
use crate::cli::control::FAMILIES;

const COMMON: &str = "\
Every command accepts --json, printing one JSON document, and --window ID,
acting in that window (IDs come from `window list`). By default a command
acts in the window showing the caller's workspace (HERDR_WORKSPACE_ID), and
exits 5 when no window shows it; outside Herdr, in the frontmost window.
Exit status: 0 done, 1 refused, 2 invalid arguments, 3 Herdr UI not running,
4 nothing yet (no notes, no event), 5 not found, 6 busy (a dialog is open),
7 timed out, 8 unsupported on this platform or by this app version.";

const BROWSER: &str = "\
herdr-personal-dev browser open URL|FILE [--workspace ID] [--no-focus] [--beside|--tab]
    Show a web address, or a local HTML file, in a browser tab of the running
    Herdr UI. The tab joins the caller's own workspace (HERDR_WORKSPACE_ID),
    or the frontmost one outside Herdr. Showing the same page again from the
    same pane reuses and reloads its tab. Bare hosts such as localhost:3000
    are accepted. A file is served with the other files in its folder.
    A file opened from a Herdr pane goes in a split next to that terminal,
    which stays in view and keeps the keyboard; a web address takes a tab of
    the group in use. --beside and --tab choose instead.
herdr-personal-dev browser reload
    Reload the pages this pane opened, for example after editing the file.
herdr-personal-dev browser feedback [--wait SECONDS]
    Print the notes the user sent about this pane's pages and have not been
    delivered yet. With --wait, wait up to SECONDS (at most 600) for them.
    Notes are otherwise typed into this pane as a prompt once it is idle.
herdr-personal-dev browser list
    List page tabs with their IDs.
herdr-personal-dev browser close|back|forward|text TAB
    Close a page tab, go back or forward in it, or print its visible text.
herdr-personal-dev browser navigate TAB URL
    Load another web address in a page tab.
herdr-personal-dev browser screenshot TAB PATH
    Save a PNG of the page (macOS).
herdr-personal-dev browser skill
    Print the agent skill that explains these commands.
herdr-personal-dev browser example
    Print an example explainer page built on the kit Herdr UI serves to local
    pages at /.herdr/kit.css and /.herdr/kit.js.
Exit status: 0 done, 1 refused, 2 invalid arguments, 3 Herdr UI not running,
4 no notes (feedback), 5 no such tab or workspace, 6 busy, 7 timed out,
8 unsupported on this platform.";

fn family(name: &str) -> &'static str {
    match name {
        "status" => {
            "herdr-personal-dev status\n    Whether Herdr UI runs, its version, protocol, windows, and hosts.\n    Exits 3 when it does not run."
        }
        "skill" => "herdr-personal-dev skill\n    Print the agent skill describing these commands.",
        "app" => {
            "herdr-personal-dev app focus|hide|quit\n    Bring the app forward, hide it, or quit it. Quitting leaves Herdr\n    and its terminals running."
        }
        "window" => {
            "herdr-personal-dev window [list]\n    The app's windows, with IDs for --window.\nherdr-personal-dev window new [--session NAME]\n    Open another window: of the caller's own Herdr session, of a named\n    local session, or outside Herdr of the frontmost window's host.\nherdr-personal-dev window focus ID\nherdr-personal-dev window fullscreen [on|off|toggle]\nherdr-personal-dev window size [WIDTHxHEIGHT]\n    Print the window's size, or resize it. Windows cannot be moved.\nherdr-personal-dev window detach|reconnect\n    Detach from, or reconnect to, the window's selected host."
        }
        "ui" => {
            "herdr-personal-dev ui sidebar [on|off|toggle]\nherdr-personal-dev ui statusbar [on|off|toggle]\nherdr-personal-dev ui font-size +|-|reset|POINTS\n    For this window and session; settings are changed with `config`."
        }
        "tab" => {
            "herdr-personal-dev tab [list]\n    The app's own tabs: pages, code, reviews, the orchestrator, Devices.\n    Terminal tabs belong to Herdr: use `herdr tab`.\nherdr-personal-dev tab focus|close ID\n    Show a tab, switching to its workspace, or close it."
        }
        "code" => {
            "herdr-personal-dev code open FILE[:LINE]\n    Show a local file read-only in a code tab, at LINE.\nherdr-personal-dev code edit FILE[:LINE]\n    Open it in your terminal editor ($VISUAL, $EDITOR) in a new pane."
        }
        "devices" => {
            "herdr-personal-dev devices [open]\n    Open the Devices overview tab in the caller's workspace."
        }
        "orchestrator" => {
            "herdr-personal-dev orchestrator [open] [--repo PATH]\n    Open the orchestrator of the caller's checkout, or of PATH.\nherdr-personal-dev orchestrator items|issues|prs [--repo PATH]\n    List what it last synced, from its local database (no network).\n    Runs here; Herdr UI need not run. Exits 4 before its first sync."
        }
        "review" => {
            "herdr-personal-dev review [open]\n    Open the review of the caller's workspace's uncommitted changes."
        }
        "notes" => {
            "herdr-personal-dev notes [list]\n    Every worktree note, most recently edited first.\nherdr-personal-dev notes get|clear\nherdr-personal-dev notes set TEXT...\n    The note on the caller's checkout (its repository and branch), or the\n    focused workspace's outside a Herdr pane. Notes are one line of at most\n    500 characters. `list` and `get` exit 4 when there is none."
        }
        "feedback" => {
            "herdr-personal-dev feedback [--wait SECONDS]\n    Notes the user sent this pane: page, review, and terminal notes that\n    were not typed into it. Exits 4 when there are none."
        }
        "checkpoint" => {
            "herdr-personal-dev checkpoint [list]\n    The current directory's checkout's checkpoints, newest first.\nherdr-personal-dev checkpoint create [LABEL...]\n    Take one now.\nherdr-personal-dev checkpoint diff CHECKPOINT [CHECKPOINT]\n    What changed between two, or since one in the working tree (untracked\n    files included).\n    These run here, on the current branch; Herdr UI need not run.\nherdr-personal-dev checkpoint restore [CHECKPOINT]\n    Open the caller's checkout's checkpoints dialog in Herdr UI with\n    CHECKPOINT chosen; the user confirms the restore there."
        }
        "scripts" => {
            "herdr-personal-dev scripts [list]\n    The current directory's repository's worktree scripts\n    (.herdr/worktree.toml). Runs here; Herdr UI need not run.\nherdr-personal-dev scripts setup|run\n    Run one in a new tab of the caller's workspace (setup: linked worktrees\n    only). Herdr UI asks before running a script it has not seen. The\n    archive script runs only when a worktree is removed in Herdr UI."
        }
        "teleport" => {
            "herdr-personal-dev teleport --to HOST\n    Open the teleport dialog for the caller's linked worktree, reviewing\n    HOST (a host ID or label). The user confirms the move there."
        }
        "fan-out" => {
            "herdr-personal-dev fan-out PROMPT... [--agent KIND[:COUNT]]...\n    Open the fan-out dialog for the caller's repository with PROMPT and\n    these agents (claude, codex, ...; at most 6 in all) chosen once the\n    host's agents are known. The user starts it there."
        }
        "pr" => {
            "herdr-personal-dev pr [status]|checks\n    The pull request of the caller's workspace's branch, as Herdr UI last\n    looked it up (selected host only; sign in to GitHub first). Exits 4\n    when there is none.\nherdr-personal-dev pr comments\n    Its review comments (the workspace must be the focused one).\nherdr-personal-dev pr open\n    Open it in a browser tab."
        }
        "host" => {
            "herdr-personal-dev host [list]\n    The window's hosts and their status. Herdr's `machine` command\n    manages saved hosts.\nherdr-personal-dev host select ID\n    Show that host in the window, as choosing it in the sidebar does."
        }
        "cloud" => {
            "herdr-personal-dev cloud [list]\n    Saved cloud machines (Herdr UI need not run).\nherdr-personal-dev cloud add\n    Open the settings that add a machine. A machine is never created or\n    destroyed from the command line."
        }
        "ports" => {
            "herdr-personal-dev ports [list]\n    Ports the caller's workspace's processes listen on, and forwards.\n    Listening ports are known only while show_listening_ports is on.\nherdr-personal-dev ports forward|unforward PORT\n    Forward a port of the workspace's SSH host to this machine, or stop.\nherdr-personal-dev ports open PORT\n    Show localhost:PORT in a browser tab, forwarding it first if remote."
        }
        "processes" => {
            "herdr-personal-dev processes [--pane ID]\n    The processes under a pane (default HERDR_PANE_ID), with CPU and\n    memory. Runs here; Herdr UI need not run."
        }
        "usage" => "herdr-personal-dev usage\n    AI plan usage on the window's selected host.",
        "system" => {
            "herdr-personal-dev system\n    The window's selected host's load, when Herdr UI samples it."
        }
        "search" => {
            "herdr-personal-dev search symbol|file QUERY... [--limit N]\n    Go to Symbol or Go to File over the current directory's checkout,\n    printing paths relative to its root. Runs here; Herdr UI need not run."
        }
        "config" => {
            "herdr-personal-dev config [path]\n    The settings files.\nherdr-personal-dev config get KEY\n    The setting's value, as TOML: your override, else the default.\nherdr-personal-dev config set KEY VALUE\nherdr-personal-dev config unset KEY\n    Change a setting in config-gpui.local.toml. VALUE is read as TOML\n    (14, true, [\"a\"], { show = true }), else as text. A value or key\n    Herdr UI would not load is refused and the file left as it was.\n    A running Herdr UI applies the change at once. KEY is dotted, such\n    as terminal.size. Unsetting a key that is not set exits 4.\nherdr-personal-dev config check\n    Report settings that do not load and unknown keys, and, when Herdr UI\n    runs, fonts that are not installed; exits 1 when there are any.\nherdr-personal-dev config reload\n    Reread Herdr's config and this app's.\nOnly reload needs Herdr UI running."
        }
        "theme" => {
            "herdr-personal-dev theme [list]\n    The themes there are; * marks the selected one.\nherdr-personal-dev theme set NAME...\n    Choose and save a theme, or light:NAME,dark:NAME to follow the\n    system appearance. Saved even when Herdr UI is not running."
        }
        "font" => {
            "herdr-personal-dev font [list]\n    The installed font families.\nherdr-personal-dev font set NAME...\nherdr-personal-dev font reset\n    The font of every face; reset goes back to the default.\nThese need Herdr UI running, which knows the installed fonts."
        }
        "keybinds" => {
            "herdr-personal-dev keybinds [list]\n    The window's shortcuts.\nherdr-personal-dev keybinds check\n    Herdr's custom commands this client cannot reach; exits 1 if any."
        }
        "caffeine" => {
            "herdr-personal-dev caffeine [status]\nherdr-personal-dev caffeine on|off|toggle [--for 90s|30m|2h]\n    Keep the display and the machine awake. With --for (at most 24h),\n    caffeine turns off then, unless it was changed in the meantime."
        }
        "sound" => {
            "herdr-personal-dev sound [list]\n    The notification sounds.\nherdr-personal-dev sound play [done|request]\n    Play one now, from the file Herdr's sound settings name or the\n    built-in one."
        }
        "bell" => "herdr-personal-dev bell\n    Ring the system bell, as a terminal bell does.",
        "events" => {
            "herdr-personal-dev events [--after CURSOR] [--wait SECONDS] [--kind KIND]... [--follow]\n    What happened since CURSOR, the INSTANCE:SEQ the last call printed:\n    windows and tabs opening and closing, notes, and host status. A cursor\n    from an earlier app run gets everything this run kept. --follow keeps\n    printing until interrupted. Kinds:\n    window_opened, window_closed, window_focused, tab_opened, tab_closed,\n    feedback, note, host."
        }
        "wait" => {
            "herdr-personal-dev wait feedback|window|online|tab-closed ID [--timeout SECONDS]\n    Wait (default 60, at most 600 seconds). Exits 4 when time runs out."
        }
        "run" => {
            "herdr-personal-dev run COMMAND\n    Run an app command by its [keybindings] name, as the command palette\n    does, for example toggle_sidebar. See `commands`."
        }
        "commands" => {
            "herdr-personal-dev commands\n    The commands `run` accepts, with their shortcuts."
        }
        "logs" => {
            "herdr-personal-dev logs [--lines N] [--path]\n    The app's latest log records (default 100, at most 5000), or the log file's path.\n    Runs here; Herdr UI need not run. Exits 4 when there is no log yet."
        }
        "update" => {
            "herdr-personal-dev update [status]\nherdr-personal-dev update check\n    Ask for the latest release. Never downloads or installs. Local\n    builds report updates as off."
        }
        "diagnostics" => {
            "herdr-personal-dev diagnostics bundle PATH\n    Write build information, settings diagnostics, your settings\n    overrides, and recent logs to a new file at PATH for a bug report,\n    with home paths shortened and secret-looking values redacted. Read\n    it before sharing. Never replaces an existing file. Runs here."
        }
        "integrations" => {
            "herdr-personal-dev integrations [list]\n    The agent integrations Herdr offers the window's host. Install them\n    from Herdr UI's integrations settings.\nherdr-personal-dev integrations install-skill\n    Install or update the agent skill for Claude Code and other agents.\n    Runs here; Herdr UI need not run."
        }
        _ => "",
    }
}

/// Usage for one family, or the list of families.
pub(super) fn usage(name: Option<&str>) -> String {
    match name {
        Some("browser") => BROWSER.to_owned(),
        Some(name) => format!("{}\n\n{COMMON}", family(name)),
        None => {
            let mut text =
                String::from("herdr-personal-dev COMMAND ... [--json] [--window ID]\n\n");
            for (name, summary) in FAMILIES {
                text.push_str(&format!("  {name:<14}{summary}\n"));
            }
            text.push_str("\nRun `herdr-personal-dev COMMAND --help` for one command's usage.\n\n");
            text.push_str(COMMON);
            text
        }
    }
}
