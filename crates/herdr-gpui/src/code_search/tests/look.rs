// Not a glob, for the parent's reason: `gpui::*` would shadow `#[test]`.
use super::{Mode, checkout, window};
use gpui::{TestAppContext, px, size};

#[gpui::test]
fn code_search_rows_are_list_rows_with_a_tile(cx: &mut TestAppContext) {
    let dir = checkout();
    let (view, cx) = window(cx, Some(&dir.path().join("src")));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_code_search(Mode::Symbols, window, cx)
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    let row = cx.debug_bounds("code-search-row-0").unwrap();
    assert_eq!(row.size.height, px(44.));
    let tile = cx.debug_bounds("code-search-tile-0").unwrap();
    assert_eq!(tile.size, size(px(24.), px(24.)));
    let icon = cx.debug_bounds("code-search-search-icon").unwrap();
    assert_eq!(icon.size, size(px(18.), px(18.)));
}

#[gpui::test]
fn modes_are_underlined_tabs(cx: &mut TestAppContext) {
    let dir = checkout();
    let (view, cx) = window(cx, Some(dir.path()));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_code_search(Mode::Symbols, window, cx)
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    let underline = cx.debug_bounds("code-search-mode-underline").unwrap();
    let symbols = cx.debug_bounds("code-search-mode-symbols").unwrap();
    assert_eq!(underline.size.height, px(2.));
    assert!(underline.left() > symbols.left() && underline.right() < symbols.right());
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    let moved = cx.debug_bounds("code-search-mode-underline").unwrap();
    let files = cx.debug_bounds("code-search-mode-files").unwrap();
    assert!(moved.left() > files.left() && moved.right() < files.right());
}
