//! Closing a tab or a pane asks in the redesigned dialog: a red heading with
//! its cross, the checkbox, and Cancel before the red Close.

use super::*;
use gpui::{Modifiers, px, size};

#[gpui::test]
fn close_confirmation_is_a_danger_dialog(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(
            serde_json::from_str(include_str!(
                "../../../../herdr-protocol/tests/fixtures/endpoint-snapshot-v1.json"
            ))
            .unwrap(),
        ));
        view.config.confirm_close_pane = true;
        view
    });
    cx.simulate_resize(size(px(900.), px(700.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_close_confirmation(Command::ClosePane, window, cx)
        });
        window.draw(cx).clear(cx);
    });
    let header = cx.debug_bounds("close-header").unwrap();
    let cross = cx.debug_bounds("close-dismiss").unwrap();
    assert!(header.contains(&cross.center()), "{cross:?} in {header:?}");
    let cancel = cx.debug_bounds("close-cancel").unwrap();
    let confirm = cx.debug_bounds("close-confirm").unwrap();
    assert!(confirm.left() > cancel.right(), "{cancel:?} {confirm:?}");
    assert_eq!(cancel.size.height, px(30.));
    assert_eq!(confirm.size.height, px(30.));
    let option = cx.debug_bounds("close-do-not-ask").unwrap();
    cx.simulate_click(option.center(), Modifiers::default());
    assert!(view.read_with(cx, |view, _| {
        view.menu.close.as_ref().unwrap().do_not_ask_again
    }));
}
