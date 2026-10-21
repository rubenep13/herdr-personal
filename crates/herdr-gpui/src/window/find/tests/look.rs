use super::*;
use gpui::px;

/// The find bar is the menus' panel in small: its field holds the count,
/// and its buttons are the header's icon buttons.
#[gpui::test]
fn the_find_bar_is_a_small_panel_with_its_buttons(cx: &mut TestAppContext) {
    let (view, _peer, cx) = open(cx, &["pane.copy_search"]);
    find(&view, cx);
    let bar = cx.debug_bounds("find-bar").unwrap();
    for selector in ["find-older", "find-newer", "find-close"] {
        let button = cx.debug_bounds(selector).unwrap();
        assert_eq!(button.size.height, px(28.), "{selector}");
        assert!(
            bar.contains(&button.origin) && bar.contains(&button.bottom_right()),
            "{selector}: {button:?} in {bar:?}"
        );
    }
    let field = cx.debug_bounds("find-field").unwrap();
    let count = cx.debug_bounds("find-count").unwrap();
    assert!(field.contains(&count.center()));
    assert_eq!(field.size.height, px(28.));
}
