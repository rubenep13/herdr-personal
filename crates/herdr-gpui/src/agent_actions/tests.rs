use super::*;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

/// A home whose `herdr` logs each call's argv, one argument per line, and
/// answers as Herdr would. `$HOME/<step>` files change one answer: `busy`
/// holds how many `agent start` calls are refused as busy, `start` replaces
/// the start's behavior with a shell snippet, `tab` the tab creation's, and
/// `terminal` the terminal `pane get` reports, and `missing` makes it answer
/// as a shell without the CLI does.
fn fake_home(dir: &Path) -> Host {
    let bin = dir.join("home/.local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    let script = r#"#!/bin/sh
{ printf 'call'; for arg in "$@"; do printf '\n%s' "$arg"; done; printf '\n'; } >> "$HOME/calls"
# As a shell reports a CLI it cannot find, without searching a real one.
if [ -f "$HOME/missing" ]; then echo "herdr: not found" >&2; exit 127; fi
case "$1 $2" in
  "tab create")
    if [ -f "$HOME/tab" ]; then . "$HOME/tab"; fi
    printf '%s' '{"id":"cli:tab:create","result":{"tab":{"tab_id":"w1:t2"},"root_pane":{"pane_id":"w1:p7","terminal_id":"term7"}}}' ;;
  "pane get")
    terminal=term7
    if [ -f "$HOME/terminal" ]; then read -r terminal < "$HOME/terminal"; fi
    printf '{"result":{"pane":{"pane_id":"w1:p7","terminal_id":"%s"}}}' "$terminal" ;;
  "agent start")
    if [ -f "$HOME/busy" ]; then
      read -r busy < "$HOME/busy"
      if [ "$busy" -gt 0 ]; then
        printf '%s\n' "$((busy - 1))" > "$HOME/busy"
        printf '%s' '{"id":"cli:agent:start","error":{"code":"agent_pane_busy","message":"not an available shell"}}' >&2
        exit 1
      fi
    fi
    if [ -f "$HOME/start" ]; then . "$HOME/start"; fi
    printf '%s' '{"id":"cli:agent:start","result":{"agent":{"pane_id":"w1:p7"}}}' ;;
