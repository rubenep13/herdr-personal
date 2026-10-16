use super::*;

#[test]
fn agents_nest_in_their_exact_space_and_empty_spaces_remain() {
    let s = snapshot();
    let k = key("local", "local");
    let hosts = [host(0, &k, "Local", Some(&s))];
    let rows = grouped(&hosts, &TreePreferences::default(), "");
    assert_eq!(
        labels(&hosts, &rows),
        ["Local", "API", "Review", "Implement", "Tools"]
    );
    assert_eq!(rows[0].id, NodeId::Host("local".into()));
    assert_eq!(rows[1].id, space(&k, "boot", "w1"));
    assert_eq!(rows[2].id, agent(&k, "boot", "w1", "p1"));
    assert_eq!(rows[2].id.parent(), Some(rows[1].id.clone()));
    assert_eq!(rows[1].id.parent(), Some(rows[0].id.clone()));
    assert_eq!(rows[4].id, space(&k, "boot", "w2"));
    assert_eq!(
        rows.iter().map(|row| row.entry).collect::<Vec<_>>(),
        [
            Entry::Host,
            Entry::Space(0),
            Entry::Agent(0),
            Entry::Agent(1),
            Entry::Space(1)
        ]
    );
    // Attention counts blocked and done agents, up to the host.
    assert_eq!(
        rows.iter().map(|row| row.attention).collect::<Vec<_>>(),
        [1, 1, 0, 1, 0]
    );
    assert!(rows.iter().all(|row| !row.stale && row.host == 0));
}

#[test]
fn repeated_ids_are_isolated_between_hosts_and_sessions() {
    let s = snapshot();
    let a = key("local", "one");
    let b = key("remote", "one");
    let c = key("local", "two");
    let prefs = TreePreferences::default();
    let first = |k: &EndpointKey| {
        grouped(&[host(0, k, "H", Some(&s))], &prefs, "")[2]
            .id
            .clone()
    };
    assert_ne!(first(&a), first(&b));
    assert_ne!(first(&a), first(&c));
    // A rank made for one session never applies to another of the same host.
    let ranked = TreePreferences {
        space_orders: vec![preferences::SpaceOrder {
            context: context(&a, "boot"),
            workspaces: vec!["w2".into(), "w1".into()],
        }],
        ..Default::default()
    };
    let hosts = [host(0, &a, "A", Some(&s)), host(1, &c, "C", Some(&s))];
    let rows = grouped(&hosts, &ranked, "");
    assert_eq!(
        labels(&hosts, &rows),
        [
            "A",
            "Tools",
            "API",
            "Review",
            "Implement",
            "C",
            "API",
            "Review",
            "Implement",
            "Tools"
        ]
    );
    let unique: HashSet<_> = rows.iter().map(|row| &row.id).collect();
    // Both hosts share the endpoint ID "local", so only their header repeats.
    assert_eq!(unique.len(), rows.len() - 1);
    assert_eq!(rows[5].host, 1);
}

#[test]
fn plugin_membership_and_order_are_authoritative_and_an_empty_view_lists_none() {
    let mut s = snapshot();
    s.agent_view_label = Some("Review only".into());
    s.agent_order = vec!["p2".into(), "missing".into(), "p2".into(), "p1".into()];
    let k = key("local", "local");
    let prefs = TreePreferences {
        agent_orders: vec![preferences::AgentOrder {
            context: context(&k, "boot"),
            workspace: "w1".into(),
            panes: vec!["p1".into(), "p2".into()],
        }],
        ..Default::default()
    };
    let rows = |s: &ClientShellSnapshot| grouped(&[host(0, &k, "Local", Some(s))], &prefs, "");
    assert_eq!(panes(&rows(&s)), ["p2", "p1"]);
    s.agent_order = vec!["p2".into()];
    assert_eq!(panes(&rows(&s)), ["p2"]);
    s.agent_order.clear();
    assert!(panes(&rows(&s)).is_empty());
    // Spaces stay listed under an empty view.
    assert_eq!(rows(&s).len(), 3);
}

