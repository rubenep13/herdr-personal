//! The graph stays live while it is shown: a change in the repository
//! reloads it, the selection follows its commit (or HEAD when it is gone),
//! hidden tabs ask nothing, remote checks reuse one shell, and a failed
//! check reads as reconnecting.

use super::*;
use crate::git_graph::{
    GraphCheckout,
    error::Error,
    history::History,
    model::{Ref, RefKind},
    view::{Connection, GitGraphView, Liveness},
    worker::{Reply, Request},
};
use crate::teleport::Host;
use gpui::{AppContext, Entity, TestAppContext};
use herdr_client::ConnectTarget;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

fn later() -> Instant {
    Instant::now() + Duration::from_secs(60)
}

/// A view with a worker that never answers (its folder does not exist), its
/// history given by the test.
fn answered(cx: &mut TestAppContext, history: History) -> Entity<GitGraphView> {
    let checkout = GraphCheckout::new("/nonexistent/graph-test".into()).unwrap();
    let local = Host::new(&ConnectTarget::Local).unwrap();
    let view = cx.new(|cx| GitGraphView::new(Ok(local), checkout, look(), Connection::Online, cx));
    cx.update(|cx| {
        view.update(cx, |view, cx| {
            let first = view.generation();
            assert!(view.receive(first, Reply::History(Arc::new(history)), cx));
        })
    });
    view
}

#[gpui::test]
fn reload_keeps_the_selected_commit(cx: &mut TestAppContext) {
    let view = answered(
        cx,
        History::new(&line(5), &[], Some("c0".into()), None, false),
    );
    cx.update(|cx| {
        view.update(cx, |view, cx| {
            view.select_oid("c2");
            assert_eq!(view.selected_row(), Some(2));
            let mut commits = vec![commit("new", &["c0"])];
            commits.extend(line(5));
            let reloaded = History::new(&commits, &[], Some("new".into()), None, false);
            let asked = view.generation();
            assert!(view.receive(asked, Reply::History(Arc::new(reloaded)), cx));
            assert_eq!(view.selected_row(), Some(3), "c2 moved down a row");
        })
    });
}

#[gpui::test]
fn rewritten_history_falls_back_to_head(cx: &mut TestAppContext) {
    let view = answered(
        cx,
        History::new(&line(5), &[], Some("c0".into()), None, false),
    );
    cx.update(|cx| {
        view.update(cx, |view, cx| {
            view.select_oid("c0");
            // A reset moved main back: c0 is gone, c1 is HEAD.
            let refs = [Ref {
                name: "main".into(),
                target: "c1".into(),
                kind: RefKind::Current,
            }];
            let rewritten = History::new(&line(5)[1..], &refs, Some("c1".into()), None, false);
            let asked = view.generation();
            assert!(view.receive(asked, Reply::History(Arc::new(rewritten)), cx));
            assert_eq!(view.selected_row(), Some(0), "HEAD's row");
        })
    });
}

#[gpui::test]
fn hidden_tabs_do_not_probe(cx: &mut TestAppContext) {
    let view = answered(cx, History::new(&line(5), &[], None, None, false));
    cx.update(|cx| {
        view.update(cx, |view, cx| {
            view.set_visible(false, cx);
            view.live_tick(later());
            assert_eq!(view.last_probe(), None);
            view.set_visible(true, cx);
            view.live_tick(later());
            assert_eq!(
                view.last_probe(),
                None,
                "the first tick only starts the clock"
            );
            view.live_tick(later() + Duration::from_secs(60));
            assert!(view.last_probe().is_some());
            // One check at a time.
            let probe = view.last_probe();
            view.live_tick(later() + Duration::from_secs(120));
            assert_eq!(view.last_probe(), probe);
        })
    });
}

