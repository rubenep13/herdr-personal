//! The palette's key hints drop out whole when its footer is narrow.
use super::*;
use gpui::{px, size};

#[gpui::test]
fn palette_hints_drop_whole(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.config.ui.size = 18.;
        view
    });
    cx.simulate_resize(size(px(680.), px(560.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| view.open_palette(Filter::All, window, cx));
        crate::sidebar::layout_tests::full_draw(window, cx).clear(cx);
    });
    let count = hints::HINTS.len();
    crate::sidebar::layout_tests::whole_or_gone(cx, "palette-hints", "palette-hint", count);
}
