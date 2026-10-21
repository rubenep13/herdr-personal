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

/// The colour as the `0xRRGGBB` the contrast helpers read.
fn hex(colour: Rgba) -> u32 {
    let channel = |value: f32| (value * 255.).round() as u32;
    channel(colour.r) << 16 | channel(colour.g) << 8 | channel(colour.b)
}

#[test]
fn dialog_colours_come_from_the_theme() {
    for theme in themes() {
        let look = DialogLook::of(&theme, &font(12.), false);
        assert_eq!(look.well_fill(), rgb(theme.surface));
        assert_eq!(look.line(), rgb(theme.active));
        let danger = crate::menu::danger(&theme);
        assert_eq!(look.error_fill(), Rgba { a: 0.13, ..danger });
        assert_eq!(look.error_ink(), danger);
        assert_eq!(
            look.note_ink(NoteTone::Done),
            rgb(theme.ink(theme.palette[2]))
        );
        assert_eq!(look.note_ink(NoteTone::Warning), danger);
        assert_eq!(look.note_ink(NoteTone::Info), rgb(theme.muted));
    }
}

#[test]
fn primary_and_danger_buttons_read_on_their_fill() {
    let mut all = themes().to_vec();
    all.push(Theme::builtin("Catppuccin Mocha").unwrap());
    for theme in all {
        let look = SettingsLook::of(&theme, &font(12.));
        for kind in [ButtonKind::Primary, ButtonKind::Danger] {
            let (fill, _, ink) = look.button_colours(kind);
            let ratio = crate::contrast::ratio(hex(ink), hex(fill));
            assert!(ratio >= 3., "{kind:?}: {ratio} on {:06x}", hex(fill));
        }
        let (fill, _, _) = look.button_colours(ButtonKind::Danger);
        assert_eq!(fill, crate::menu::danger(&theme));
    }
    // The design's colour where the theme allows it: the window's background
    // on Mocha's accent and danger red. Latte's pink accent takes its dark
    // text, which reads where the light background does not.
    let theme = Theme::builtin("Catppuccin Mocha").unwrap();
    let look = SettingsLook::of(&theme, &font(12.));
    for kind in [ButtonKind::Primary, ButtonKind::Danger] {
        assert_eq!(
            look.button_colours(kind).2,
            rgb(theme.background),
            "{kind:?}"
        );
    }
}

#[test]
fn dialog_measures_grow_with_the_ui_size() {
    let theme = Theme::default();
    let small = DialogLook::of(&theme, &font(12.), false);
    assert_eq!(
        [small.at(52.), small.at(62.), small.at(30.)],
        [px(52.), px(62.), px(30.)]
    );
    let large = DialogLook::of(&theme, &font(18.), false);
    assert_eq!(
        [large.at(52.), large.at(62.), large.at(30.)],
        [px(78.), px(93.), px(45.)]
    );
}

#[test]
fn a_danger_header_paints_its_icon_red() {
    for theme in themes() {
        let look = MenuLook::of(&theme, &font(12.), false);
        assert_eq!(look.header_icon(true), crate::menu::danger(&theme));
        assert_eq!(look.header_icon(false), rgb(theme.muted));
    }
}
