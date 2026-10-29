//! A narrow dialog's key hints drop out whole, from the last, and its
//! buttons stay whole.
use super::*;
use crate::sidebar::layout_tests::whole_or_gone;
use gpui::{px, size};

#[gpui::test]
fn footer_hints_drop_whole(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(
            serde_json::from_str(include_str!(
                "../../../../herdr-protocol/tests/fixtures/endpoint-snapshot-v1.json"
            ))
            .unwrap(),
        ));
        view.config.confirm_close_pane = true;
        view.config.ui.size = 18.;
        view
    });
    cx.simulate_resize(size(px(680.), px(560.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_close_confirmation(Command::ClosePane, window, cx)
        });
        window.draw(cx).clear(cx);
    });
    whole_or_gone(cx, "close-footer-hints", "close-footer-hint", 3);
    let footer = cx.debug_bounds("close-footer").unwrap();
    for button in ["close-cancel", "close-confirm"] {
        let button = cx.debug_bounds(button).unwrap();
        assert!(
            button.left() >= footer.left() && button.right() <= footer.right(),
            "{button:?} in {footer:?}"
        );
    }
    // Squeezed, a hint goes rather than being cut.
    let hints = cx.debug_bounds("close-footer-hints").unwrap();
    let last = cx.debug_bounds("close-footer-hint-2").unwrap();
    assert!(last.top() >= hints.bottom() || last.right() <= hints.right());
}
