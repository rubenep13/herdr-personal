//! A workspace on an SSH host gets its graph read there: an offline host
//! says so and reads again once back, SSH failures read as the host's, and
//! closing the tab stops the script it was waiting for.

use super::*;
use crate::{
    browser::{Location, Store, scope},
    controls::Command,
    git_graph::{
        GraphCheckout,
        error::Error,
        view::{Connection, GitGraphView, error_title},
        worker::Request,
    },
    sidebar::layout_tests::{fixture_window, snapshot},
    teleport::Host,
};
use gpui::{AppContext, TestAppContext};
use herdr_client::ConnectTarget;
use std::sync::Arc;

fn ssh() -> ConnectTarget {
    ConnectTarget::Ssh {
        target: "graph-test.invalid".into(),
        session: "default".into(),
    }
}

#[gpui::test]
fn ssh_hosts_open_a_graph(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        let mut shown = snapshot(4);
        shown.focused_workspace_id = Some("w0".into());
        view.live.snapshot = Some(Arc::new(shown));
        // Not connected yet, so nothing runs over SSH in this test.
        view.endpoints[0].connection.target = ssh();
        view
    });
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.command(Command::OpenGitGraph, window, cx)
        })
    });
    let tabs = cx.update(|_, cx| {
        let tab_scope = scope(&view.read(cx).endpoints[0]);
        cx.global::<Store>()
            .in_workspace(&tab_scope, "w0")
            .filter(|tab| matches!(tab.location, Some(Location::GitGraph { .. })))
            .count()
    });
    assert_eq!(tabs, 1);
    cx.update(|_, cx| {
        let window = view.read(cx);
        let graph = window.git_graphs.values().next().expect("a view");
        let graph = graph.view().read(cx);
        assert_eq!(graph.host(), Some(&Host::new(&ssh()).unwrap()));
        assert_eq!(graph.connection(), Connection::Offline);
        assert_eq!(graph.generation(), 0, "nothing is asked of an offline host");
    });
}

#[gpui::test]
fn offline_host_says_so_and_reloads_when_back(cx: &mut TestAppContext) {
    let checkout = GraphCheckout::new("/nonexistent/graph-test".into()).unwrap();
    let local = Host::new(&ConnectTarget::Local).unwrap();
    let view = cx.new(|cx| GitGraphView::new(Ok(local), checkout, look(), Connection::Online, cx));
    cx.update(|cx| {
        view.update(cx, |view, cx| {
            let first = view.generation();
            view.set_connection(Connection::Offline, cx);
            assert_eq!(view.indicator(), "offline");
            view.reload(cx);
            assert_eq!(view.generation(), first, "an offline host is not asked");
            view.set_connection(Connection::Online, cx);
            assert_eq!(view.indicator(), "live");
            assert_eq!(view.last_request(), (first + 1, Request::Load));
            view.set_connection(Connection::Online, cx);
            assert_eq!(view.generation(), first + 1, "only a return reloads");
        })
    });
}

#[cfg(unix)]
#[gpui::test]
fn closing_the_tab_cancels_its_worker(cx: &mut TestAppContext) {
    use std::{
        os::unix::fs::PermissionsExt,
        sync::atomic::Ordering,
        time::{Duration, Instant},
    };
    // A git that says it started, then hangs.
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join(".local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(
        bin.join("git"),
        "#!/bin/sh\ntouch \"$HOME/started\"\nexec sleep 30\n",
    )
    .unwrap();
    std::fs::set_permissions(bin.join("git"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut host = Host::new(&ConnectTarget::Local).unwrap();
    host.env = vec![("HOME".into(), home.path().to_string_lossy().into_owned())];
    let checkout = GraphCheckout::new("/tmp".into()).unwrap();
    let view = cx.new(|cx| GitGraphView::new(Ok(host), checkout, look(), Connection::Online, cx));

    let started = Instant::now();
    while !home.path().join("started").exists() {
        assert!(started.elapsed() < Duration::from_secs(10), "git never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    let finished = cx.update(|cx| view.read(cx).worker_finished().unwrap());
    assert!(
        !cx.update(|cx| view.update(cx, |view, cx| view.poll(cx))),
        "no reply yet"
    );
    assert!(!finished.load(Ordering::Acquire));

    // The tab's view goes, and with it the worker.
    drop(view);
    // Released entities drop at the end of an update.
    cx.update(|_| {});
    let closed = Instant::now();
    while !finished.load(Ordering::Acquire) {
        assert!(
            closed.elapsed() < Duration::from_secs(5),
            "the worker kept waiting"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn ssh_failures_read_as_host_errors() {
    let script = |source| Error::Script {
        operation: "log",
        source,
    };
    let refused = script(herdr_client::Error::ScriptConnect(std::io::Error::other(
        "refused",
    )));
    assert_eq!(
        error_title(&refused, Some("Build box")),
        "Cannot reach Build box"
    );
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        let exit = |code: i32| herdr_client::Error::ScriptExit {
            status: std::process::ExitStatus::from_raw(code << 8),
            stderr: "ssh: connect to host graph-test.invalid: Name or service not known".into(),
        };
        assert_eq!(
            error_title(&script(exit(255)), Some("Build box")),
            "Cannot reach Build box"
        );
        // Git's own failure stays a repository error, here or over SSH.
        assert_eq!(
            error_title(&script(exit(128)), Some("Build box")),
            "Could not read the repository"
        );
        assert_eq!(
            error_title(&script(exit(255)), None),
            "Could not read the repository"
        );
    }
    assert_eq!(
        error_title(
            &script(herdr_client::Error::ScriptTimeout),
            Some("Build box")
        ),
        "Build box took too long to answer"
    );
    assert_eq!(
        error_title(&script(herdr_client::Error::ScriptOutputLimit), None),
        "The repository's history is too large to read at once"
    );
    assert_eq!(
        error_title(&Error::NotARepository, Some("x")),
        "Not a Git repository"
    );
    assert_eq!(
        error_title(
            &script(herdr_client::Error::ScriptUnsupported),
            Some("Build box")
        ),
        "Remote Git graphs need macOS or Linux"
    );
}

#[cfg(windows)]
#[gpui::test]
fn windows_has_no_remote_graph(cx: &mut TestAppContext) {
    use std::time::{Duration, Instant};
    let host = Host::new(&ssh()).unwrap();
    let checkout = GraphCheckout::new("/home/me/repo".into()).unwrap();
    let view = cx.new(|cx| {
        GitGraphView::new(Ok(host), checkout, look(), Connection::Online, cx)
            .remote(Some("Build box".into()))
    });
    let started = Instant::now();
    while cx.update(|cx| view.read(cx).error().is_none()) {
        cx.update(|cx| view.update(cx, |view, cx| view.poll(cx)));
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(20));
    }
    let title = cx.update(|cx| {
        let view = view.read(cx);
        error_title(view.error().unwrap(), Some("Build box"))
    });
    assert_eq!(title, "Remote Git graphs need macOS or Linux");
}
