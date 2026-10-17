use super::*;
use crate::config::{TabContent, TabPlacement, TabsStyle};

#[test]
fn tabs_style_defaults_to_the_spec() -> anyhow::Result<()> {
    for config in [
        Config::default(),
        Config::parse("")?,
        Config::parse(DEFAULT_CONFIG)?,
    ] {
        assert_eq!(config.tabs_style, TabsStyle::default());
        assert_eq!(config.tabs_style.content, TabContent::Name);
        assert_eq!(config.tabs_style.placement, TabPlacement::Subheader);
    }
    Ok(())
}

#[test]
fn tabs_style_parses_every_value() -> anyhow::Result<()> {
    for (name, content) in [
        ("name", TabContent::Name),
        ("agent", TabContent::Agent),
        ("status", TabContent::Status),
    ] {
        let config = Config::parse(&format!("[tabs]\ncontent = \"{name}\""))?;
        assert_eq!(config.tabs_style.content, content);
    }
    for (name, placement) in [
        ("subheader", TabPlacement::Subheader),
        ("bottom", TabPlacement::Bottom),
        ("herdr", TabPlacement::Herdr),
    ] {
        let config = Config::parse(&format!("[tabs]\nposition = \"{name}\""))?;
        assert_eq!(config.tabs_style.placement, placement);
    }
    for text in ["content = \"icon\"", "position = \"top\"", "content = 1"] {
        assert!(Config::parse(&format!("[tabs]\n{text}")).is_err(), "{text}");
    }
    Ok(())
}

#[test]
fn tabs_font_keys_still_load() -> anyhow::Result<()> {
    let config = Config::parse("[tabs]\nsize = 14\ncontent = \"agent\"")?;
    assert_eq!(config.tabs.size, 14.);
    assert_eq!(config.tabs_style.content, TabContent::Agent);
    let config = Config::parse("[tabs]\nfamily = \"Custom Sans\"\nfallback = []")?;
    assert_eq!(config.tabs.family, "Custom Sans");
    assert!(config.unknown_keys.is_empty(), "{:?}", config.unknown_keys);
    // Invalid font values are still refused, not ignored.
    assert!(Config::parse("[tabs]\nsize = 7.9").is_err());
    assert!(Config::parse("[tabs]\nfamily = 14").is_err());
    Ok(())
}

#[test]
fn unknown_tabs_keys_are_reported() -> anyhow::Result<()> {
    let config = Config::parse("[tabs]\nfoo = 1\nline_height = 1.2\nposition = \"bottom\"")?;
    assert!(config.unknown_keys.contains(&"tabs.foo".to_owned()));
    // Terminal-only font keys stay reported, as they were before `[tabs]`
    // gained style keys.
    assert!(config.unknown_keys.contains(&"tabs.line_height".to_owned()));
    assert_eq!(config.tabs_style.placement, TabPlacement::Bottom);
    Ok(())
}
