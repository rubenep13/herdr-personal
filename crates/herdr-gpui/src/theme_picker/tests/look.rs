//! The Color Scheme picker in the redesigned menus' look.
use super::*;

fn open(cx: &mut TestAppContext) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(1000.), px(800.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.config.theme = "Nord".into();
            view.open_theme_picker(window, cx);
            let picker = view.menu.themes.as_mut().unwrap();
            picker.names = vec!["Default".into(), "Nord".into(), "Dracula".into()];
            picker.filter("");
            cx.notify();
        })
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (view, cx)
}

#[gpui::test]
fn the_theme_picker_wears_the_menu_panel(cx: &mut TestAppContext) {
    let (_, cx) = open(cx);
    assert_eq!(cx.debug_bounds("theme-row-0").unwrap().size.height, px(32.));
    let close = cx.debug_bounds("theme-close").unwrap();
    assert_eq!((close.size.width, close.size.height), (px(28.), px(28.)));
    assert_eq!(
        cx.debug_bounds("theme-status").unwrap().size.height,
        px(36.)
    );
    assert!(cx.debug_bounds("theme-count").is_some());
    assert!(
        cx.debug_bounds("theme-row-1-check").is_some(),
        "Nord is current"
    );
    assert!(cx.debug_bounds("theme-row-0-check").is_none());
}

#[gpui::test]
fn theme_rows_keep_their_width_before_the_palette_loads(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let before = cx.debug_bounds("theme-name-Default").unwrap();
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.theme_swatches.get("Default"), None, "nothing loaded yet");
    });
    // The rows on screen asked for their strips; let the load finish.
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.menu.theme_swatches.get("Default").is_some());
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(cx.debug_bounds("theme-row-0-swatches").is_some());
    let after = cx.debug_bounds("theme-name-Default").unwrap();
    assert_eq!(before.left(), after.left());
}
