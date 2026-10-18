use super::*;

#[gpui::test]
fn git_rows_are_menu_rows(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    let input = crate::pull_request::Input {
        checkout: None,
        repo_key: Some(REPO_KEY.into()),
        branch: "develop".into(),
    };
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.git = crate::git::Git::fixture(input, status(146, 42, 0));
            view.open_git_menu(point(px(900.), px(20.)), window, cx);
        })
    });
    cx.update(|window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    });
    let label = view.read_with(cx, |view, _| view.git_rows()[0].1.clone());
    let row = cx.debug_bounds(format!("git-menu-{label}").leak()).unwrap();
    assert_eq!(row.size.height, px(32.));
    let icon = cx
        .debug_bounds(format!("git-menu-{label}-icon").leak())
        .unwrap();
    assert_eq!(icon.size, size(px(16.), px(16.)));
}
