#![allow(clippy::unwrap_used)]
use super::*;
use crate::config::{Config, Theme, mix};
use gpui::rgb;
use herdr_client::protocol::AgentStatus;

fn font(size: f32) -> FontConfig {
    FontConfig {
        size,
        ..Config::default().ui
    }
}

/// Each theme with whether it is light.
fn themes() -> [(Theme, bool); 2] {
    [
        (Theme::default(), false),
        (Theme::builtin("Catppuccin Latte").unwrap(), true),
    ]
}

/// The colour as the `0xRRGGBB` the contrast helpers read.
fn hex(colour: Rgba) -> u32 {
    let channel = |value: f32| (value * 255.).round() as u32;
    channel(colour.r) << 16 | channel(colour.g) << 8 | channel(colour.b)
}

#[test]
fn view_measures_grow_with_the_ui_size() {
    let theme = Theme::default();
    let small = ViewLook::of(&theme, &font(12.), false);
    assert_eq!(
        [small.at(40.), small.at(28.), small.at(46.)],
        [px(40.), px(28.), px(46.)]
    );
    let large = ViewLook::of(&theme, &font(18.), false);
    assert_eq!(
        [large.at(40.), large.at(28.), large.at(46.)],
        [px(60.), px(42.), px(69.)]
    );
}

#[test]
fn tones_read_on_their_wash() {
    for (theme, light) in themes() {
        let look = ViewLook::of(&theme, &font(12.), light);
        let working =
            crate::sidebar::Indicators::new(None, None, light, &theme).color(AgentStatus::Working);
        for tone in [
            Tone::Good,
            Tone::Warn,
            Tone::Danger,
            Tone::Accent,
            Tone::Merged,
            Tone::Status(working),
        ] {
            let (ink, wash) = look.settings().tone(tone);
            let ratio = crate::contrast::ratio(hex(ink), hex(wash));
            assert!(ratio >= 3., "{tone:?} on {:06x}: {ratio}", hex(wash));
        }
        let (ink, wash) = look.pill_on();
        let ratio = crate::contrast::ratio(hex(ink), hex(wash));
        assert!(ratio >= 3., "lit pill on {:06x}: {ratio}", hex(wash));
    }
}

#[test]
fn view_colours_come_from_the_theme() {
    for (theme, light) in themes() {
        let look = ViewLook::of(&theme, &font(12.), light);
        assert_eq!(look.background(), rgb(theme.background));
        assert_eq!(look.side(), rgb(theme.surface));
        assert_eq!(look.selection(), rgb(theme.active));
        assert_eq!(look.hover(), rgb(mix(theme.background, theme.active, 60)));
        assert_eq!(look.line(), rgb(theme.active));
    }
}

#[test]
fn a_status_chip_takes_the_colour_it_is_given() {
    let theme = Theme::default();
    let look = SettingsLook::of(&theme, &font(12.));
    let colour = 0x12_34_56;
    assert_eq!(
        look.tone(Tone::Status(colour)),
        (rgb(colour), rgb(mix(theme.background, colour, 13)))
    );
}
