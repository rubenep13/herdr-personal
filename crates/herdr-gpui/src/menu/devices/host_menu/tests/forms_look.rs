//! A saved device's forms in the redesigned look: Rename device is a
//! dialog with its cross and a Cancel, Remove device stays in the menu's
//! panel and ends in red.

use super::*;

fn open(cx: &mut TestAppContext) -> (gpui::Entity<crate::HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
        let mut view = fixture_window(window, cx);
        add_host(&mut view);
        view
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    view.update_in(cx, |view, window, cx| {
        view.open_host_menu(HOST, point(px(20.), px(20.)), window, cx)
    });
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    (view, cx)
}

#[gpui::test]
fn rename_device_has_a_cross_and_a_cancel(cx: &mut TestAppContext) {
    if cfg!(windows) {
        return;
    }
    let (view, cx) = open(cx);
    view.update_in(cx, |view, window, cx| {
        view.activate_host_menu(Action::Rename, window, cx)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let header = cx.debug_bounds("rename-device-header").unwrap();
    let cross = cx.debug_bounds("rename-device-close").unwrap();
    assert!(header.contains(&cross.center()), "{cross:?} in {header:?}");
    let cancel = cx.debug_bounds("rename-device-cancel").unwrap();
    let submit = cx.debug_bounds("rename-device-submit").unwrap();
    assert!(cancel.right() < submit.left(), "{cancel:?} {submit:?}");
    assert_eq!(submit.size.height, px(30.));
    cx.simulate_click(cancel.center(), Modifiers::default());
    assert!(view.read_with(cx, |view, _| view.menu.page.is_none()));
}

#[gpui::test]
fn remove_device_is_a_danger_form_beside_its_menu(cx: &mut TestAppContext) {
    if cfg!(windows) {
        return;
    }
    let (view, cx) = open(cx);
    let menu = cx.debug_bounds("menu-panel").unwrap();
    view.update_in(cx, |view, window, cx| {
        view.activate_host_menu(Action::Remove, window, cx)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let form = cx.debug_bounds("menu-panel").unwrap();
    assert!(
        (form.origin.x - menu.origin.x).abs() <= px(1.),
        "{menu:?} {form:?}"
    );
    assert!(
        (form.origin.y - menu.origin.y).abs() <= px(1.),
        "{menu:?} {form:?}"
    );
    let submit = cx.debug_bounds("remove-device-submit").unwrap();
    assert_eq!(submit.size.height, px(26.));
    assert!(cx.debug_bounds("remove-device-header").is_some());
}
