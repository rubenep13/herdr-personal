use super::*;
use crate::config::{AgentDetails, TreeHierarchy, TreeTabs};

#[test]
fn tree_options_default_to_the_spec() -> anyhow::Result<()> {
    for config in [
        Config::default(),
        Config::parse("")?,
        Config::parse(DEFAULT_CONFIG)?,
    ] {
        let style = &config.sidebar_style;
        assert_eq!(style.hierarchy, TreeHierarchy::Guide);
        assert_eq!(style.tree_tabs, TreeTabs::All);
        assert!(!style.state_text);
        assert_eq!(style.agent_details, AgentDetails::Herdr);
    }
    Ok(())
}

#[test]
fn tree_options_parse_every_value() -> anyhow::Result<()> {
    let style = |text: &str| -> anyhow::Result<SidebarStyle> {
        Ok(Config::parse(&format!("[sidebar]\n{text}"))?.sidebar_style)
    };
    for (name, hierarchy) in [
        ("indent", TreeHierarchy::Indent),
        ("block", TreeHierarchy::Block),
        ("guide", TreeHierarchy::Guide),
    ] {
        assert_eq!(style(&format!("hierarchy = \"{name}\""))?.hierarchy, hierarchy);
    }
    for (name, tabs) in [("all", TreeTabs::All), ("agents", TreeTabs::Agents)] {
        assert_eq!(style(&format!("tree_tabs = \"{name}\""))?.tree_tabs, tabs);
    }
    assert!(style("state_text = true")?.state_text);
    assert!(!style("state_text = false")?.state_text);
    for (name, details) in [("herdr", AgentDetails::Herdr), ("off", AgentDetails::Off)] {
        assert_eq!(
            style(&format!("agent_details = \"{name}\""))?.agent_details,
            details
        );
    }
    // The new keys sit beside the font and spacing keys without displacing them.
    let config = Config::parse("[sidebar]\nsize = 14\nindent = 20\nhierarchy = \"block\"")?;
    assert_eq!(config.sidebar.size, 14.);
    assert_eq!(config.sidebar_style.overrides.indent, Some(20.));
    assert_eq!(config.sidebar_style.hierarchy, TreeHierarchy::Block);
    Ok(())
}

#[test]
fn tree_options_refuse_unknown_values() {
    for text in [
        "hierarchy = \"tree\"",
        "tree_tabs = \"some\"",
        "state_text = \"yes\"",
        "agent_details = \"daemon\"",
    ] {
        assert!(Config::parse(&format!("[sidebar]\n{text}")).is_err(), "{text}");
    }
}
