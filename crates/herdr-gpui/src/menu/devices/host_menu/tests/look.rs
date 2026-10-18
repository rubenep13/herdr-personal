use super::*;

#[test]
fn removing_the_device_is_the_only_danger() {
    for &(action, label) in SSH_ACTIONS {
        assert_eq!(action.danger(), action == Action::Remove, "{label}");
    }
}

/// The menu names the device under its icon, and every row is a menu row
/// led by an icon.
#[gpui::test]
fn host_menu_heads_with_the_device_and_icons_each_row(cx: &mut TestAppContext) {
    if cfg!(windows) {
        return;
    }
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        add_host(&mut view);
        view
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    view.update_in(cx, |view, window, cx| {
        view.open_host_menu(HOST, point(px(20.), px(20.)), window, cx)
    });
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("host-menu-header").is_some());
    assert!(cx.debug_bounds("host-menu-name").is_some());
    let icon = cx.debug_bounds("host-menu-header-icon").unwrap();
    assert_eq!(icon.size, size(px(16.), px(16.)));
    for index in 0..SSH_ACTIONS.len() {
        let row = cx
            .debug_bounds(format!("host-menu-{index}").leak())
            .unwrap();
        assert_eq!(row.size.height, px(32.), "row {index}");
        assert!(
            cx.debug_bounds(format!("host-menu-{index}-icon").leak())
                .is_some(),
            "row {index}"
        );
    }
}

#[gpui::test]
fn leaving_a_row_restores_it(cx: &mut TestAppContext) {
    if cfg!(windows) {
        return;
    }
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        add_host(&mut view);
        view
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    view.update_in(cx, |view, window, cx| {
        view.open_host_menu(HOST, point(px(20.), px(20.)), window, cx)
    });
    let draw = |cx: &mut VisualTestContext| {
        cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx))
    };
    draw(cx);
    let selected = |cx: &mut VisualTestContext| {
        view.read_with(cx, |view, _| view.menu.host.as_ref().unwrap().selected)
    };
    let row = cx.debug_bounds("host-menu-0").unwrap();
    cx.simulate_mouse_move(row.center(), None, Modifiers::default());
    draw(cx);
    assert_eq!(selected(cx), Some(0));
    let panel = cx.debug_bounds("menu-panel").unwrap();
    cx.simulate_mouse_move(
        point(panel.right() + px(20.), panel.bottom() + px(20.)),
        None,
        Modifiers::default(),
    );
    draw(cx);
    assert_eq!(selected(cx), None);
}