#[test]
fn contradicting_or_missing_ancestry_never_creates_or_reassigns_agents() {
    let k = key("local", "local");
    let rows = |s: &ClientShellSnapshot| {
        grouped(
            &[host(0, &k, "Local", Some(s))],
            &TreePreferences::default(),
            "",
        )
    };
    let mut s = snapshot();
    // An unlisted pane leaves the agent in the workspace it names.
    s.panes.retain(|pane| pane.pane_id != "p1");
    assert_eq!(panes(&rows(&s)), ["p1", "p2"]);
    // A listed pane in another workspace contradicts the agent.
    s.agents[1].workspace_id = "w2".into();
    assert_eq!(panes(&rows(&s)), ["p1"]);
    // So does a listed tab of another workspace.
    s.agents[0].tab_id = "t2".into();
    assert!(panes(&rows(&s)).is_empty());
    // A workspace that is gone gives the agent no parent to sit under.
    let mut s = snapshot();
    s.panes.clear();
    s.tabs.clear();
    s.agents[0].workspace_id = "missing".into();
    let result = rows(&s);
    assert_eq!(panes(&result), ["p2"]);
    assert_eq!(result.len(), 4);
}

#[test]
fn search_keeps_ancestors_and_reveals_folded_nodes_without_unfolding_them() {
    let s = snapshot();
    let k = key("local", "local");
    let mut prefs = TreePreferences::default();
    prefs.set_space_collapsed(&context(&k, "boot"), "w1", true);
    let mut folded = host(0, &k, "Local", Some(&s));
    folded.collapsed = true;
    let hosts = [folded];
    let rows = grouped(&hosts, &prefs, "");
    assert_eq!(labels(&hosts, &rows), ["Local"]);
    assert_eq!(rows[0].attention, 1);
    assert!(rows[0].collapsed);
    let found = grouped(&hosts, &prefs, "  REVIEW ");
    assert_eq!(labels(&hosts, &found), ["Local", "API", "Review"]);
    assert!(found[1].collapsed);
    // A matching space lists all of its agents.
    let found = grouped(&hosts, &prefs, "api");
    assert_eq!(
        labels(&hosts, &found),
        ["Local", "API", "Review", "Implement"]
    );
    assert!(grouped(&hosts, &prefs, "nothing").is_empty());
}

#[test]
fn a_folded_space_hides_its_agents_and_keeps_their_attention() {
    let s = snapshot();
    let k = key("local", "local");
    let mut prefs = TreePreferences::default();
    prefs.set_space_collapsed(&context(&k, "boot"), "w1", true);
    let hosts = [host(0, &k, "Local", Some(&s))];
    let rows = grouped(&hosts, &prefs, "");
    assert_eq!(labels(&hosts, &rows), ["Local", "API", "Tools"]);
    assert!(rows[1].collapsed);
    assert_eq!(rows[1].attention, 1);
    // A fold made in another boot does not apply.
    let mut other = snapshot();
    other.boot_id = "next".into();
    let hosts = [host(0, &k, "Local", Some(&other))];
    assert_eq!(grouped(&hosts, &prefs, "").len(), 5);
}

#[test]
fn manual_order_holds_through_status_changes_and_a_new_boot_drops_it() {
    let k = key("local", "local");
    let mut s = snapshot();
    let prefs = TreePreferences {
        agent_orders: vec![preferences::AgentOrder {
            context: context(&k, "boot"),
            workspace: "w1".into(),
            panes: vec!["p2".into(), "p1".into()],
        }],
        space_orders: vec![preferences::SpaceOrder {
            context: context(&k, "boot"),
            workspaces: vec!["w2".into(), "w1".into()],
        }],
        ..Default::default()
    };
    let rows = |s: &ClientShellSnapshot| grouped(&[host(0, &k, "Local", Some(s))], &prefs, "");
    let before = rows(&s);
    assert_eq!(before[1].id, space(&k, "boot", "w2"));
    assert_eq!(panes(&before), ["p2", "p1"]);
    s.agents[0].agent_status = AgentStatus::Blocked;
    s.agents[1].agent_status = AgentStatus::Idle;
    s.agents[0].state_change_seq = 9;
    assert_eq!(panes(&rows(&s)), ["p2", "p1"]);
    s.boot_id = "replacement".into();
    let after = rows(&s);
    assert_eq!(after[1].id, space(&k, "replacement", "w1"));
    assert_eq!(panes(&after), ["p1", "p2"]);
}

#[test]
fn new_siblings_list_after_ranked_ones_in_the_daemon_order() {
    let k = key("local", "local");
    let mut s = snapshot();
    let mut third = s.workspaces[1].clone();
    third.workspace_id = "w3".into();
    third.label = "Docs".into();
    s.workspaces.push(third);
    let prefs = TreePreferences {
        space_orders: vec![preferences::SpaceOrder {
            context: context(&k, "boot"),
            workspaces: vec!["w2".into(), "gone".into()],
        }],
        ..Default::default()
    };
    let hosts = [host(0, &k, "Local", Some(&s))];
    let rows = grouped(&hosts, &prefs, "");
    let spaces: Vec<_> = labels(&hosts, &rows)
        .into_iter()
        .zip(&rows)
        .filter(|(_, row)| matches!(row.entry, Entry::Space(_)))
        .map(|(label, _)| label)
        .collect();
    assert_eq!(spaces, ["Tools", "API", "Docs"]);
}

