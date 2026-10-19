//! An open dropdown lists its options as the redesigned menus do.
use crate::settings_window::{SettingsWindow, section::Section, tests::skill_fixture};
use gpui::{Entity, TestAppContext, VisualTestContext, px, size};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

fn open_layout(cx: &mut TestAppContext) -> (Entity<SettingsWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    view.update(cx, |view, cx| {
        view.section = Section::Sidebar;
        cx.notify();
    });
    cx.simulate_resize(size(px(1200.), px(900.)));
    draw(cx);
    let opener = cx.debug_bounds("sidebar-layout-control").unwrap();
    cx.simulate_click(opener.center(), Default::default());
    draw(cx);
    (view, cx)
}

fn current(view: &Entity<SettingsWindow>, cx: &mut VisualTestContext) -> usize {
    view.read_with(cx, |view, _| view.dropdown.as_ref().unwrap().current)
}

#[gpui::test]
fn an_open_dropdown_draws_menu_rows(cx: &mut TestAppContext) {
    let (_, cx) = open_layout(cx);
    let row = cx.debug_bounds("sidebar-layout-control-option-0").unwrap();
    assert_eq!(row.size.height, px(32.));
    assert!(row.size.width >= px(170.), "{row:?}");
}

#[gpui::test]
fn the_saved_option_keeps_the_check(cx: &mut TestAppContext) {
    let (view, cx) = open_layout(cx);
    let saved = current(&view, cx);
    let check = |cx: &mut VisualTestContext, index: usize| {
        let selector: &'static str =
            Box::leak(format!("sidebar-layout-control-option-{index}-check").into_boxed_str());
        cx.debug_bounds(selector).is_some()
    };
    assert!(check(cx, saved));
    cx.simulate_keystrokes("down");
    draw(cx);
    let selected = view.read_with(cx, |view, _| view.dropdown.as_ref().unwrap().selected);
    assert_ne!(selected, saved);
    assert!(check(cx, saved), "the check stays on the saved option");
    assert!(!check(cx, selected));
}

#[gpui::test]
fn hover_moves_the_highlight_and_leaving_restores_it(cx: &mut TestAppContext) {
    let (view, cx) = open_layout(cx);
    let saved = current(&view, cx);
    view.update(cx, |view, cx| view.hover_dropdown(2, true, cx));
    view.read_with(cx, |view, _| {
        assert_eq!(view.dropdown.as_ref().unwrap().selected, 2)
    });
    view.update(cx, |view, cx| view.hover_dropdown(2, false, cx));
    view.read_with(cx, |view, _| {
        assert_eq!(view.dropdown.as_ref().unwrap().selected, saved)
    });
}
