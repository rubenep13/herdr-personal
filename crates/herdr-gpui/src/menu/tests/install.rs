use super::*;
use gpui::{Modifiers, TestAppContext, px, size};

#[gpui::test]
fn install_is_a_dialog_with_a_cross_and_its_buttons_on_the_right(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| view.show_install_modal(window, cx));
        window.draw(cx).clear(cx);
    });
    let panel = cx.debug_bounds("menu-panel").unwrap();
    let cross = cx.debug_bounds("install-dismiss").unwrap();
    assert!(panel.contains(&cross.origin), "{cross:?} in {panel:?}");
    assert!(
        cross.right() >= panel.right() - px(12.),
        "{cross:?} in {panel:?}"
    );
    assert!(
        cross.top() <= panel.top() + px(52.),
        "{cross:?} in {panel:?}"
    );
    let dismiss = cx.debug_bounds("menu-dismiss").unwrap();
    let install = cx.debug_bounds("menu-install").unwrap();
    assert!(install.left() > dismiss.right(), "{dismiss:?} {install:?}");
    assert_eq!(dismiss.size.height, px(30.));
    assert_eq!(install.size.height, px(30.));
    cx.simulate_click(cross.center(), Modifiers::default());
    assert!(view.read_with(cx, |view, _| view.menu.page.is_none()));
}
