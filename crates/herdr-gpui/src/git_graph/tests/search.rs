//! Searching commits by message, author or hash: `/` opens the field, the
//! results walk with n and N (reading pages if needed), matching rows are
//! marked, a capped list says so, and Escape closes it all.

use super::*;
use crate::git_graph::{
    history::History,
    search::{SEARCH_LIMIT, SearchHit, results_label},
    view::{Connection, GitGraphView},
    worker::{Reply, Request},
};
use crate::teleport::Host;
use gpui::{TestAppContext, VisualTestContext, px, size};
use herdr_client::ConnectTarget;
use std::sync::Arc;

fn hit(oid: &str) -> SearchHit {
    SearchHit {
        oid: oid.into(),
        subject: format!("subject of {oid}"),
        author: "Ada".into(),
        authored: 1_700_000_000,
    }
}

fn focus(view: &gpui::Entity<GitGraphView>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let handle = view.read(cx).focus_handle.clone();
        window.focus(&handle, cx);
    });
}

#[cfg(unix)]
#[test]
fn search_matches_message_author_and_sha() {
    use crate::git_graph::search::search;
    let fixture = Fixture::with_commits(2);
    fixture.write("fix.txt", "x\n");
    let fix = fixture.commit("fix: keep lanes stable");
    fixture.write("graph.txt", "x\n");
    fixture.git(&["add", "-A"]);
    fixture.git(&[
        "-c",
        "user.name=Rubén Esteban",
        "commit",
        "-qm",
        "feat: draw the graph",
    ]);
    let ruben = fixture.head();
    let repo = fixture.open();
    let cancelled = AtomicBool::new(false);
    let snapshot = repo.snapshot(&cancelled).unwrap();
    let found = |query: &str| -> Vec<String> {
        let (hits, more) = search(&repo, &snapshot, query, SEARCH_LIMIT, &cancelled).unwrap();
        assert!(!more);
        hits.into_iter().map(|hit| hit.oid).collect()
    };
    assert_eq!(found("FIX"), [fix]);
    assert_eq!(found("rubén"), std::slice::from_ref(&ruben));
    assert_eq!(found(&ruben[..7]), std::slice::from_ref(&ruben));
    assert!(found("nothing like this").is_empty());
    // Every commit is by an author named Test, or by Rubén who committed
    // last: newest first.
    let everyone = found("test");
    assert_eq!(everyone.len(), 4, "{everyone:?}");
    assert_eq!(everyone[0], ruben);
    let (hits, _) = search(&repo, &snapshot, "graph", SEARCH_LIMIT, &cancelled).unwrap();
    assert_eq!(hits[0].subject, "feat: draw the graph");
    assert_eq!(hits[0].author, "Rubén Esteban");
}

#[gpui::test]
fn slash_opens_search_and_escape_closes_it(cx: &mut TestAppContext) {
    let (view, cx) = shown(cx, History::new(&line(5), &[], None, None, false), 600.);
    focus(&view, cx);
    assert!(!drawn(cx, "graph-search-input"));
    cx.simulate_keystrokes("/");
    draw(cx);
    assert!(drawn(cx, "graph-search-input"));
    // Typed letters go to the field, not to the graph's keys.
    cx.simulate_keystrokes("j");
    assert_eq!(view.read_with(cx, |view, _| view.selected_row()), None);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.start_search("c1", cx);
            let reply = Reply::Search {
                query: "c1".into(),
                hits: Arc::new(vec![hit("c1")]),
                more: false,
            };
            assert!(view.receive(0, reply, cx));
            assert!(view.is_match("c1"));
        })
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(!drawn(cx, "graph-search-input"));
    assert!(!view.read_with(cx, |view, _| view.is_match("c1")));
}

#[gpui::test]
fn n_and_shift_n_walk_the_results(cx: &mut TestAppContext) {
    let checkout = crate::git_graph::GraphCheckout::new("/nonexistent/graph-test".into()).unwrap();
    let local = Host::new(&ConnectTarget::Local).unwrap();
    let (view, cx) = cx.add_window_view(|_, cx| {
        GitGraphView::new(Ok(local), checkout, look(), Connection::Online, cx)
    });
    cx.simulate_resize(size(px(1000.), px(600.)));
    let commits = line(30);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let first = view.generation();
            let history = History::new(&commits[..10], &[], None, None, true);
            assert!(view.receive(first, Reply::History(Arc::new(history)), cx));
            view.start_search("x", cx);
            let asked = view.last_search();
            let reply = Reply::Search {
                query: "x".into(),
                hits: Arc::new(vec![hit("c2"), hit("c5"), hit("c25")]),
                more: false,
            };
            assert!(view.receive(asked, reply, cx));
            assert_eq!(view.selected_row(), Some(2), "the first result is selected");
            assert!(view.is_match("c5") && !view.is_match("c3"));
        })
    });
    focus(&view, cx);
    cx.simulate_keystrokes("n");
    assert_eq!(view.read_with(cx, |view, _| view.selected_row()), Some(5));
    cx.simulate_keystrokes("n");
    // c25 is not loaded: a page is read to find it.
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let (more, request) = view.last_request();
            assert_eq!(request, Request::More);
            let history = History::new(&commits, &[], None, None, false);
            assert!(view.receive(more, Reply::History(Arc::new(history)), cx));
        })
    });
    assert_eq!(view.read_with(cx, |view, _| view.selected_row()), Some(25));
    cx.simulate_keystrokes("shift-n");
    assert_eq!(view.read_with(cx, |view, _| view.selected_row()), Some(5));
    cx.simulate_keystrokes("n n");
    assert_eq!(
        view.read_with(cx, |view, _| view.selected_row()),
        Some(2),
        "past the last result comes the first"
    );
}

#[gpui::test]
fn results_list_says_when_there_are_more(cx: &mut TestAppContext) {
    assert_eq!(results_label(200, true), "200+ results");
    assert_eq!(results_label(1, false), "1 result");
    assert_eq!(results_label(3, false), "3 results");
    assert_eq!(results_label(0, false), "No results");
    let (view, cx) = shown(cx, History::new(&line(5), &[], None, None, false), 600.);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.start_search("c", cx);
            let hits: Vec<SearchHit> = (0..SEARCH_LIMIT).map(|n| hit(&format!("x{n}"))).collect();
            let reply = Reply::Search {
                query: "c".into(),
                hits: Arc::new(hits),
                more: true,
            };
            assert!(view.receive(0, reply, cx));
        })
    });
    draw(cx);
    assert!(drawn(cx, "graph-search-count"));
    assert!(drawn(cx, "graph-search-result-0"));
}