#[test]
fn attention_sorts_within_each_space_and_never_overrides_a_plugin() {
    let k = key("local", "local");
    let mut s = snapshot();
    let prefs = TreePreferences {
        agent_orders: vec![preferences::AgentOrder {
            context: context(&k, "boot"),
            workspace: "w1".into(),
            panes: vec!["p1".into(), "p2".into()],
        }],
        ..Default::default()
    };
    let rows = |s: &ClientShellSnapshot| {
        project(
            &[host(0, &k, "Local", Some(s))],
            &prefs,
            AgentSort::Priority,
            "",
        )
    };
    assert_eq!(panes(&rows(&s)), ["p2", "p1"]);
    s.agent_view_label = Some("Plugin".into());
    s.agent_order = vec!["p1".into(), "p2".into()];
    assert_eq!(panes(&rows(&s)), ["p1", "p2"]);
}

#[test]
fn a_disconnected_host_keeps_its_last_state_marked_stale() {
    let k = key("local", "local");
    let s = snapshot();
    let mut offline = host(0, &k, "Local", Some(&s));
    offline.connected = false;
    let rows = grouped(&[offline], &TreePreferences::default(), "");
    assert_eq!(rows.len(), 5);
    assert!(rows.iter().all(|row| row.stale));
    let mut empty = host(0, &k, "Local", None);
    empty.connected = false;
    let hosts = [empty];
    assert_eq!(
        labels(&hosts, &grouped(&hosts, &TreePreferences::default(), "")),
        ["Local"]
    );
    assert!(grouped(&hosts, &TreePreferences::default(), "missing").is_empty());
}

#[test]
fn hosts_list_in_the_order_given_with_their_endpoint_index() {
    let a = key("a", "local");
    let b = key("b", "local");
    let s = snapshot();
    let hosts = [host(3, &b, "B", Some(&s)), host(1, &a, "A", Some(&s))];
    let rows = grouped(&hosts, &TreePreferences::default(), "");
    let headers: Vec<_> = rows
        .iter()
        .filter(|row| row.entry == Entry::Host)
        .map(|row| (row.id.clone(), row.host))
        .collect();
    assert_eq!(
        headers,
        [(NodeId::Host("b".into()), 3), (NodeId::Host("a".into()), 1)]
    );
}

#[test]
fn children_follow_the_shown_order_and_refuse_another_boot() {
    let k = key("local", "local");
    let s = snapshot();
    let hosts = [host(0, &k, "Local", Some(&s))];
    let prefs = TreePreferences::default();
    let sort = AgentSort::Grouped;
    assert_eq!(
        children(&hosts, &prefs, sort, &NodeId::Host("local".into())),
        Some(vec!["w1".to_owned(), "w2".to_owned()])
    );
    assert_eq!(
        children(&hosts, &prefs, sort, &space(&k, "boot", "w1")),
        Some(vec!["p1".to_owned(), "p2".to_owned()])
    );
    assert_eq!(
        children(&hosts, &prefs, sort, &space(&k, "old", "w1")),
        None
    );
    assert_eq!(
        children(&hosts, &prefs, sort, &space(&k, "boot", "w9")),
        None
    );
}

#[test]
fn endpoint_keys_tell_sessions_and_targets_apart() {
    use std::path::PathBuf;
    let keys = [
        ConnectTarget::Local,
        ConnectTarget::Session {
            name: "work".into(),
            development: false,
        },
        ConnectTarget::Session {
            name: "work".into(),
            development: true,
        },
        ConnectTarget::Socket(PathBuf::from("/tmp/work")),
        ConnectTarget::Ssh {
            target: "box".into(),
            session: "work".into(),
        },
        ConnectTarget::Ssh {
            target: "box".into(),
            session: "default".into(),
        },
    ]
    .map(|target| EndpointKey::new("device", &target));
    let unique: HashSet<_> = keys.iter().collect();
    assert_eq!(unique.len(), keys.len());
    assert!(keys.iter().all(|key| key.endpoint_id == "device"));
}
