#![allow(clippy::unwrap_used)]
use super::*;
use crate::config::{Config, Theme};
use gpui::{Rgba, rgb};

fn font(size: f32) -> FontConfig {
    FontConfig {
        size,
        ..Config::default().ui
    }
}

fn themes() -> [Theme; 2] {
    [
        Theme::default(),
        Theme::builtin("Catppuccin Latte").unwrap(),
    ]
}

const SELECTED: ItemState = ItemState {
    selected: true,
    disabled: false,
    danger: false,
};

#[test]
fn menu_look_takes_every_colour_from_the_theme() {
    for theme in themes() {
        let look = MenuLook::of(&theme, &font(12.), false);
        assert_eq!(look.fill(SELECTED), Some(rgb(theme.active)));
        assert_eq!(look.ink(ItemState::default()), rgb(theme.subtext()));
        assert_eq!(look.ink(SELECTED), rgb(theme.foreground));
        let disabled = ItemState {
            disabled: true,
            ..ItemState::default()
        };
        assert_eq!(look.ink(disabled), rgb(theme.muted));
        let danger = ItemState {
            danger: true,
            ..ItemState::default()
        };
        assert_eq!(look.ink(danger), crate::menu::danger(&theme));
    }
}

#[test]
fn a_resting_row_has_no_fill() {
    let look = MenuLook::of(&Theme::default(), &font(12.), false);
    assert_eq!(look.fill(ItemState::default()), None);
    let danger = ItemState {
        danger: true,
        ..ItemState::default()
    };
    assert_eq!(look.fill(danger), None);
}

#[test]
fn a_selected_danger_row_is_a_fourteen_percent_wash() {
    for theme in themes() {
        let look = MenuLook::of(&theme, &font(12.), false);
        let red = crate::menu::danger(&theme);
        let state = ItemState {
            danger: true,
            ..SELECTED
        };
        assert_eq!(look.fill(state), Some(Rgba { a: 0.14, ..red }));
    }
}

#[test]
fn rows_grow_with_the_ui_size() {
    let theme = Theme::default();
    let look = MenuLook::of(&theme, &font(12.), false);
    assert_eq!(look.row_height(), 32.);
    assert_eq!(look.list_row_height(), 44.);
    let large = MenuLook::of(&theme, &font(18.), false);
    assert_eq!(large.row_height(), 48.);
    assert_eq!(large.list_row_height(), 66.);
}

#[test]
fn light_themes_get_a_fainter_shadow() {
    let theme = Theme::builtin("Catppuccin Latte").unwrap();
    let light = MenuLook::of(&theme, &font(12.), true);
    let dark = MenuLook::of(&Theme::default(), &font(12.), false);
    assert!(light.shadow_opacity() < dark.shadow_opacity());
}

/// Separators part groups of rows, so they must show on dark themes too:
/// they stand further from the panel than its hairline border does.
#[test]
fn separators_stand_out_from_the_panel() {
    let distance = |a: Rgba, b: Rgba| (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs();
    for theme in themes() {
        let look = MenuLook::of(&theme, &font(12.), false);
        let panel = rgb(theme.background);
        assert!(distance(look.rule(), panel) > distance(look.line(), panel));
    }
}

/// A shortcut splits into one chip per key, chords included.
#[test]
fn shortcuts_split_into_keys() {
    assert_eq!(
        keycaps("ctrl-shift-t").collect::<Vec<_>>(),
        ["Ctrl", "Shift", "T"]
    );
    assert_eq!(keycaps("ctrl-b n").collect::<Vec<_>>(), ["Ctrl", "B", "N"]);
}

/// No shortcut, no keys: an unbound entry draws no empty chip.
#[test]
fn an_empty_shortcut_has_no_keys() {
    assert_eq!(keycaps("").count(), 0);
}

struct CloseHarness;

impl Render for CloseHarness {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        let look = MenuLook::of(&Theme::default(), &font(12.), false);
        div().child(look.close_button("close-demo"))
    }
}

#[gpui::test]
fn close_button_is_a_28px_square(cx: &mut gpui::TestAppContext) {
    let (_, cx) = cx.add_window_view(|_, _| CloseHarness);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let close = cx.debug_bounds("close-demo").unwrap();
    assert_eq!((close.size.width, close.size.height), (px(28.), px(28.)));
}
