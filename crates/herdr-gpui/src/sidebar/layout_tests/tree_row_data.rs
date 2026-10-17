use super::*;
use crate::{
    icons::AgentIcon,
    sidebar::{agent_name, agents::tree_agent_titles},
};

#[test]
fn tree_agent_rows_take_the_tab_label_and_the_agent_kind() {
    let mut listing = snapshot(3);
    listing.tabs[0].label = "visor-git".into();
    listing.agents[0].display_agent = None;
    assert_eq!(
        tree_agent_titles(&listing.agents[0], &listing),
        ("visor-git", "Claude")
    );
    // An integration's own display name wins over the identity's.
    listing.agents[0].display_agent = Some("Claude Code".into());
    assert_eq!(
        tree_agent_titles(&listing.agents[0], &listing),
        ("visor-git", "Claude Code")
    );
    // The tab names the row even when it is the space's only tab.
    listing.tabs.truncate(1);
    assert_eq!(
        tree_agent_titles(&listing.agents[0], &listing).0,
        "visor-git"
    );
    // An agent whose tab the snapshot does not list leads with its own name.
    let lost = &listing.agents[1];
    assert_eq!(tree_agent_titles(lost, &listing).0, agent_name(lost));
}

#[test]
fn agent_kinds_have_a_display_label_and_claude_keeps_its_brand() {
    for (icon, label) in [
        (AgentIcon::Claude, "Claude"),
        (AgentIcon::Codex, "Codex"),
        (AgentIcon::OpenCode, "OpenCode"),
        (AgentIcon::Generic, "Agent"),
    ] {
        assert_eq!(icon.label(), label);
    }
    assert_eq!(AgentIcon::Claude.brand(), Some(0xd97757));
    assert_eq!(AgentIcon::Codex.brand(), None);
    assert_eq!(AgentIcon::Generic.brand(), None);
}
