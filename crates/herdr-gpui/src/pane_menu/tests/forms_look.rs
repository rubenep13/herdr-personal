//! Renaming a pane keeps the pane menu's panel, its field in Settings' frame.

use super::*;

#[gpui::test]
fn rename_field_keeps_ime_inside_its_frame(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot()));
        view
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.open_pane_menu("inactive", point(px(20.), px(20.)), window, cx);
            v.activate_pane_menu(Action::Rename, window, cx);
        });
        window.draw(cx).clear(cx);
    });
    let frame = cx.debug_bounds("pane-rename-field").unwrap();
    assert_eq!(frame.size.height, px(30.));
    let input = cx.debug_bounds("pane-rename-input").unwrap();
    assert!(
        frame.contains(&input.origin) && input.bottom_right().x <= frame.right(),
        "{input:?} in {frame:?}"
    );
    let submit = cx.debug_bounds("pane-rename-submit").unwrap();
    assert_eq!(submit.size.height, px(26.));
}