esac
"#;
    std::fs::write(bin.join("herdr"), script).unwrap();
    std::fs::set_permissions(bin.join("herdr"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut host = Host::new(&ConnectTarget::Local).unwrap();
    host.env = vec![(
        "HOME".into(),
        dir.join("home").to_string_lossy().into_owned(),
    )];
    host
}

fn home(dir: &Path) -> PathBuf {
    dir.join("home")
}

/// Every call the fake `herdr` received, each as its argv.
fn calls(dir: &Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(home(dir).join("calls"))
        .unwrap_or_default()
        .split("call\n")
        .filter(|call| !call.is_empty())
        .map(|call| call.lines().map(str::to_owned).collect())
        .collect()
}

fn starts(dir: &Path) -> usize {
    calls(dir)
        .iter()
        .filter(|argv| argv[..2] == ["agent", "start"])
        .count()
}

fn request(workspace: &str) -> StartAgent {
    StartAgent {
        workspace_id: workspace.into(),
        kind: AgentKind::Claude,
        name: "claude-test".into(),
    }
}

fn start(host: &Host, request: &StartAgent) -> StartOutcome {
    start_agent(host, request, &AtomicBool::new(false), DEADLINE)
}

#[test]
fn a_fresh_tab_in_the_exact_space_runs_the_official_start() {
    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    let outcome = start(&host, &request("w1"));
    assert!(
        matches!(&outcome, StartOutcome::Ready { pane_id } if pane_id == "w1:p7"),
        "{outcome:?}"
    );
    assert_eq!(
        calls(dir.path()),
        [
            vec!["tab", "create", "--workspace", "w1", "--no-focus"],
            vec![
                "agent",
                "start",
                "claude-test",
                "--kind",
                "claude",
                "--pane",
                "w1:p7",
                "--timeout",
                READY_TIMEOUT_MS,
            ],
        ]
    );
}

#[test]
fn hostile_text_reaches_the_cli_as_one_argument_and_is_never_evaluated() {
    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    let workspace = "w1'; touch \"$HOME/pwned\"; echo '$(id) `id` \\ * ?";
    let outcome = start(&host, &request(workspace));
    assert!(matches!(outcome, StartOutcome::Ready { .. }), "{outcome:?}");
    assert_eq!(calls(dir.path())[0][3], workspace);
    assert!(!home(dir.path()).join("pwned").exists());
}

#[test]
fn a_shell_still_starting_is_waited_for_on_the_same_terminal_only() {
    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    std::fs::write(home(dir.path()).join("busy"), "2\n").unwrap();
    let outcome = start(&host, &request("w1"));
    assert!(matches!(outcome, StartOutcome::Ready { .. }), "{outcome:?}");
    assert_eq!(starts(dir.path()), 3);

    // A replaced terminal behind the same public pane ID is never started in.
    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    std::fs::write(home(dir.path()).join("busy"), "1\n").unwrap();
    std::fs::write(home(dir.path()).join("terminal"), "other\n").unwrap();
    let outcome = start(&host, &request("w1"));
    let StartOutcome::Failed(Error::Rejected { code, .. }) = &outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(code, "agent_pane_busy");
    assert_eq!(starts(dir.path()), 1);
}

#[test]
fn a_rejection_before_launch_fails_and_anything_after_it_is_unknown() {
    for (code, launched) in [
        ("unsupported_agent_kind", false),
        ("agent_pane_not_found", false),
        ("agent_name_taken", false),
        ("agent_not_ready", true),
        ("agent_kind_mismatch", true),
        ("agent_start_transport_failed", true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let host = fake_home(dir.path());
        std::fs::write(
            home(dir.path()).join("start"),
            format!(
                r#"printf '%s' '{{"id":"cli:agent:start","error":{{"code":"{code}","message":"m"}}}}' >&2; exit 1"#
            ),
        )
        .unwrap();
        let outcome = start(&host, &request("w1"));
        match (&outcome, launched) {
            (StartOutcome::Failed(Error::Rejected { code: got, .. }), false) => {
                assert_eq!(got, code);
            }
            (StartOutcome::Unknown { pane_id, .. }, true) => assert_eq!(pane_id, "w1:p7"),
            _ => panic!("{code}: {outcome:?}"),
        }
        assert_eq!(starts(dir.path()), 1, "{code} must never be retried");
    }
}

#[test]
fn a_missing_cli_or_a_failed_tab_starts_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    std::fs::write(home(dir.path()).join("missing"), "").unwrap();
    let outcome = start(&host, &request("w1"));
    assert!(
        matches!(
            outcome,
            StartOutcome::Failed(Error::Script {
                step: Step::CreateTab,
                ..
            })
        ),
        "{outcome:?}"
    );

    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    std::fs::write(
        home(dir.path()).join("tab"),
        r#"printf '%s' '{"id":"cli:tab:create","error":{"code":"workspace_not_found","message":"m"}}' >&2; exit 1"#,
    )
    .unwrap();
    let outcome = start(&host, &request("w9"));
    assert!(matches!(outcome, StartOutcome::Failed(_)), "{outcome:?}");
    assert_eq!(starts(dir.path()), 0);
}

#[test]
fn the_deadline_is_absolute_even_while_the_start_keeps_printing() {
    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    // Progress on every beat keeps an idle timeout from ever firing.
    std::fs::write(
        home(dir.path()).join("start"),
        "while :; do printf .; sleep 0.05; done",
    )
    .unwrap();
    let begun = Instant::now();
    let outcome = start_agent(
        &host,
        &request("w1"),
        &AtomicBool::new(false),
        Duration::from_millis(600),
    );
    assert!(
        begun.elapsed() < Duration::from_secs(10),
        "{:?}",
        begun.elapsed()
    );
    let StartOutcome::Unknown { pane_id, error } = &outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(pane_id, "w1:p7");
    assert!(matches!(error, Error::Deadline(_)), "{error:?}");
    assert_eq!(starts(dir.path()), 1);
}

#[test]
fn cancelling_a_sent_start_leaves_it_unknown_and_unrepeated() {
    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    std::fs::write(home(dir.path()).join("start"), "sleep 30").unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let canceller = thread::spawn(move || {
        thread::sleep(Duration::from_millis(400));
        flag.store(true, Ordering::Release);
    });
    let begun = Instant::now();
    let outcome = start_agent(&host, &request("w1"), &cancelled, DEADLINE);
    canceller.join().unwrap();
    assert!(
        begun.elapsed() < Duration::from_secs(10),
        "{:?}",
        begun.elapsed()
    );
    assert!(
        matches!(
            outcome,
            StartOutcome::Unknown {
                error: Error::Cancelled,
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(starts(dir.path()), 1);
}

#[test]
fn a_connection_lost_after_the_launch_is_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    // As `ssh` reports a dropped connection.
    std::fs::write(home(dir.path()).join("start"), "exit 255").unwrap();
    let outcome = start(&host, &request("w1"));
    assert!(
        matches!(
            outcome,
            StartOutcome::Unknown {
                error: Error::Script {
                    step: Step::StartAgent,
                    ..
                },
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(starts(dir.path()), 1);
}

#[test]
fn output_beyond_the_bound_is_not_read_as_success() {
    let dir = tempfile::tempdir().unwrap();
    let host = fake_home(dir.path());
    std::fs::write(
        home(dir.path()).join("start"),
        "head -c 2000000 /dev/zero | tr '\\0' x",
    )
    .unwrap();
    let outcome = start(&host, &request("w1"));
    assert!(
        matches!(outcome, StartOutcome::Unknown { .. }),
        "{outcome:?}"
    );
}

#[test]
fn agent_names_are_valid_herdr_names_and_unique() {
    let names: std::collections::HashSet<_> =
        (0..64).map(|_| agent_name(AgentKind::Cursor)).collect();
    assert_eq!(names.len(), 64);
    for name in &names {
        assert!(name.len() <= 32, "{name}");
        assert!(name.starts_with("cursor-"), "{name}");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{name}"
        );
    }
}
