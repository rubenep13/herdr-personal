use super::*;

#[gpui::test]
fn tab_rows_are_menu_rows_with_an_icon_each(cx: &mut TestAppContext) {
    let (view, cx) = group_window(cx);
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_tab_menu("t0", None, point(px(400.), px(300.)), window, cx)
        });
        window.draw(cx).clear(cx);
    });
    for index in 0..ACTIONS.len() {
        let row = cx.debug_bounds(format!("tab-menu-{index}").leak()).unwrap();
        assert_eq!(row.size.height, px(32.), "row {index}");
        let icon = cx
            .debug_bounds(format!("tab-menu-{index}-icon").leak())
            .unwrap();
        assert_eq!(icon.size, size(px(16.), px(16.)), "row {index}");
    }
}

#[gpui::test]
fn leaving_a_row_restores_it(cx: &mut TestAppContext) {
    let (view, cx) = group_window(cx);
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_tab_menu("t0", None, point(px(200.), px(150.)), window, cx)
        });
        window.draw(cx).clear(cx);
    });
    let selected = |cx: &mut VisualTestContext| {
        view.read_with(cx, |v, _| v.menu.tab.as_ref().unwrap().selected)
    };
    let row = cx.debug_bounds("tab-menu-1").unwrap();
    cx.simulate_mouse_move(row.center(), None, Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(selected(cx), Some(1));
    let panel = cx.debug_bounds("menu-panel").unwrap();
    cx.simulate_mouse_move(
        point(panel.right() + px(20.), panel.bottom() + px(20.)),
        None,
        Modifiers::default(),
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(selected(cx), None);
}
