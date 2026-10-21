//! The GitHub sign-in's body in the redesigned look: its status as a note,
//! the one-time code in a well.

use gpui::{TestAppContext, px, size};

fn open(waiting: bool, cx: &mut TestAppContext) -> &mut gpui::VisualTestContext {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(900.), px(700.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| view.github_fixture(waiting, window, cx));
        window.draw(cx).clear(cx);
    });
    cx
}

#[gpui::test]
fn the_github_status_is_a_note(cx: &mut TestAppContext) {
    let cx = open(false, cx);
    assert!(cx.debug_bounds("github-status").is_some());
    assert!(cx.debug_bounds("github-status-icon").is_some());
}

#[gpui::test]
fn the_device_code_sits_in_a_well(cx: &mut TestAppContext) {
    let cx = open(true, cx);
    let well = cx.debug_bounds("github-code").unwrap();
    let code = cx.debug_bounds("github-device-code").unwrap();
    assert!(well.contains(&code.origin), "{code:?} in {well:?}");
}
