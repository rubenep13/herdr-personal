use super::*;
use crate::config::preferences::Preference;

/// Settings > Tabs & panes writes `[tabs] position` and `content`, and the
/// file reads back to the same choices.
#[test]
fn tab_preferences_round_trip() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("local.toml");
    Config::save_preference_path(Preference::TabsPosition(TabPlacement::Herdr), &path)?;
    Config::save_preference_path(Preference::TabsContent(TabContent::Status), &path)?;
    let text = fs::read_to_string(&path)?;
    let table: toml::Table = toml::from_str(&text)?;
    assert_eq!(table["tabs"]["position"].as_str(), Some("herdr"));
    assert_eq!(table["tabs"]["content"].as_str(), Some("status"));
    let style = Config::parse(&text)?.tabs_style;
    assert_eq!(style.placement, TabPlacement::Herdr);
    assert_eq!(style.content, TabContent::Status);
    Ok(())
}
