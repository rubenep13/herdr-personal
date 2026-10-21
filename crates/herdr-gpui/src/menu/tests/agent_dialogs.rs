//! Checkpoints, Fan out and Teleport in the redesigned look: each opens with
//! its cross in the heading and its buttons in the shared footer.

use super::*;
use gpui::{Modifiers, TestAppContext, px, size};

fn open(
    action: WorkspaceMenuAction,
    cx: &mut TestAppContext,
) -> (
    gpui::Entity<crate::HerdrWindow>,
    &mut gpui::VisualTestContext,
) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(1000.), px(800.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.live.status = crate::state::ConnectionStatus::Connected;
            view.live.local_daemon_peer = true;
            view.endpoints[0].connection.target = herdr_client::ConnectTarget::Local;
            let snapshot = std::sync::Arc::make_mut(view.live.snapshot.as_mut().unwrap());
            snapshot.workspaces = crate::sidebar::layout_tests::snapshot(7).workspaces;
            for workspace in &mut snapshot.workspaces {
                if let Some(tree) = &mut workspace.worktree {
                    tree.key = "/repo/.git".into();
                }
            }
            view.open_workspace_menu("w4", Default::default(), window, cx);
            view.activate_workspace_menu(action, window, cx);
        });
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (view, cx)
}

#[gpui::test]
fn agent_dialogs_have_a_cross_and_the_shared_footer(cx: &mut TestAppContext) {
    for (action, page, cross, footer) in [
        (
            WorkspaceMenuAction::Checkpoints,
            Page::Checkpoints,
            "checkpoints-dismiss",
            "checkpoints-footer",
        ),
        (
            WorkspaceMenuAction::FanOut,
            Page::FanOut,
            "fan-out-dismiss",
            "fan-out-footer",
        ),
        (
            WorkspaceMenuAction::Teleport,
            Page::Teleport,
            "teleport-dismiss",
            "teleport-footer",
        ),
    ] {
        let (view, cx) = open(action, cx);
        assert_eq!(view.read_with(cx, |view, _| view.menu.page), Some(page));
        let panel = cx.debug_bounds("menu-panel").unwrap();
        let bounds = cx.debug_bounds(cross).unwrap();
        assert!(
            bounds.right() >= panel.right() - px(12.) && bounds.top() <= panel.top() + px(52.),
            "{page:?}: {bounds:?} in {panel:?}"
        );
        let footer = cx.debug_bounds(footer).unwrap();
        assert!(
            (footer.bottom() - panel.bottom()).abs() <= px(2.),
            "{page:?}: {footer:?} {panel:?}"
        );
        cx.simulate_click(bounds.center(), Modifiers::default());
        assert!(
            view.read_with(cx, |view, _| view.menu.page.is_none()),
            "{page:?}"
        );
    }
}
