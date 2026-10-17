use super::*;

/// The fixture with a third tab, `t3` "logs", in `w1` that no agent runs in.
/// `t2` in `w2` has no agent either.
fn with_logs() -> ClientShellSnapshot {
    let mut s = snapshot();
    let mut logs = s.tabs[0].clone();
    logs.tab_id = "t3".into();
    logs.number = 2;
    logs.label = "logs".into();
    s.tabs.push(logs);
    s
}

fn tab(key: &EndpointKey, boot: &str, workspace: &str, tab: &str) -> NodeId {
    NodeId::Tab {
        context: context(key, boot),
        workspace: workspace.into(),
        tab: tab.into(),
    }
}

fn all(hosts: &[HostInput<'_>], preferences: &TreePreferences, query: &str) -> Vec<TreeRow> {
    project(hosts, preferences, AgentSort::Grouped, query, TreeTabs::All)
}

#[test]
fn all_lists_agentless_tabs_after_the_agents() {
    let s = with_logs();
    let k = key("local", "local");
    let hosts = [host(0, &k, "Local", Some(&s))];
    let rows = all(&hosts, &TreePreferences::default(), "");
    assert_eq!(
        rows.iter().map(|row| row.entry).collect::<Vec<_>>(),
        [
            Entry::Host,
            Entry::Space(0),
            Entry::Agent(0),
            Entry::Agent(1),
            Entry::Tab(2),
            Entry::Space(1),
            Entry::Tab(1)
        ]
    );
    assert_eq!(
        labels(&hosts, &rows),
        ["Local", "API", "Review", "Implement", "logs", "Tools", "shell"]
    );
    assert_eq!(rows[4].id, tab(&k, "boot", "w1", "t3"));
    assert_eq!(rows[4].id.parent(), Some(space(&k, "boot", "w1")));
    // A terminal never asks for attention, so the counts stay the agents'.
    assert_eq!(
        rows.iter().map(|row| row.attention).collect::<Vec<_>>(),
        [1, 1, 0, 1, 0, 0, 0]
    );
}

#[test]
fn agents_mode_lists_no_tabs() {
    let s = with_logs();
    let k = key("local", "local");
    let hosts = [host(0, &k, "Local", Some(&s))];
    let rows = project(
        &hosts,
        &TreePreferences::default(),
        AgentSort::Grouped,
        "",
        TreeTabs::Agents,
    );
    assert!(!rows.iter().any(|row| matches!(row.entry, Entry::Tab(_))));
    assert_eq!(
        labels(&hosts, &rows),
        ["Local", "API", "Review", "Implement", "Tools"]
    );
}

#[test]
fn a_tab_with_an_agent_is_not_listed_twice() {
    let s = with_logs();
    let k = key("local", "local");
    let hosts = [host(0, &k, "Local", Some(&s))];
    let rows = all(&hosts, &TreePreferences::default(), "");
    // `t1` runs both agents: it shows through them, not as a row of its own.
    assert!(!rows.iter().any(|row| row.entry == Entry::Tab(0)));
    assert!(!rows.iter().any(|row| row.id == tab(&k, "boot", "w1", "t1")));
}

#[test]
fn search_finds_a_tab_by_label_and_keeps_its_ancestors() {
    let s = with_logs();
    let k = key("local", "local");
    let hosts = [host(0, &k, "Local", Some(&s))];
    let rows = all(&hosts, &TreePreferences::default(), "LOG");
    assert_eq!(labels(&hosts, &rows), ["Local", "API", "logs"]);
    // A matching space still shows all of its tabs.
    let rows = all(&hosts, &TreePreferences::default(), "tools");
    assert_eq!(labels(&hosts, &rows), ["Local", "Tools", "shell"]);
}

#[test]
fn folded_space_hides_its_tabs() {
    let s = with_logs();
    let k = key("local", "local");
    let hosts = [host(0, &k, "Local", Some(&s))];
    let mut preferences = TreePreferences::default();
    preferences.set_space_collapsed(&context(&k, "boot"), "w2", true);
    let rows = all(&hosts, &preferences, "");
    assert!(!rows.iter().any(|row| row.entry == Entry::Tab(1)));
    assert!(rows.iter().any(|row| row.entry == Entry::Tab(2)));
    // A search still shows inside the fold.
    let rows = all(&hosts, &preferences, "shell");
    assert_eq!(labels(&hosts, &rows), ["Local", "Tools", "shell"]);
}

#[test]
fn a_plugin_view_keeps_agentless_tabs() {
    let mut s = with_logs();
    s.agent_view_label = Some("review".into());
    s.agent_order = vec!["p2".into()];
    let k = key("local", "local");
    let hosts = [host(0, &k, "Local", Some(&s))];
    let rows = all(&hosts, &TreePreferences::default(), "");
    assert_eq!(
        labels(&hosts, &rows),
        ["Local", "API", "Implement", "logs", "Tools", "shell"]
    );
}

#[test]
fn tabs_cannot_be_moved() {
    let s = with_logs();
    let k = key("local", "local");
    let hosts = [host(0, &k, "Local", Some(&s))];
    let mut preferences = TreePreferences::default();
    let logs = tab(&k, "boot", "w1", "t3");
    let review = agent(&k, "boot", "w1", "p1");
    for (node, anchor) in [(&logs, &review), (&review, &logs)] {
        assert_eq!(
            preferences.move_sibling(&hosts, AgentSort::Grouped, node, anchor, true),
            Err(Error::Unmovable)
        );
    }
    assert_eq!(preferences, TreePreferences::default());
}
