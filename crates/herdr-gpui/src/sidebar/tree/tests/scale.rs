use super::*;
use std::time::{Duration, Instant};

/// A snapshot with `spaces` spaces of `per_space` agents each, every agent
/// on its own pane and tab, in daemon order.
fn large(spaces: usize, per_space: usize) -> ClientShellSnapshot {
    let mut snapshot = snapshot();
    let workspace = snapshot.workspaces[0].clone();
    let tab = snapshot.tabs[0].clone();
    let pane = snapshot.panes[0].clone();
    let agent = snapshot.agents[0].clone();
    snapshot.workspaces.clear();
    snapshot.tabs.clear();
    snapshot.panes.clear();
    snapshot.agents.clear();
    for space in 0..spaces {
        let mut workspace = workspace.clone();
        workspace.workspace_id = format!("w{space}");
        workspace.label = format!("space {space}");
        snapshot.workspaces.push(workspace);
        let mut tab = tab.clone();
        tab.tab_id = format!("w{space}:t1");
        tab.workspace_id = format!("w{space}");
        snapshot.tabs.push(tab);
        for index in 0..per_space {
            let id = format!("w{space}:p{index}");
            let mut pane = pane.clone();
            pane.pane_id = id.clone();
            pane.workspace_id = format!("w{space}");
            pane.tab_id = format!("w{space}:t1");
            snapshot.panes.push(pane);
            let mut agent = agent.clone();
            agent.pane_id = id;
            agent.workspace_id = format!("w{space}");
            agent.tab_id = format!("w{space}:t1");
            agent.name = Some(format!("agent {space}.{index}"));
            agent.agent_status = [
                AgentStatus::Working,
                AgentStatus::Blocked,
                AgentStatus::Idle,
            ][index % 3];
            snapshot.agents.push(agent);
        }
    }
    snapshot
}

/// The plan's acceptance fixture: 20 hosts, 200 spaces, 1000 agents, every
/// sibling list ranked. Timing depends on the machine and profile, so this
/// runs only on request, in release: `cargo test --release --locked -p
/// herdr-gpui --bin herdr-personal-dev large_tree -- --ignored --nocapture`.
#[test]
#[ignore = "timing; run in release on the machine being measured"]
fn a_large_tree_projects_within_a_frame() {
    let snapshots: Vec<_> = (0..20).map(|_| large(10, 5)).collect();
    let keys: Vec<_> = (0..20)
        .map(|host| key(&format!("host{host}"), "local"))
        .collect();
    let mut preferences = TreePreferences::default();
    for (key, snapshot) in keys.iter().zip(&snapshots) {
        let context = context(key, "boot");
        let mut spaces: Vec<_> = snapshot
            .workspaces
            .iter()
            .map(|space| space.workspace_id.clone())
            .collect();
        spaces.reverse();
        preferences.space_orders.push(preferences::SpaceOrder {
            context: context.clone(),
            workspaces: spaces,
        });
        for space in &snapshot.workspaces {
            let mut panes: Vec<_> = snapshot
                .agents
                .iter()
                .filter(|agent| agent.workspace_id == space.workspace_id)
                .map(|agent| agent.pane_id.clone())
                .collect();
            panes.reverse();
            preferences.agent_orders.push(preferences::AgentOrder {
                context: context.clone(),
                workspace: space.workspace_id.clone(),
                panes,
            });
        }
    }
    let hosts: Vec<_> = keys
        .iter()
        .zip(&snapshots)
        .enumerate()
        .map(|(index, (key, snapshot))| host(index, key, "Host", Some(snapshot)))
        .collect();
    let mut times = Vec::new();
    let mut rows = 0;
    for _ in 0..200 {
        let start = Instant::now();
        // The default lists every tab, the most rows a tree can project.
        rows = std::hint::black_box(project(
            &hosts,
            &preferences,
            AgentSort::Grouped,
            "",
            TreeTabs::All,
        ))
        .len();
        times.push(start.elapsed());
    }
    times.sort();
    let p95 = times[times.len() * 95 / 100];
    println!(
        "large tree: {rows} rows, p50 {:?}, p95 {p95:?}, max {:?}",
        times[times.len() / 2],
        times[times.len() - 1]
    );
    assert_eq!(rows, 20 + 200 + 1000);
    assert!(p95 < Duration::from_millis(16), "p95 {p95:?}");
}
