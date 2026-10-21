//! The Git menu's forms keep its panel: Commit under the menu's heading,
//! with small buttons, disabled until there is a message.

use super::*;

#[gpui::test]
fn commit_keeps_the_git_menus_panel(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(900.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.git = crate::git::Git::fixture(
                crate::pull_request::Input {
                    checkout: None,
                    repo_key: Some(REPO_KEY.into()),
                    branch: "develop".into(),
                },
                status(12, 3, 1),
            );
            view.open_git_menu(point(px(860.), px(20.)), window, cx);
        })
    });
    cx.update(|window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    });
    let menu = cx.debug_bounds("menu-panel").unwrap();
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.activate_git_row(Row::Commit, window, cx)
        });
        window.refresh();
        let _ = window.draw(cx);
    });
    let form = cx.debug_bounds("menu-panel").unwrap();
    assert!(
        (form.top() - menu.top()).abs() <= px(1.),
        "{menu:?} {form:?}"
    );
    assert!(
        (form.right() - menu.right()).abs() <= px(1.),
        "{menu:?} {form:?}"
    );
    assert!(cx.debug_bounds("git-commit-header").is_some());
    let submit = cx.debug_bounds("git-commit-submit").unwrap();
    assert_eq!(submit.size.height, px(26.));
    // Disabled with an empty message: a click does nothing.
    view.update(cx, |view, _| {
        if let Some(input) = view.menu.input.as_mut() {
            input.text.clear();
        }
    });
    cx.update(|window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    });
    let submit = cx.debug_bounds("git-commit-submit").unwrap();
    cx.simulate_click(submit.center(), gpui::Modifiers::default());
    assert!(view.read_with(cx, |view, _| view.menu.error.is_none()));
}
