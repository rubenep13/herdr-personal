use super::*;

/// The strip menu's rows are menu rows; its listening ports sit under a
/// section heading, and runs are parted by an inset separator rather than a
/// rule on a row's top edge.
#[gpui::test]
fn group_rows_and_listening_follow_the_menu_look(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    let group = tunnels::remote_menu(&view, cx);
    for action in actions(&view, cx, group) {
        let selector: &'static str = action.selector().leak();
        let row = cx.debug_bounds(selector).unwrap();
        assert_eq!(row.size.height, px(32.), "{selector}");
    }
    let heading = cx.debug_bounds("group-menu-section-listening").unwrap();
    let separator = cx.debug_bounds("group-menu-separator-listening").unwrap();
    assert_eq!(separator.size.height, px(1.));
    assert!(separator.bottom() <= heading.top());
    let port = cx.debug_bounds("group-menu-Port5173").unwrap();
    assert!(heading.bottom() <= port.top());
}
