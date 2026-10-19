#![allow(clippy::unwrap_used)]
use super::*;
use crate::config::{Config, Theme, mix};
use gpui::rgb;

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

#[test]
fn settings_look_takes_every_colour_from_the_theme() {
    for theme in themes() {
        let look = SettingsLook::of(&theme, &font(12.));
        assert_eq!(
            look.card_fill(),
            rgb(mix(theme.background, theme.active, 30))
        );
        assert_eq!(look.nav(), rgb(theme.sidebar_background()));
        assert_eq!(look.line(), rgb(theme.active));
        assert_eq!(look.accent(), rgb(theme.primary()));
        assert_eq!(look.well(), rgb(theme.surface));
        assert_eq!(look.muted(), rgb(theme.muted));
    }
}

#[test]
fn tags_and_chips_follow_the_theme() {
    for theme in themes() {
        let look = SettingsLook::of(&theme, &font(12.));
        assert_eq!(
            look.shared_fill(),
            rgb(mix(theme.background, theme.primary(), 14))
        );
        assert_eq!(
            look.local_fill(),
            rgb(mix(theme.background, theme.active, 70))
        );
        assert_eq!(look.good(), rgb(theme.ink(theme.palette[2])));
        assert_eq!(look.warn(), rgb(theme.ink(theme.palette[3])));
    }
}

#[test]
fn settings_measures_grow_with_the_ui_size() {
    let measures = |size| {
        let look = SettingsLook::of(&Theme::default(), &font(size));
        [
            look.nav_row_height(),
            look.control_height(),
            look.segment_height(),
            look.row_min_height(),
            look.small_button_height(),
        ]
    };
    assert_eq!(measures(12.), [32., 30., 26., 48., 26.]);
    assert_eq!(measures(18.), [48., 45., 39., 72., 39.]);
}
