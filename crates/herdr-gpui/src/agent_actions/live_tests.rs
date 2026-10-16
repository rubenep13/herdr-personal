//! Opt-in: a start against a real Herdr CLI and an isolated daemon. Requires
//! `HERDR_TEST_BINARY` (see AGENTS.md). The sandbox has no agent installed,
//! so the start is sent but cannot become ready: this checks the real CLI's
//! answers, the pane the tab gives, and that nothing is started twice.

use super::*;
use crate::teleport::live_tests::Daemon;
use std::path::PathBuf;

#[test]
#[ignore = "requires HERDR_TEST_BINARY and starts one isolated daemon"]
fn a_real_cli_starts_in_a_new_tab_of_the_exact_space_once() {
    let binary = PathBuf::from(
        std::env::var_os("HERDR_TEST_BINARY").expect("set HERDR_TEST_BINARY to an absolute herdr"),
    );
    assert!(binary.is_absolute());
    let daemon = Daemon::start(&binary, "agent");
    let created = daemon.herdr(&[
        "workspace",
        "create",
        "--cwd",
        &daemon.dir.to_string_lossy(),
        "--no-focus",
    ]);
    let workspace = created["workspace"]["workspace_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let begun = Instant::now();
    let outcome = start_agent(
        &daemon.host,
        &StartAgent {
            workspace_id: workspace.clone(),
            kind: AgentKind::Claude,
            name: agent_name(AgentKind::Claude),
        },
        &AtomicBool::new(false),
        DEADLINE,
    );
    eprintln!("outcome after {:?}: {outcome:?}", begun.elapsed());
    let pane = match &outcome {
        StartOutcome::Unknown { pane_id, .. } => pane_id.clone(),
        other => panic!("without an installed agent the start cannot be ready: {other:?}"),
    };
    assert!(begun.elapsed() < DEADLINE + Duration::from_secs(5));
    // The pane the start went to is the new tab's, in the space asked for.
    let info = daemon.herdr(&["pane", "get", &pane]);
    assert_eq!(info["pane"]["workspace_id"], workspace.as_str(), "{info}");
    let tabs = daemon.herdr(&["tab", "list", "--workspace", &workspace]);
    assert_eq!(tabs["tabs"].as_array().unwrap().len(), 2, "{tabs}");
}
