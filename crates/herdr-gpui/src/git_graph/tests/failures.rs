//! Failures leave the graph standing: a failed search, or a failed detail
//! whose panel was closed, never clears the rows, and a page that failed to
//! load is not asked for again on every frame.

use super::*;
use crate::git_graph::{
    GraphCheckout,
    error::Error,
    history::History,
    view::{Connection, GitGraphView},
    worker::{Reply, Request},
};
use crate::teleport::Host;
use gpui::{TestAppContext, px, size};
use herdr_client::ConnectTarget;
use std::sync::Arc;

fn failed() -> Reply {
    Reply::Failed(Arc::new(Error::WorkerStopped))
}

#[gpui::test]
fn a_failed_search_or_closed_panel_never_clears_the_graph(cx: &mut TestAppContext) {
    let checkout = GraphCheckout::new("/nonexistent/graph-test".into()).unwrap();
    let local = Host::new(&ConnectTarget::Local).unwrap();
    let (view, cx) = cx.add_window_view(|_, cx| {
        GitGraphView::new(Ok(local), checkout, look(), Connection::Online, cx)
    });
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let first = view.generation();
            let history = History::new(&line(5), &[], None, None, false);
            assert!(view.receive(first, Reply::History(Arc::new(history)), cx));

            view.start_search("x", cx);
            let search = view.last_search();
            view.receive(search, failed(), cx);
            assert!(view.history().is_some(), "a failed search keeps the rows");
            assert!(view.error().is_none());

            view.open_detail(0, cx);
            let detail = view.last_detail();
            view.close_detail_for_test(cx);
            view.receive(detail, failed(), cx);
            assert!(
                view.history().is_some(),
                "a closed panel's failure keeps the rows"
            );
            assert!(view.error().is_none());
        })
    });
}

#[gpui::test]
fn a_failed_page_is_not_asked_again_until_a_reload(cx: &mut TestAppContext) {
    let checkout = GraphCheckout::new("/nonexistent/graph-test".into()).unwrap();
    let local = Host::new(&ConnectTarget::Local).unwrap();
    let (view, cx) = cx.add_window_view(|_, cx| {
        GitGraphView::new(Ok(local), checkout, look(), Connection::Online, cx)
    });
    cx.simulate_resize(size(px(1000.), px(600.)));
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let first = view.generation();
            let history = History::new(&line(200), &[], None, None, true);
            assert!(view.receive(first, Reply::History(Arc::new(history)), cx));
            view.scroll_to_row(190);
        })
    });
    draw(cx);
    let (more, request) = view.read_with(cx, |view, _| view.last_request());
    assert_eq!(request, Request::More);
    cx.update(|_, cx| view.update(cx, |view, cx| view.receive(more, failed(), cx)));
    for _ in 0..3 {
        draw(cx);
    }
    assert_eq!(
        view.read_with(cx, |view, _| view.generation()),
        more,
        "no new page is asked for after a failure"
    );
    assert!(view.read_with(cx, |view, _| view.history().is_some()));
    assert!(drawn(cx, "graph-banner"), "the failure is said");
    // A reload clears it, and paging resumes.
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.reload(cx);
            let (load, _) = view.last_request();
            let history = History::new(&line(200), &[], None, None, true);
            assert!(view.receive(load, Reply::History(Arc::new(history)), cx));
            view.scroll_to_row(195);
        })
    });
    draw(cx);
    assert!(!drawn(cx, "graph-banner"));
    assert_eq!(
        view.read_with(cx, |view, _| view.last_request().1),
        Request::More
    );
}
