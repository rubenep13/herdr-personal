use super::*;
use crate::browser::{Shown, Store};
use gpui::{App, Entity, Window};

/// A window on one connection whose workspace `w1` has the focused `w1:p1`
/// and `w1:p2` in tab `w1:t1`, and whose workspace `w2` has `w2:p1` in tab
/// `w2:t1`. A page is in front of the terminal in `in_front_of`.
fn page_in_front<'a>(
    cx: &'a mut gpui::TestAppContext,
    in_front_of: &str,
) -> (Entity<HerdrWindow>, Server, &'a mut gpui::VisualTestContext) {
    let (endpoint, server) = connected_endpoint("reveal");
    let (fixture, cx) = cx.add_window_view(|window, cx| {
        Fixture(cx.new(|cx| crate::sidebar::layout_tests::fixture_window(window, cx)))
    });
    let view = fixture.read_with(cx, |fixture, _| fixture.0.clone());
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.endpoints = vec![endpoint];
            view.options = ConnectOptions::default();
            view.reset_selected(cx);
            project_until(view, cx, "initial surface", HerdrWindow::input_ready);
            let mut shown = (**view.live.snapshot.as_ref().unwrap()).clone();
            let mut second = shown.panes[0].clone();
            second.pane_id = "w1:p2".into();
            second.focused = false;
            let mut workspace = shown.workspaces[0].clone();
            workspace.workspace_id = "w2".into();
            workspace.number = 2;
            workspace.label = "other".into();
            workspace.active_tab_id = "w2:t1".into();
            workspace.focused = false;
            let mut tab = shown.tabs[0].clone();
            tab.tab_id = "w2:t1".into();
            tab.workspace_id = "w2".into();
            tab.focused = false;
            let mut elsewhere = second.clone();
            elsewhere.pane_id = "w2:p1".into();
            elsewhere.workspace_id = "w2".into();
            elsewhere.tab_id = "w2:t1".into();
            shown.panes.extend([second, elsewhere]);
            shown.workspaces.push(workspace);
            shown.tabs.push(tab);
            view.live.snapshot = Some(Arc::new(shown));
            let home = view.live.snapshot.as_ref().unwrap().focused_pane_id.clone();
            arrive(view, in_front_of, window, cx);
            let (scope, workspace) = view.browser_key().unwrap();
            let page =
                Store::update(cx, |store| store.open(scope, &workspace, None, None)).unwrap();
            view.show_browser_tab(page, window, cx);
            arrive(view, home.as_deref().unwrap(), window, cx);
        });
    });
    (view, server, cx)
}

/// The daemon moves its focus to `pane`, and the window's tick follows:
/// the group follows the tab, then a terminal asked for comes out.
fn arrive(view: &mut HerdrWindow, pane: &str, window: &mut Window, cx: &mut Context<HerdrWindow>) {
    let mut next = (**view.live.snapshot.as_ref().unwrap()).clone();
    let old = (
        next.focused_workspace_id.clone(),
        next.focused_tab_id.clone(),
    );
    let found = next
        .panes
        .iter()
        .find(|p| p.pane_id == pane)
        .unwrap()
        .clone();
    next.revision += 1;
    next.focused_workspace_id = Some(found.workspace_id.clone());
    next.focused_tab_id = Some(found.tab_id.clone());
    next.focused_pane_id = Some(pane.into());
    {
        let mut state = view.endpoints[0].connection.inbox.lock().unwrap();
        state.apply(ClientEvent::Snapshot(Arc::new(next.clone())));
        state.apply(ClientEvent::Surface(surface(&next)));
    }
    project_until(view, cx, "the daemon's focus", |view| {
        view.live
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.focused_pane_id.as_deref() == Some(pane))
    });
    if old != (Some(found.workspace_id.clone()), Some(found.tab_id.clone())) {
        view.terminal_focus_moved(
            &found.tab_id,
            old.0.as_ref() != Some(&found.workspace_id),
            cx,
        );
    }
    view.reveal_terminal(window, cx);
}

fn shown(view: &HerdrWindow, cx: &App) -> Shown {
    view.group_shown(view.primary_group().unwrap(), cx)
}

#[gpui::test]
fn an_agent_in_the_focused_tab_comes_out_from_behind_a_page(cx: &mut gpui::TestAppContext) {
    let (view, _server, cx) = page_in_front(cx, "w1:p1");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            assert!(matches!(shown(view, cx), Shown::Page(_)));
            assert!(view.navigate_endpoint("reveal", NavigationTarget::Pane("w1:p2"), cx));
            assert_eq!(view.reveal, Some(NavigationTarget::Pane("w1:p2".into())));
            arrive(view, "w1:p2", window, cx);
            assert_eq!(shown(view, cx), Shown::Terminal);
            assert_eq!(view.reveal, None);
        });
    });
}

#[gpui::test]
fn the_focused_agent_comes_out_without_a_move(cx: &mut gpui::TestAppContext) {
    let (view, _server, cx) = page_in_front(cx, "w1:p1");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            assert!(view.navigate_endpoint("reveal", NavigationTarget::Pane("w1:p1"), cx));
            view.reveal_terminal(window, cx);
            assert_eq!(shown(view, cx), Shown::Terminal);
        });
    });
}

