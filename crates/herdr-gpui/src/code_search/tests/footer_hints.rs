//! Go to File's key hints drop out whole when its footer is narrow.
// Not a glob, for the parent's reason: `gpui::*` would shadow `#[test]`.
use super::{Mode, checkout, window};
use gpui::{TestAppContext, px, size};

#[gpui::test]
fn go_to_file_hints_drop_whole(cx: &mut TestAppContext) {
    let dir = checkout();
    let (view, cx) = window(cx, Some(dir.path()));
    view.update(cx, |view, _| view.config.ui.size = 18.);
    cx.simulate_resize(size(px(680.), px(560.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_code_search(Mode::Files, window, cx)
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    crate::sidebar::layout_tests::whole_or_gone(cx, "code-search-hints", "code-search-hint", 5);
}
