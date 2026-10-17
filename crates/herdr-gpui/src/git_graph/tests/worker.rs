//! The tab's worker and view: a history arrives off the UI thread, a stale
//! reply never replaces a newer one, and failures show in the tab.

use super::*;
use crate::git_graph::{
    GraphCheckout,
    error::Error,
    git::WorktreeCounts,
    history::History,
    model::WORKTREE_OID,
    view::{Connection, GitGraphView, GraphLook, error_title},
    worker::Reply,
};
use crate::{browser::checkout_path_ok, teleport::Host};
use gpui::{AppContext, Entity, TestAppContext, VisualTestContext};
use herdr_client::ConnectTarget;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

fn look() -> GraphLook {
    let config = crate::config::Config::default();
    GraphLook {
        theme: Default::default(),
        ui: config.ui.clone(),
        mono: config.terminal.clone(),
    }
}

fn local() -> Result<Host, Error> {
    Ok(Host::new(&ConnectTarget::Local).unwrap())
}

fn view_of<'a>(
    cx: &'a mut TestAppContext,
    folder: &str,
) -> (Entity<GitGraphView>, &'a mut VisualTestContext) {
    let checkout = GraphCheckout::new(folder.into()).unwrap();
    cx.add_window_view(|_, cx| GitGraphView::new(local(), checkout, look(), Connection::Online, cx))
}

/// Polls the view until it is no longer loading, for at most ten seconds.
fn settle(view: &Entity<GitGraphView>, cx: &mut VisualTestContext) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.poll(cx);
            view.is_loading()
        })
    }) {
        assert!(Instant::now() < deadline, "the worker never answered");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[gpui::test]
fn old_generation_replies_are_dropped(cx: &mut TestAppContext) {
    let (view, cx) = view_of(cx, "/nonexistent/graph-test");
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let first = view.generation();
            view.reload(cx);
            assert!(view.generation() > first);
            let stale = Arc::new(History::new(&[commit("a", &[])], &[], None, None, false));
            assert!(!view.receive(first, Reply::History(stale), cx));
            assert!(view.history().is_none());
            let fresh = Arc::new(History::new(&[commit("b", &[])], &[], None, None, false));
            assert!(view.receive(view.generation(), Reply::History(fresh), cx));
            assert_eq!(view.history().unwrap().commits[0].oid, "b");
            // A stale failure does not replace the fresh history either.
            assert!(!view.receive(first, Reply::Failed(Arc::new(Error::NotARepository)), cx));
            assert!(view.error().is_none());
        })
    });
}

#[cfg(unix)]
#[gpui::test]
fn view_shows_loading_then_rows_then_errors(cx: &mut TestAppContext) {
    let fixture = Fixture::with_commits(3);
    fixture.write("file.txt", "uncommitted\n");
    let (view, cx) = view_of(cx, fixture.path().to_str().unwrap());
    assert!(view.read_with(cx, |view, _| view.is_loading()));
    settle(&view, cx);
    let rows = view.read_with(cx, |view, _| {
        let history = view.history().expect("rows").clone();
        assert!(view.error().is_none());
        history
            .commits
            .iter()
            .map(|c| c.oid.clone())
            .collect::<Vec<_>>()
    });
    assert_eq!(rows.len(), 4, "three commits under the uncommitted changes");
    assert_eq!(rows[0], WORKTREE_OID);
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("graph-row-0").is_some());
    assert!(cx.debug_bounds("graph-row-3").is_some());

    let outside = fixture.temp.path().join("home");
    let checkout = GraphCheckout::new(outside.to_str().unwrap().into()).unwrap();
    let failed = cx.update(|_, cx| {
        cx.new(|cx| GitGraphView::new(local(), checkout, look(), Connection::Online, cx))
    });
    settle(&failed, cx);
    failed.read_with(cx, |view, _| {
        assert!(view.history().is_none());
        let error = view.error().expect("an error");
        assert!(matches!(**error, Error::NotARepository), "{error:?}");
        assert_eq!(error_title(error, None), "Not a Git repository");
    });
}

#[test]
fn guarded_checkout_paths() {
    let long = format!("/{}", "a".repeat(4096));
    for (path, ok) in [
        ("/repo", true),
        ("repo", false),
        ("", false),
        ("/re\npo", false),
        ("/re\u{1b}po", false),
        (long.as_str(), false),
    ] {
        assert_eq!(GraphCheckout::new(path.into()).is_ok(), ok, "{path:?}");
        assert_eq!(checkout_path_ok(path), ok, "{path:?}");
    }
    assert_eq!(
        GraphCheckout::new("/work/app/".into()).unwrap().name(),
        "app"
    );
    let saved: Result<GraphCheckout, _> = serde_json::from_str(r#"{"checkout":"relative"}"#);
    assert!(saved.is_err());
}

#[test]
fn the_worktree_row_sits_on_head() {
    let counts = WorktreeCounts {
        staged: 1,
        unstaged: 2,
        ..Default::default()
    };
    let commits = [commit("a2", &["a1"]), commit("a1", &[])];
    let history = History::new(&commits, &[], Some("a2".into()), Some(counts), false);
    assert_eq!(history.row_of(WORKTREE_OID), Some(0));
    assert_eq!(history.commits[0].parents, ["a2"]);
    assert_eq!(history.commits[0].description, "1 staged, 2 unstaged");
    assert_eq!(history.row_of("a2"), Some(1));
    // No changes, no row.
    let clean = History::new(
        &commits,
        &[],
        Some("a2".into()),
        Some(WorktreeCounts::default()),
        false,
    );
    assert_eq!(clean.row_of(WORKTREE_OID), None);
}

#[test]
fn segments_in_finds_the_lines_crossing_a_window() {
    // A long lane from "top" to n299 passes every row of a 300-commit line.
    let names: Vec<String> = (0..300).map(|index| format!("n{index}")).collect();
    let mut commits = vec![commit("top", &["n299"])];
    for (index, name) in names.iter().enumerate() {
        let parents: Vec<&str> = names
            .get(index + 1)
            .map(String::as_str)
            .into_iter()
            .collect();
        commits.push(commit(name, &parents));
    }
    let history = History::new(&commits, &[], None, None, false);
    let visible: Vec<_> = history.segments_in(150..160).collect();
    let top = history.layout.nodes[0].lane;
    for row in 150..160 {
        assert!(
            visible
                .iter()
                .any(|s| s.from.lane == top && s.from.row <= row && s.to.row > row),
            "row {row} of the long lane"
        );
    }
    assert!(
        visible
            .iter()
            .all(|s| s.from.row.max(s.to.row) >= 149 && s.from.row.min(s.to.row) < 160)
    );
    assert!(visible.len() < 40, "{}", visible.len());
}