#[gpui::test]
fn an_agent_in_another_workspace_comes_out_from_behind_its_page(cx: &mut gpui::TestAppContext) {
    let (view, _server, cx) = page_in_front(cx, "w2:p1");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            assert_eq!(shown(view, cx), Shown::Terminal, "w1 has no page");
            assert!(view.navigate_endpoint("reveal", NavigationTarget::Pane("w2:p1"), cx));
            arrive(view, "w2:p1", window, cx);
            assert_eq!(shown(view, cx), Shown::Terminal);
        });
    });
}

#[gpui::test]
fn choosing_a_workspace_keeps_its_page(cx: &mut gpui::TestAppContext) {
    let (view, _server, cx) = page_in_front(cx, "w2:p1");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            assert!(view.navigate_endpoint("reveal", NavigationTarget::Workspace("w2"), cx));
            assert_eq!(view.reveal, None);
            arrive(view, "w2:p1", window, cx);
            assert!(matches!(shown(view, cx), Shown::Page(_)));
        });
    });
}

#[gpui::test]
fn a_refused_navigation_leaves_nothing_pending(cx: &mut gpui::TestAppContext) {
    let (view, _server, cx) = page_in_front(cx, "w1:p1");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.menu.page = Some(crate::menu::Page::Tab);
            assert!(!view.navigate_endpoint("reveal", NavigationTarget::Pane("w1:p2"), cx));
            assert_eq!(view.reveal, None);
            view.menu.page = None;
            // The focus moves there some other way, such as Herdr's CLI.
            arrive(view, "w1:p2", window, cx);
            assert!(matches!(shown(view, cx), Shown::Page(_)));
        });
    });
}

#[gpui::test]
fn a_navigation_that_lands_elsewhere_forgets_its_agent(cx: &mut gpui::TestAppContext) {
    let (view, _server, cx) = page_in_front(cx, "w1:p1");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.ask_reveal(NavigationTarget::Pane("w1:p2"));
            // No navigation is in flight, so the agent is not on its way.
            view.reveal_terminal(window, cx);
            assert_eq!(view.reveal, None);
            arrive(view, "w1:p2", window, cx);
            assert!(matches!(shown(view, cx), Shown::Page(_)));
        });
    });
}

#[gpui::test]
fn the_palette_brings_an_agent_out_from_behind_a_page(cx: &mut gpui::TestAppContext) {
    let (view, mut server, cx) = page_in_front(cx, "w1:p1");
    let key = |key: &str| gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
        is_held: false,
        prefer_character_input: false,
    };
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.command(Command::WorkspacePicker, window, cx);
            // Go To lists the workspace, then its panes: a lone tab has no row.
            for _ in 0..2 {
                view.palette_key(&key("down"), window, cx);
            }
            view.palette_key(&key("enter"), window, cx);
            assert_eq!(view.reveal, Some(NavigationTarget::Pane("w1:p2".into())));
            arrive(view, "w1:p2", window, cx);
            assert_eq!(shown(view, cx), Shown::Terminal);
        });
    });
    let ClientMessage::ClientShellEndpointRequest { request, .. } = server.receive() else {
        panic!("missing the palette's navigation");
    };
    let request: serde_json::Value = serde_json::from_str(&request).unwrap();
    assert_eq!(request["params"]["pane_id"], "w1:p2");
}

#[gpui::test]
fn an_agent_shortcut_brings_its_terminal_out(cx: &mut gpui::TestAppContext) {
    let (view, _server, cx) = page_in_front(cx, "w1:p1");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            arrive(view, "w1:p2", window, cx);
            // The fixture's one agent runs in `w1:p1`.
            view.command(Command::AgentNumber(1), window, cx);
            assert_eq!(view.reveal, Some(NavigationTarget::Pane("w1:p1".into())));
            arrive(view, "w1:p1", window, cx);
            assert_eq!(shown(view, cx), Shown::Terminal);
        });
    });
}

#[gpui::test]
fn a_notification_brings_its_agent_out(cx: &mut gpui::TestAppContext) {
    let (view, _server, cx) = page_in_front(cx, "w1:p1");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            let mut wire = crate::notifications::tests::notification("Navigate");
            wire.workspace_id = Some("w1".into());
            wire.tab_id = Some("w1:t1".into());
            wire.pane_id = Some("w1:p2".into());
            let notice = crate::notifications::Notice::new(wire, Instant::now())
                .with_snapshot(view.live.snapshot.as_deref())
                .preview();
            view.endpoints[0].toasts.receive([notice]);
            view.tick_toasts(false, Instant::now());
            view.command(Command::OpenNotificationTarget, window, cx);
            assert_eq!(view.reveal, Some(NavigationTarget::Pane("w1:p2".into())));
            arrive(view, "w1:p2", window, cx);
            assert_eq!(shown(view, cx), Shown::Terminal);
        });
    });
}
