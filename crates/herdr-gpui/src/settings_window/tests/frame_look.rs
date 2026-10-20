//! The section list, the page's frame, the footer and search results in the
//! redesigned look, and how they grow with a large interface face.
use super::*;
use gpui::{px, size};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

fn open(
    cx: &mut TestAppContext,
    width: f32,
    height: f32,
) -> (Entity<SettingsWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(width), px(height)));
    draw(cx);
    (view, cx)
}

#[gpui::test]
fn navigation_rows_are_thirty_two_tall_with_sixteen_px_icons(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, 1200., 900.);
    let row = cx.debug_bounds("settings-section-theme").unwrap();
    assert_eq!(row.size.height, px(32.));
    let icon = cx.debug_bounds("settings-section-theme-icon").unwrap();
    assert_eq!((icon.size.width, icon.size.height), (px(16.), px(16.)));
}

#[gpui::test]
fn navigation_starts_224_wide(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, 1200., 900.);
    assert_eq!(
        cx.debug_bounds("settings-navigation").unwrap().size.width,
        px(224.)
    );
}

#[gpui::test]
fn the_footer_is_thirty_six_tall(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, 1200., 900.);
    assert_eq!(
        cx.debug_bounds("settings-footer").unwrap().size.height,
        px(36.)
    );
}

#[gpui::test]
fn search_results_share_one_card(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, 1200., 900.);
    view.update(cx, |view, cx| view.set_search_query("indent", cx));
    draw(cx);
    let card = cx.debug_bounds("settings-search-results-card").unwrap();
    let result = cx.debug_bounds("settings-search-result-0").unwrap();
    assert!(card.contains(&result.origin) && card.contains(&result.center()));
    assert!(cx.debug_bounds("settings-search-result-0-page").is_some());
}

#[gpui::test]
fn settings_grow_with_a_large_ui_face(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    view.update(cx, |view, cx| {
        view.config.ui.size = 18.;
        view.section = Section::Sidebar;
        cx.notify();
    });
    cx.simulate_resize(size(px(960.), px(780.)));
    draw(cx);
    assert_eq!(
        cx.debug_bounds("settings-section-theme")
            .unwrap()
            .size
            .height,
        px(48.)
    );
    assert!(cx.debug_bounds("sidebar-layout").unwrap().size.height >= px(72.));
    let body = cx.debug_bounds("settings-body").unwrap();
    let control = cx.debug_bounds("sidebar-select-control").unwrap();
    assert!(control.right() <= body.right(), "{control:?} in {body:?}");
}
