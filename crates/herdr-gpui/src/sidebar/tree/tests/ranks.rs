use super::*;
use crate::sidebar::tree::preferences::{AgentOrder, Error, MAX_ID_BYTES, MAX_LISTS, SpaceOrder};

fn spaces(rows: &[TreeRow]) -> Vec<&str> {
    rows.iter()
        .filter_map(|row| match &row.id {
            NodeId::Space { workspace, .. } => Some(workspace.as_str()),
            _ => None,
        })
        .collect()
}

/// `w1` with a third agent, so a move has somewhere to land.
fn three_agents() -> ClientShellSnapshot {
    let mut s = snapshot();
    let mut pane = s.panes[0].clone();
    pane.pane_id = "p3".into();
    s.panes.push(pane);
    let mut agent = s.agents[0].clone();
    agent.pane_id = "p3".into();
    agent.name = Some("Test".into());
    s.agents.push(agent);
    s
}

#[test]
fn moving_a_sibling_reorders_it_against_what_the_tree_shows() {
    let k = key("local", "local");
    let s = three_agents();
    let hosts = [host(0, &k, "Local", Some(&s))];
    let mut prefs = TreePreferences::default();
    let sort = AgentSort::Grouped;
    let p = |pane| agent(&k, "boot", "w1", pane);
    // With nothing saved yet, the move still lands relative to every sibling.
    prefs
        .move_sibling(&hosts, sort, &p("p1"), &p("p3"), false)
        .unwrap();
    assert_eq!(panes(&grouped(&hosts, &prefs, "")), ["p2", "p3", "p1"]);
    prefs
        .move_sibling(&hosts, sort, &p("p1"), &p("p2"), true)
        .unwrap();
    assert_eq!(panes(&grouped(&hosts, &prefs, "")), ["p1", "p2", "p3"]);
    prefs
        .move_sibling(
            &hosts,
            sort,
            &space(&k, "boot", "w2"),
            &space(&k, "boot", "w1"),
            true,
        )
        .unwrap();
    assert_eq!(spaces(&grouped(&hosts, &prefs, "")), ["w2", "w1"]);
    assert_eq!(prefs.space_orders.len(), 1);
    assert_eq!(prefs.agent_orders.len(), 1);
    // The daemon order in the snapshot is untouched: it is never written.
    assert_eq!(s.workspaces[0].workspace_id, "w1");
}

#[test]
fn moves_outside_one_parent_or_one_boot_are_refused() {
    let k = key("local", "local");
    let other = key("remote", "local");
    let s = three_agents();
    let hosts = [
        host(0, &k, "Local", Some(&s)),
        host(1, &other, "Remote", Some(&s)),
    ];
    let mut prefs = TreePreferences::default();
    let sort = AgentSort::Grouped;
    let refused = |prefs: &mut TreePreferences, node: &NodeId, anchor: &NodeId| {
        prefs
            .move_sibling(&hosts, sort, node, anchor, true)
            .unwrap_err()
    };
    let a = agent(&k, "boot", "w1", "p1");
    assert_eq!(
        refused(&mut prefs, &a, &agent(&other, "boot", "w1", "p2")),
        Error::NotSiblings
    );
    assert_eq!(
        refused(&mut prefs, &a, &space(&k, "boot", "w1")),
        Error::NotSiblings
    );
    assert_eq!(refused(&mut prefs, &a, &a), Error::NotSiblings);
    assert_eq!(
        refused(
            &mut prefs,
            &NodeId::Host("local".into()),
            &NodeId::Host("remote".into())
        ),
        Error::HostOrder
    );
    assert_eq!(
        refused(
            &mut prefs,
            &agent(&k, "old", "w1", "p1"),
            &agent(&k, "old", "w1", "p2")
        ),
        Error::Unlisted
    );
    assert_eq!(
        refused(&mut prefs, &a, &agent(&k, "boot", "w1", "gone")),
        Error::Unlisted
    );
    assert_eq!(prefs, TreePreferences::default());
}

#[test]
fn a_plugin_view_or_attention_sort_refuses_agent_moves_but_not_space_moves() {
    let k = key("local", "local");
    let mut s = three_agents();
    let mut prefs = TreePreferences::default();
    let (a, b) = (agent(&k, "boot", "w1", "p1"), agent(&k, "boot", "w1", "p2"));
    let hosts = [host(0, &k, "Local", Some(&s))];
    assert_eq!(
        prefs.move_sibling(&hosts, AgentSort::Priority, &a, &b, false),
        Err(Error::AttentionSort)
    );
    s.agent_view_label = Some("Plugin".into());
    s.agent_order = vec!["p1".into(), "p2".into()];
    let hosts = [host(0, &k, "Local", Some(&s))];
    assert_eq!(
        prefs.move_sibling(&hosts, AgentSort::Grouped, &a, &b, false),
        Err(Error::PluginView)
    );
    prefs
        .move_sibling(
            &hosts,
            AgentSort::Grouped,
            &space(&k, "boot", "w2"),
            &space(&k, "boot", "w1"),
            true,
        )
        .unwrap();
    assert_eq!(spaces(&grouped(&hosts, &prefs, "")), ["w2", "w1"]);
}

