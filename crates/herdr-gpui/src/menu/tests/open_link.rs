use gpui::{TestAppContext, px, size};

#[gpui::test]
fn open_link_puts_no_before_yes_on_the_right(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            let url =
                crate::terminal::ExternalUrl::try_from("vscode://file/tmp/readme.md:12").unwrap();
            view.ask_to_open_link(url, window, cx);
        });
        window.draw(cx).clear(cx);
    });
    let panel = cx.debug_bounds("menu-panel").unwrap();
    let no = cx.debug_bounds("open-link-no").unwrap();
    let yes = cx.debug_bounds("open-link-yes").unwrap();
    assert!(yes.left() > no.right(), "{no:?} {yes:?}");
    assert!(no.left() > panel.center().x, "{no:?} in {panel:?}");
    let cross = cx.debug_bounds("open-link-dismiss").unwrap();
    assert!(
        cross.right() >= panel.right() - px(12.),
        "{cross:?} in {panel:?}"
    );
    let url = cx.debug_bounds("open-link-url").unwrap();
    assert!(panel.contains(&url.origin) && url.right() <= panel.right());
}
