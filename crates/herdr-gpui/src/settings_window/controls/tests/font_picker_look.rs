//! The font family picker is the redesigned menus' panel, centred over the
//! Settings window.
use super::*;

fn open_terminal_picker(cx: &mut TestAppContext) -> &mut VisualTestContext {
    let (view, cx) = cx.add_window_view(skill_fixture);
    view.update(cx, |view, cx| {
        view.section = Section::Fonts;
        view.controls.names = vec![
            "Adwaita Sans".into(),
            "DejaVu Sans Mono".into(),
            "Noto Sans".into(),
        ];
        view.controls.filtered = filter_fonts(&view.controls.names, "");
        view.controls.discovering = false;
        view.config.terminal.family = "DejaVu Sans Mono".into();
        cx.notify();
    });
    cx.simulate_resize(size(px(1200.), px(900.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let button = cx.debug_bounds("fonts-terminal-family").unwrap();
    cx.simulate_click(button.center(), Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx
}

#[gpui::test]
fn the_font_picker_is_a_centred_menu_panel(cx: &mut TestAppContext) {
    let cx = open_terminal_picker(cx);
    let picker = cx.debug_bounds("settings-font-picker").unwrap();
    assert_eq!(picker.size.width, px(440.));
    let centre = cx.update(|window, _| window.viewport_size().width / 2.);
    assert!((picker.center().x - centre).abs() <= px(1.), "{picker:?}");
    assert_eq!(
        cx.debug_bounds("settings-font-result-0")
            .unwrap()
            .size
            .height,
        px(32.)
    );
    let close = cx.debug_bounds("settings-font-picker-close").unwrap();
    assert_eq!((close.size.width, close.size.height), (px(28.), px(28.)));
    assert!(cx.debug_bounds("settings-font-picker-title").is_some());
}

#[gpui::test]
fn the_current_family_carries_a_check(cx: &mut TestAppContext) {
    let cx = open_terminal_picker(cx);
    assert!(cx.debug_bounds("settings-font-result-1-check").is_some());
    assert!(cx.debug_bounds("settings-font-result-0-check").is_none());
}