#[test]
fn renaming_keeps_a_rank_and_new_or_removed_siblings_keep_the_others() {
    let k = key("local", "local");
    let mut s = three_agents();
    let hosts = [host(0, &k, "Local", Some(&s))];
    let mut prefs = TreePreferences::default();
    let p = |pane| agent(&k, "boot", "w1", pane);
    prefs
        .move_sibling(&hosts, AgentSort::Grouped, &p("p3"), &p("p1"), true)
        .unwrap();
    s.agents[2].name = Some("Renamed".into());
    s.workspaces[0].label = "Renamed space".into();
    let rows = |s: &ClientShellSnapshot| grouped(&[host(0, &k, "Local", Some(s))], &prefs, "");
    assert_eq!(panes(&rows(&s)), ["p3", "p1", "p2"]);
    s.agents.remove(1);
    assert_eq!(panes(&rows(&s)), ["p3", "p1"]);
    let mut fresh = s.agents[0].clone();
    fresh.pane_id = "p4".into();
    s.agents.insert(0, fresh);
    let mut pane = s.panes[0].clone();
    pane.pane_id = "p4".into();
    s.panes.push(pane);
    assert_eq!(panes(&rows(&s)), ["p3", "p1", "p4"]);
}

#[test]
fn reconcile_drops_another_boot_and_unlisted_ids_but_keeps_other_endpoints() {
    let k = key("local", "local");
    let remote = key("remote", "local");
    let ranked = |key: &EndpointKey, boot: &str| SpaceOrder {
        context: context(key, boot),
        workspaces: vec!["w2".into(), "gone".into(), "w2".into(), "w1".into()],
    };
    let mut prefs = TreePreferences {
        agent_orders: vec![
            AgentOrder {
                context: context(&k, "boot"),
                workspace: "w1".into(),
                panes: vec!["p2".into(), "gone".into()],
            },
            AgentOrder {
                context: context(&k, "boot"),
                workspace: "removed".into(),
                panes: vec!["p1".into()],
            },
            AgentOrder {
                context: context(&k, "old"),
                workspace: "w1".into(),
                panes: vec!["p1".into()],
            },
        ],
        collapsed_hosts: ["local".to_owned()].into(),
        space_orders: vec![
            ranked(&k, "boot"),
            ranked(&k, "old"),
            ranked(&remote, "old"),
        ],
        ..Default::default()
    };
    prefs.set_space_collapsed(&context(&k, "old"), "w1", true);
    prefs.set_space_collapsed(&context(&k, "boot"), "w1", true);
    prefs.set_space_collapsed(&context(&k, "boot"), "gone", true);
    prefs.set_space_collapsed(&context(&remote, "old"), "w1", true);
    assert!(prefs.reconcile(&k, &snapshot()));
    assert_eq!(
        prefs.space_orders,
        [
            SpaceOrder {
                context: context(&k, "boot"),
                workspaces: vec!["w2".into(), "w1".into()],
            },
            ranked(&remote, "old")
        ]
    );
    assert_eq!(
        prefs.agent_orders,
        [AgentOrder {
            context: context(&k, "boot"),
            workspace: "w1".into(),
            panes: vec!["p2".into()],
        }]
    );
    assert_eq!(prefs.collapsed_spaces.len(), 2);
    assert!(prefs.collapsed_hosts.contains("local"));
    // Nothing more to forget.
    assert!(!prefs.reconcile(&k, &snapshot()));
}

#[test]
fn version_one_round_trips_and_refuses_what_it_does_not_bound() {
    let k = key("local", "local");
    let mut prefs = TreePreferences::default();
    prefs.set_host_collapsed("ssh:box", true);
    prefs.set_space_collapsed(&context(&k, "boot"), "w1", true);
    prefs.space_orders.push(SpaceOrder {
        context: context(&k, "boot"),
        workspaces: vec!["w2".into(), "w1".into()],
    });
    let text = serde_json::to_string(&prefs).unwrap();
    assert_eq!(
        serde_json::from_str::<TreePreferences>(&text).unwrap(),
        prefs
    );
    assert!(prefs.validate().is_ok());
    // Unknown fields are a newer format, not something to drop silently.
    assert!(serde_json::from_str::<TreePreferences>(r#"{"extra":1}"#).is_err());
    let mut long = prefs.clone();
    long.collapsed_hosts.insert("x".repeat(MAX_ID_BYTES + 1));
    assert_eq!(long.validate(), Err(Error::TooManyEntries));
    let many = TreePreferences {
        collapsed_hosts: (0..=MAX_LISTS).map(|id| id.to_string()).collect(),
        ..Default::default()
    };
    assert_eq!(many.validate(), Err(Error::TooManyEntries));
    let mut empty = prefs;
    empty.space_orders[0].workspaces.push(String::new());
    assert_eq!(empty.validate(), Err(Error::TooManyEntries));
}
