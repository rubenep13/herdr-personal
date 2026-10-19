//! A host's terminal theme picker in the redesigned menus' look.
use super::*;

#[gpui::test]
fn the_device_theme_picker_matches(cx: &mut TestAppContext) {
    if cfg!(windows) {
        return;
    }
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
        let mut view = fixture_window(window, cx);
        add_host(&mut view);
        view.selected_endpoint = 1;
        view
    });
    cx.simulate_resize(size(px(1000.), px(800.)));
    view.update_in(cx, |view, window, cx| {
        view.open_host_menu(HOST, point(px(10.), px(10.)), window, cx)
    });
    cx.simulate_keystrokes("down down down down enter");
    cx.run_until_parked();
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    let first = cx.debug_bounds("device-theme-row-0").unwrap();
    assert_eq!(first.size.height, px(32.));
    assert!(
        cx.debug_bounds("device-theme-row-0-icon").is_some(),
        "Same as app links"
    );
    let close = cx.debug_bounds("device-theme-close").unwrap();
    assert_eq!((close.size.width, close.size.height), (px(28.), px(28.)));
    assert!(cx.debug_bounds("device-theme-count").is_some());
    assert!(cx.debug_bounds("device-theme-status-note").is_some());
}
