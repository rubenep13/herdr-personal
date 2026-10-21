//! The pull request review in the redesigned look: its checks are menu rows.

use super::*;

#[gpui::test]
fn review_checks_are_menu_rows(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, crate::pull_request::fixture().unwrap());
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.activate_git_row(Row::Review, window, cx)
        })
    });
    cx.simulate_resize(size(px(900.), px(700.)));
    draw(cx);
    let check = cx.debug_bounds("pr-review-check").unwrap();
    assert_eq!(check.size.height, px(32.));
}
