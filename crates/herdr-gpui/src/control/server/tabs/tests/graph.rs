//! `graph` opens the caller's workspace's Git graph and `graph close` closes
//! it, refusing a workspace no window shows.
use super::windows::{queued, socket_dir};
use super::*;
use herdr_client::ConnectTarget;

/// Answers `op` for `caller` against the window, letting arrivals land.
fn answer(
    server: &socket::Server,
    op: TabOp,
    caller: Caller,
    cx: &mut VisualTestContext,
) -> Response {
    let (call, incoming, waiting) = queued(server, op, caller);
    let mut watches = Watches::default();
    cx.cx.update(|cx| {
        serve(call, incoming, &mut watches, cx);
        watches.serve(cx);
    });
    waiting.join().unwrap().unwrap()
}

fn graphs(cx: &mut VisualTestContext) -> Vec<TabInfo> {
    cx.cx.update(|cx| {
        cx.try_global::<Store>().map_or_else(Vec::new, |store| {
            store
                .tabs()
                .map(info)
                .filter(|tab| tab.kind == TabKind::Graph)
                .collect()
        })
    })
}

#[gpui::test]
fn graph_open_opens_the_callers_workspace_graph(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx);
    cx.cx.update(|cx| {
        view.update(cx, |view, _| {
            view.endpoints[0].connection.target = ConnectTarget::Local;
        })
    });
    let directory = socket_dir();
    let server = socket::Server::bind(&directory.path().join("control.sock")).unwrap();
    let Response::Tab {
        reply: Reply::Opened { id, workspace },
    } = answer(&server, TabOp::Graph, Caller::default(), cx)
    else {
        panic!("the graph did not open");
    };
    assert_eq!(workspace, "w0");
    let opened = graphs(cx);
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].id, id);
    assert_eq!(opened[0].kind.name(), "graph");
    // Asked again, the same tab answers.
    let Response::Tab {
        reply: Reply::Opened { id: again, .. },
    } = answer(&server, TabOp::Graph, Caller::default(), cx)
    else {
        panic!("the graph did not open");
    };
    assert_eq!(again, id);
}

#[gpui::test]
fn graph_close_closes_it(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx);
    cx.cx.update(|cx| {
        view.update(cx, |view, _| {
            view.endpoints[0].connection.target = ConnectTarget::Local;
        })
    });
    let directory = socket_dir();
    let server = socket::Server::bind(&directory.path().join("control.sock")).unwrap();
    answer(&server, TabOp::Graph, Caller::default(), cx);
    assert_eq!(graphs(cx).len(), 1);
    assert!(matches!(
        answer(&server, TabOp::GraphClose, Caller::default(), cx),
        Response::Done { .. }
    ));
    assert!(graphs(cx).is_empty());
    // Closing what is not open is not an error.
    assert!(matches!(
        answer(&server, TabOp::GraphClose, Caller::default(), cx),
        Response::Done { .. }
    ));
}

#[gpui::test]
fn graph_refuses_another_sessions_window(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = window(cx);
    let directory = socket_dir();
    let server = socket::Server::bind(&directory.path().join("control.sock")).unwrap();
    for op in [TabOp::Graph, TabOp::GraphClose] {
        let caller = Caller {
            workspace_id: Some("w_missing".into()),
            ..Caller::default()
        };
        let Response::Error { code, message } = answer(&server, op.clone(), caller, cx) else {
            panic!("{op:?} was not refused");
        };
        assert_eq!(code, ErrorCode::WorkspaceNotFound);
        let error = crate::Error::ControlRejected { code, message };
        assert_eq!(
            crate::control::client::status(&error),
            crate::control::client::exit::NOT_FOUND
        );
    }
    assert!(graphs(cx).is_empty());
}
