//! Renaming a tab keeps the tab menu's panel: the menu's heading, the field
//! in Settings' frame, and small buttons under a hairline.

use super::*;

fn rename(cx: &mut TestAppContext) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = group_window(cx);
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_tab_menu("t0", None, point(px(200.), px(150.)), window, cx)
        });
        window.draw(cx).clear(cx);
    });
    (view, cx)
}

#[gpui::test]
fn rename_tab_keeps_the_menus_panel_and_heading(cx: &mut TestAppContext) {
    let (view, cx) = rename(cx);
    let menu = cx.debug_bounds("menu-panel").unwrap();
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.activate_tab_menu(Action::Rename, window, cx)
        });
        window.draw(cx).clear(cx);
    });
    let form = cx.debug_bounds("menu-panel").unwrap();
    assert!(
        (form.origin.x - menu.origin.x).abs() <= px(1.),
        "{menu:?} {form:?}"
    );
    assert!(
        (form.origin.y - menu.origin.y).abs() <= px(1.),
        "{menu:?} {form:?}"
    );
    let header = cx.debug_bounds("tab-rename-header").unwrap();
    let field = cx.debug_bounds("tab-rename-field").unwrap();
    assert!(header.bottom() <= field.top(), "{header:?} {field:?}");
    assert_eq!(field.size.height, px(30.));
    let cancel = cx.debug_bounds("tab-rename-cancel").unwrap();
    let submit = cx.debug_bounds("tab-rename-submit").unwrap();
    assert!(submit.left() > cancel.right(), "{cancel:?} {submit:?}");
    assert_eq!(submit.size.height, px(26.));
}

#[gpui::test]
fn enter_and_escape_still_work_in_rename_tab(cx: &mut TestAppContext) {
    let (view, cx) = rename(cx);
    cx.simulate_keystrokes("down down enter");
    assert!(view.read_with(cx, |v, _| v.menu.page == Some(Page::RenameTab)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_keystrokes("escape");
    assert!(view.read_with(cx, |v, _| v.menu.page.is_none()));
}
