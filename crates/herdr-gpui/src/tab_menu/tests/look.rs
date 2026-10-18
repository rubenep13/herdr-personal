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
