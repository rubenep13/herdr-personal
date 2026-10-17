//! The Git graph opens from a group's "…" menu when the workspace has a
//! folder on a host that runs scripts.
use super::*;

#[gpui::test]
fn the_group_menu_offers_the_git_graph_where_scripts_run(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    let group = groups(&view, cx)[0];
    // The fixture's explicit socket cannot run scripts.
    assert!(!actions(&view, cx, group).contains(&Action::GitGraph));
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.endpoints[0].connection.target = herdr_client::ConnectTarget::Local;
        })
    });
    let offered = actions(&view, cx, group);
    assert_eq!(Action::GitGraph.label(), "Git Graph");
    assert_eq!(Action::GitGraph.icon(), Some("icons/git-graph.svg"));
    let at = offered.iter().position(|a| *a == Action::GitGraph).unwrap();
    assert!(offered[..at].iter().all(|a| a.section() == 0));
    run(&view, cx, group, Action::GitGraph);
    view.read_with(cx, |view, _| assert_eq!(view.git_graphs.len(), 1));
}
