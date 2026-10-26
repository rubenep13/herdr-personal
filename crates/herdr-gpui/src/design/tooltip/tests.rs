#![allow(clippy::unwrap_used)]
use super::*;
use crate::config::{Config, FontConfig, Theme};
use gpui::rgb;

fn font() -> FontConfig {
    Config::default().ui
}

#[test]
fn a_tooltip_wears_the_menus_panel() {
    let mocha = Theme::builtin("Catppuccin Mocha").unwrap();
    let latte = Theme::builtin("Catppuccin Latte").unwrap();
    for (theme, light) in [(Theme::default(), false), (latte.clone(), true)] {
        let tooltip = MenuLook::of(&theme, &font(), light).tooltip_card("Sessions");
        assert_eq!(tooltip.fill, rgb(theme.background));
        assert_eq!(tooltip.line, rgb(theme.active));
        assert_eq!(tooltip.ink, rgb(theme.subtext()));
        assert_eq!(tooltip.text, SharedString::from("Sessions"));
    }
    let dark = MenuLook::of(&mocha, &font(), false).tooltip_card("x");
    let light = MenuLook::of(&latte, &font(), true).tooltip_card("x");
    assert!(light.shadow < dark.shadow);
}
