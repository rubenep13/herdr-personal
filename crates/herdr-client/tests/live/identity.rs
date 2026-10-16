use super::*;

fn create_workspace(daemon: &Daemon, session: &mut Session, label: &str) -> String {
    let boot = session.snapshot.as_ref().unwrap().boot_id.clone();
    let request = session
        .client
        .handle
        .request(
            &boot,
            Method::WorkspaceCreate,
            json!({"cwd": daemon.sandbox.dir, "label": label, "focus": true}),
        )
        .unwrap();
    let result = session.response(request);
    let workspace = result["workspace"]["workspace_id"]
        .as_str()
        .unwrap()
        .to_owned();
    if session
        .snapshot
        .as_ref()
        .unwrap()
        .focused_workspace_id
        .as_deref()
        != Some(&workspace)
    {
        session.until("created workspace", |event| {
            matches!(event,
            ClientEvent::Snapshot(s) if s.focused_workspace_id.as_deref() == Some(&workspace))
        });
    }
    workspace
}

fn restart(daemon: &mut Daemon, session: Session) -> Session {
    session.detach();
    let child = daemon.child.as_mut().unwrap();
    // Signal only the exact child this fixture owns; graceful shutdown saves
    // the structural session. Drop still kills/reaps it on every failure path.
    rustix::process::kill_process(
        rustix::process::Pid::from_child(child),
        rustix::process::Signal::TERM,
    )
    .unwrap();
    let deadline = Instant::now() + TIMEOUT;
    while child.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "isolated daemon did not stop");
        thread::sleep(Duration::from_millis(20));
    }
    assert!(child.wait().unwrap().success());
    daemon.child = Some(
        daemon
            .sandbox
            .command(daemon_binary(), "restart.log")
            .arg("server")
            .spawn()
            .unwrap(),
    );
    daemon
        .sandbox
        .wait_for_daemon(daemon.child.as_mut().unwrap(), TIMEOUT);
    Session::open(daemon)
}

fn public_ids(session: &Session) -> (Vec<String>, Vec<String>) {
    let snapshot = session.snapshot.as_ref().unwrap();
    let mut workspaces: Vec<_> = snapshot
        .workspaces
        .iter()
        .map(|w| w.workspace_id.clone())
        .collect();
    let mut panes: Vec<_> = snapshot.panes.iter().map(|p| p.pane_id.clone()).collect();
    workspaces.sort();
    panes.sort();
    (workspaces, panes)
}

#[test]
#[ignore = "requires explicit HERDR_TEST_BINARY; restarts only an isolated daemon"]
fn public_ids_survive_structural_session_restore() {
    let mut daemon = Daemon::start();
    let mut session = Session::open(&daemon);
    let workspace = create_workspace(&daemon, &mut session, "persisted");
    let boot = session.snapshot.as_ref().unwrap().boot_id.clone();
    let request = session
        .client
        .handle
        .request(
            &boot,
            Method::PaneSplit,
            json!({"workspace_id": workspace, "direction": "right", "focus": true}),
        )
        .unwrap();
    let response = session.response(request);
    let pane = response["pane"]["pane_id"].as_str().unwrap().to_owned();
    if !session
        .snapshot
        .as_ref()
        .unwrap()
        .panes
        .iter()
        .any(|p| p.pane_id == pane)
    {
        session.until("split snapshot", |event| {
            matches!(event,
            ClientEvent::Snapshot(s) if s.panes.iter().any(|p| p.pane_id == pane))
        });
    }
    let before = public_ids(&session);
    let restored = restart(&mut daemon, session);
    assert_ne!(restored.snapshot.as_ref().unwrap().boot_id, boot);
    assert_eq!(public_ids(&restored), before);
    eprintln!("public workspace/pane IDs retained across a new daemon boot: {before:?}");
    restored.detach();
}

#[test]
#[ignore = "requires explicit HERDR_TEST_BINARY; restarts only an isolated daemon"]
fn deleted_highest_workspace_id_can_be_reused_after_restart() {
    let mut daemon = Daemon::start();
    let mut session = Session::open(&daemon);
    create_workspace(&daemon, &mut session, "retained");
    let deleted = create_workspace(&daemon, &mut session, "deleted");
    let boot = session.snapshot.as_ref().unwrap().boot_id.clone();
    let request = session
        .client
        .handle
        .request(
            &boot,
            Method::WorkspaceClose,
            json!({"workspace_id": deleted, "close_group": false}),
        )
        .unwrap();
    session.response(request);
    if session
        .snapshot
        .as_ref()
        .unwrap()
        .workspaces
        .iter()
        .any(|w| w.workspace_id == deleted)
    {
        session.until("closed workspace", |event| {
            matches!(event,
            ClientEvent::Snapshot(s) if !s.workspaces.iter().any(|w| w.workspace_id == deleted))
        });
    }
    let mut restored = restart(&mut daemon, session);
    let created = create_workspace(&daemon, &mut restored, "new unrelated workspace");
    assert_eq!(
        created, deleted,
        "characterization: public IDs are not lifetime UUIDs"
    );
    eprintln!("a different workspace reused {created} across daemon boots");
    restored.detach();
}
