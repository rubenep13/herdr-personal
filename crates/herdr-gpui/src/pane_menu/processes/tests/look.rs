//! Ending processes asks in the pane menu's panel, with small buttons and
//! the confirmation in red.

use super::*;

#[gpui::test]
fn kill_confirmation_ends_in_danger(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        let mut snapshot: ClientShellSnapshot = serde_json::from_str(include_str!(
            "../../../../../herdr-protocol/tests/fixtures/endpoint-snapshot-v1.json"
        ))
        .unwrap();
        let mut pane = snapshot.panes[0].clone();
        pane.pane_id = "inactive".into();
        snapshot.panes.push(pane);
        view.live.snapshot = Some(Arc::new(snapshot));
        view
    });
    cx.simulate_resize(size(px(900.), px(700.)));
    let (watch, updates, _commands) = Watch::fake();
    cx.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.open_pane_menu("inactive", point(px(10.), px(10.)), window, cx);
            v.menu.pane.as_mut().unwrap().daemon = Some(Daemon::Default);
            v.show_pane_processes(watch, cx);
        });
        window.draw(cx).clear(cx);
    });
    updates.send(listing(&[(10, 0), (20, 1)])).unwrap();
    let tick = |cx: &mut VisualTestContext| {
        cx.update(|window, cx| {
            view.update(cx, |v, cx| v.poll_pane_processes(cx));
            window.draw(cx).clear(cx);
        })
    };
    tick(cx);
    let end = cx.debug_bounds("pane-processes-end").unwrap();
    assert_eq!(end.size.height, px(26.));
    // The pane's own process (the first row) cannot be chosen.
    cx.simulate_keystrokes("down space down space enter");
    tick(cx);
    assert!(view.read_with(cx, |v, _| v.menu.page == Some(Page::KillProcesses)));
    let header = cx.debug_bounds("pane-kill-header").unwrap();
    let cancel = cx.debug_bounds("pane-processes-cancel").unwrap();
    let confirm = cx.debug_bounds("pane-processes-confirm").unwrap();
    assert!(header.bottom() <= cancel.top());
    assert!(confirm.left() > cancel.right(), "{cancel:?} {confirm:?}");
    assert_eq!(cancel.size.height, px(26.));
    assert_eq!(confirm.size.height, px(26.));
}