#[gpui::test]
fn failed_probe_says_reconnecting(cx: &mut TestAppContext) {
    let view = answered(cx, History::new(&line(5), &[], None, None, false));
    cx.update(|cx| {
        view.update(cx, |view, cx| {
            view.live_tick(later());
            view.live_tick(later() + Duration::from_secs(60));
            let probe = view.last_probe().unwrap();
            assert_eq!(view.liveness(), Liveness::Live);
            assert!(view.receive(probe, Reply::Failed(Arc::new(Error::WorkerStopped)), cx));
            assert_eq!(view.liveness(), Liveness::Reconnecting);
            assert_eq!(view.indicator(), "reconnecting");
            assert!(view.history().is_some(), "the rows stay");
            view.live_tick(later() + Duration::from_secs(120));
            let probe = view.last_probe().unwrap();
            assert!(view.receive(probe, Reply::Probe { changed: false }, cx));
            assert_eq!(view.liveness(), Liveness::Live);
            // A change reloads.
            view.live_tick(later() + Duration::from_secs(180));
            let probe = view.last_probe().unwrap();
            assert!(view.receive(probe, Reply::Probe { changed: true }, cx));
            assert_eq!(view.last_request().1, Request::Load);
        })
    });
}

#[cfg(unix)]
#[gpui::test]
fn a_new_commit_reloads_the_graph(cx: &mut TestAppContext) {
    let fixture = Fixture::with_commits(2);
    let checkout = GraphCheckout::new(fixture.path().to_str().unwrap().into()).unwrap();
    let view = cx
        .new(|cx| GitGraphView::new(Ok(fixture.host()), checkout, look(), Connection::Online, cx));
    let until = |cx: &mut TestAppContext, done: &dyn Fn(&GitGraphView) -> bool| {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !cx.update(|cx| {
            view.update(cx, |view, cx| {
                view.poll(cx);
                done(view)
            })
        }) {
            assert!(Instant::now() < deadline, "the worker never answered");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    until(cx, &|view| view.history().is_some());
    // An unchanged repository does not reload.
    cx.update(|cx| view.update(cx, |view, cx| view.probe_now(cx)));
    until(cx, &|view| view.last_probe().is_none());
    let loads = cx.update(|cx| view.read(cx).generation());
    let head = fixture.head();
    assert_eq!(
        cx.update(|cx| view.read(cx).history().unwrap().commits[0].oid.clone()),
        head
    );

    fixture.write("file.txt", "a new commit\n");
    let newer = fixture.commit("a new commit");
    cx.update(|cx| view.update(cx, |view, cx| view.probe_now(cx)));
    until(cx, &|view| {
        view.history()
            .is_some_and(|history| history.commits.first().is_some_and(|c| c.oid == newer))
    });
    assert!(cx.update(|cx| view.read(cx).generation()) > loads);
}

#[cfg(unix)]
#[test]
fn remote_probes_reuse_one_shell() {
    use crate::git_graph::live::Prober;
    use std::sync::atomic::AtomicBool;
    let fixture = Fixture::with_commits(1);
    let mut command = Command::new("/bin/sh");
    command
        .arg("-s")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let shell = crate::usage::Shell::start(command).unwrap();
    let mut prober = Prober::Remote(shell);
    let root = fixture.path().to_str().unwrap();
    let cancelled = AtomicBool::new(false);
    let pid = prober.shell_pid().unwrap();
    let first = prober.fingerprint(root, &cancelled).unwrap();
    assert_eq!(prober.fingerprint(root, &cancelled).unwrap(), first);
    assert_eq!(prober.fingerprint(root, &cancelled).unwrap(), first);
    assert_eq!(
        prober.shell_pid().unwrap(),
        pid,
        "one shell for every check"
    );
    fixture.write("file.txt", "changed\n");
    assert_ne!(
        prober.fingerprint(root, &cancelled).unwrap(),
        first,
        "the working tree counts"
    );
    // The same check through the local route agrees.
    let mut local = Prober::Local(fixture.host());
    assert_eq!(
        local.fingerprint(root, &cancelled).unwrap(),
        prober.fingerprint(root, &cancelled).unwrap()
    );
}
