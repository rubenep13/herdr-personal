use super::*;

/// The picker's rows are menu rows, each led by its destination's icon, under
/// a section heading.
#[gpui::test]
fn move_picker_rows_are_menu_rows(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot()));
        view.endpoints[0].connection.target = herdr_client::ConnectTarget::Local;
        view
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.open_pane_menu("inactive", point(px(100.), px(100.)), window, cx);
            v.activate_pane_menu(Action::MoveTo, window, cx);
            assert_eq!(v.menu.page, Some(Page::MovePane));
        });
        window.draw(cx).clear(cx);
    });
    assert!(cx.debug_bounds("pane-move-heading").is_some());
    let count = view.read_with(cx, |v, _| {
        v.menu.pane.as_ref().unwrap().target.destinations().len()
    });
    assert!(count > 0);
    for index in 0..count {
        let row = cx
            .debug_bounds(format!("pane-move-{index}").leak())
            .unwrap();
        assert_eq!(row.size.height, px(32.), "row {index}");
        let icon = cx
            .debug_bounds(format!("pane-move-{index}-icon").leak())
            .unwrap();
        assert_eq!(icon.size, size(px(16.), px(16.)), "row {index}");
    }
}
