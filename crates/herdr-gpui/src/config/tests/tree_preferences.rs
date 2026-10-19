use super::*;
use crate::config::preferences::Preference;

/// Settings > Sidebar > Tree writes the four tree keys under `[sidebar]`,
/// and the file reads back to the same choices.
#[test]
fn tree_preferences_round_trip() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("local.toml");
    for edit in [
        Preference::SidebarHierarchy(TreeHierarchy::Block),
        Preference::SidebarTreeTabs(TreeTabs::Agents),
        Preference::SidebarStateText(true),
        Preference::SidebarAgentDetails(AgentDetails::Off),
    ] {
        Config::save_preference_path(edit, &path)?;
    }
    let text = fs::read_to_string(&path)?;
    let table: toml::Table = toml::from_str(&text)?;
    let sidebar = &table["sidebar"];
    assert_eq!(sidebar["hierarchy"].as_str(), Some("block"));
    assert_eq!(sidebar["tree_tabs"].as_str(), Some("agents"));
    assert_eq!(sidebar["state_text"].as_bool(), Some(true));
    assert_eq!(sidebar["agent_details"].as_str(), Some("off"));
    let style = Config::parse(&text)?.sidebar_style;
    assert_eq!(style.hierarchy, TreeHierarchy::Block);
    assert_eq!(style.tree_tabs, TreeTabs::Agents);
    assert!(style.state_text);
    assert_eq!(style.agent_details, AgentDetails::Off);
    Ok(())
}
