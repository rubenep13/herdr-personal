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

/// A row's shortcut is drawn as keys.
#[gpui::test]
fn the_new_browser_tab_shortcut_is_chips(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    tunnels::remote_menu(&view, cx);
    let shortcut = view.read_with(cx, |view, _| {
        view.keymap().primary(Command::NewBrowserTab).to_owned()
    });
    let keys = crate::design::keycaps(&shortcut).count();
    assert!(keys > 0, "New Browser Tab has a default shortcut");
    for index in 0..=keys {
        let selector = format!("group-menu-NewBrowserTab-detail-key-{index}");
        assert_eq!(cx.debug_bounds(selector.leak()).is_some(), index < keys);
    }
}

#[gpui::test]
fn leaving_a_row_restores_it(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    tunnels::remote_menu(&view, cx);
    let selected = |cx: &mut VisualTestContext| {
        view.read_with(cx, |view, _| view.menu.group.as_ref().unwrap().selected)
    };
    let row = cx.debug_bounds("group-menu-NewBrowserTab").unwrap();
    cx.simulate_mouse_move(row.center(), None, Modifiers::default());
    draw(cx);
    assert!(selected(cx).is_some());
    let panel = cx.debug_bounds("menu-panel").unwrap();
    cx.simulate_mouse_move(
        point(panel.right() + px(20.), panel.bottom() + px(20.)),
        None,
        Modifiers::default(),
    );
    draw(cx);
    assert_eq!(selected(cx), None);
}
