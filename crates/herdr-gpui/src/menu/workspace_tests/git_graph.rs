//! The workspace popover opens a Git workspace's graph on a host that runs
//! scripts.
use super::*;

#[gpui::test]
fn a_git_workspace_offers_its_graph(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(sidebar::layout_tests::fixture_window);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.live.status = crate::state::ConnectionStatus::Connected;
            view.endpoints[0].connection.target = herdr_client::ConnectTarget::Local;
            view.open_workspace_menu("w3", Default::default(), window, cx);
        })
    });
    let items = cx.update(|_, cx| view.read(cx).workspace_items());
    assert!(
        items.iter().any(
            |(action, label)| *action == WorkspaceMenuAction::GitGraph && *label == "Git graph"
        ),
        "{items:?}"
    );
    // A socket host cannot run the graph's scripts.
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.dismiss_menu(window, cx);
            view.endpoints[0].connection.target =
                herdr_client::ConnectTarget::Socket("/unused.sock".into());
            view.open_workspace_menu("w3", Default::default(), window, cx);
        })
    });
    let items = cx.update(|_, cx| view.read(cx).workspace_items());
    assert!(
        !items
            .iter()
            .any(|(action, _)| *action == WorkspaceMenuAction::GitGraph)
    );
}
