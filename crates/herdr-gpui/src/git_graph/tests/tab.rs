//! A workspace's Git graph opens in a tab of its own, once, on hosts that run
//! scripts, and a restored tab gets its view back.

use crate::{
    HerdrWindow,
    browser::{Location, Store, Tab, scope},
    controls::Command,
    git_graph::GraphCheckout,
    sidebar::layout_tests::{fixture_window, snapshot},
};
use gpui::{Entity, TestAppContext, VisualTestContext};
use herdr_client::ConnectTarget;
use std::sync::Arc;

/// A window on the local host, showing workspace `w0`, whose tabs start in
/// `/tmp`.
fn window(cx: &mut TestAppContext) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        let mut shown = snapshot(4);
        shown.focused_workspace_id = Some("w0".into());
        shown.focused_tab_id = Some("t0".into());
        view.live.snapshot = Some(Arc::new(shown));
        view.endpoints[0].connection.target = ConnectTarget::Local;
        view
    })
}

fn graph_tabs(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext) -> Vec<Tab> {
    cx.update(|_, cx| {
        let tab_scope = scope(&view.read(cx).endpoints[0]);
        cx.try_global::<Store>().map_or_else(Vec::new, |store| {
            store
                .in_workspace(&tab_scope, "w0")
                .filter(|tab| matches!(tab.location, Some(Location::GitGraph { .. })))
                .cloned()
                .collect()
        })
    })
}

fn run(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext, command: Command) {
    cx.update(|window, cx| view.update(cx, |view, cx| view.command(command, window, cx)));
}

#[gpui::test]
fn command_opens_one_graph_per_workspace(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    run(&view, cx, Command::OpenGitGraph);
    let tabs = graph_tabs(&view, cx);
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].title, "Git graph");
    let location = tabs[0].location.clone().unwrap();
    assert_eq!(location.icon(), Some("icons/git-graph.svg"));
    assert_eq!(
        location,
        Location::GitGraph {
            repo: GraphCheckout::new("/tmp".into()).unwrap()
        }
    );

    // Focus elsewhere, then ask again: the same tab comes back, focused.
    cx.update(|window, cx| window.blur(cx));
    run(&view, cx, Command::OpenGitGraph);
    assert_eq!(graph_tabs(&view, cx).len(), 1);
    cx.update(|window, cx| {
        let view = view.read(cx);
        assert_eq!(view.git_graphs.len(), 1);
        let graph = view.git_graphs.get(&tabs[0].id).unwrap();
        assert!(graph.view().read(cx).focus_handle.is_focused(window));
    });
}

#[gpui::test]
fn socket_hosts_cannot_open_a_graph(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.endpoints[0].connection.target = ConnectTarget::Socket("/unused.sock".into());
        })
    });
    run(&view, cx, Command::OpenGitGraph);
    assert!(graph_tabs(&view, cx).is_empty());
    let flash = view.read_with(cx, |view, _| {
        view.flash.as_ref().map(|(flash, _)| flash.text.to_string())
    });
    assert_eq!(
        flash.as_deref(),
        Some("The Git graph needs a local or SSH host")
    );
    assert!(!view.read_with(cx, |view, _| view.can_open_git_graph()));
}

#[gpui::test]
fn restored_tabs_get_their_view(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    let id = cx.update(|_, cx| {
        let tab_scope = scope(&view.read(cx).endpoints[0]);
        Store::update(cx, |store| {
            store.open(
                tab_scope,
                "w0",
                Some(Location::GitGraph {
                    repo: GraphCheckout::new("/nonexistent/graph-test".into()).unwrap(),
                }),
                None,
            )
        })
        .unwrap()
    });
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            assert!(view.git_graphs.is_empty());
            view.poll_git_graphs(window, cx);
            assert!(view.git_graphs.contains_key(&id));
            view.poll_git_graphs(window, cx);
            assert_eq!(view.git_graphs.len(), 1);
        })
    });
    cx.update(|window, cx| {
        Store::update(cx, |store| store.close(id));
        view.update(cx, |view, cx| {
            view.poll_git_graphs(window, cx);
            assert!(view.git_graphs.is_empty());
        })
    });
}
