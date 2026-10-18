use super::*;
use crate::HerdrWindow;
use gpui::{Entity, TestAppContext, VisualTestContext, point, px, size};

/// Opens the footer menu over the status bar, as its hint does, with the
/// interface face at `ui_size`.
fn open_footer_menu(
    ui_size: f32,
    cx: &mut TestAppContext,
) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        view.config.ui.size = ui_size;
        view
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.menu.anchor = point(px(60.), px(560.));
            view.open_menu(window, cx);
        });
        window.draw(cx).clear(cx);
    });
    (view, cx)
}

#[gpui::test]
fn every_footer_item_has_a_title_case_label_and_an_icon(cx: &mut TestAppContext) {
    let (view, cx) = open_footer_menu(12., cx);
    let items = view.read_with(cx, |view, _| view.menu_items());
    assert!(items.contains(&"settings"));
    for item in items {
        let label = chrome::items::label(item);
        assert!(
            label.starts_with(|c: char| c.is_ascii_uppercase()),
            "{item} reads {label}"
        );
        assert!(chrome::items::icon(item).is_some(), "{item} has no icon");
    }
    assert_eq!(
        chrome::items::label("increase font size"),
        "Increase Font Size"
    );
    assert_eq!(chrome::items::label("GitHub sign-in"), "GitHub Sign-in");
}

#[gpui::test]
fn footer_menu_rows_are_menu_rows(cx: &mut TestAppContext) {
    let (_view, cx) = open_footer_menu(12., cx);
    let panel = cx.debug_bounds("menu-panel").unwrap();
    let row = cx.debug_bounds("menu-settings").unwrap();
    assert_eq!(row.size.height, px(32.));
    // The panel's 1 px border and 6 px inset.
    assert_eq!(row.left(), panel.left() + px(7.));
    assert_eq!(row.right(), panel.right() - px(7.));
    let icon = cx.debug_bounds("menu-settings-icon").unwrap();
    assert_eq!(icon.size, size(px(16.), px(16.)));
}

#[gpui::test]
fn footer_menu_rows_grow_with_the_ui_size(cx: &mut TestAppContext) {
    let (_view, cx) = open_footer_menu(18., cx);
    assert_eq!(
        cx.debug_bounds("menu-settings").unwrap().size.height,
        px(48.)
    );
    let icon = cx.debug_bounds("menu-settings-icon").unwrap();
    assert_eq!(icon.size, size(px(24.), px(24.)));
}
