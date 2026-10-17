#![allow(clippy::unwrap_used)]

use super::*;

mod projection;
mod ranks;
mod scale;
mod tabs;

/// Two spaces, two agents in the first: `p1` working, `p2` blocked.
pub(super) fn snapshot() -> ClientShellSnapshot {
    serde_json::from_value(serde_json::json!({
        "boot_id": "boot", "revision": 1, "update_install_command": "",
        "latest_release_notes_available": false, "integration_updates_available": false,
        "worktree_directory": "", "tab_bar_right": [], "tab_bar_right_separator": "",
        "agent_order": [], "commands": [],
        "workspaces": [
            {"workspace_id": "w1", "active_tab_id": "t1", "number": 1, "label": "API",
             "custom_label": false, "new_workspace_cwd": "/tmp", "tokens": [],
             "focused": false, "agent_status": "working"},
            {"workspace_id": "w2", "active_tab_id": "t2", "number": 2, "label": "Tools",
             "custom_label": false, "new_workspace_cwd": "/tmp", "tokens": [],
             "focused": false, "agent_status": "idle"}
        ],
        "tabs": [
            {"tab_id": "t1", "workspace_id": "w1", "number": 1, "label": "shell",
             "custom_label": false, "zoomed": false, "focused": false, "agent_status": "working"},
            {"tab_id": "t2", "workspace_id": "w2", "number": 1, "label": "shell",
             "custom_label": false, "zoomed": false, "focused": false, "agent_status": "idle"}
        ],
        "panes": [
            {"pane_id": "p1", "workspace_id": "w1", "tab_id": "t1", "focused": false,
             "right_click_passthrough": false},
            {"pane_id": "p2", "workspace_id": "w1", "tab_id": "t1", "focused": false,
             "right_click_passthrough": false}
        ],
        "agents": [
            {"pane_id": "p1", "workspace_id": "w1", "tab_id": "t1", "name": "Review",
             "agent_status": "working", "state_change_seq": 1, "state_labels": [],
             "tokens": [], "focused": false},
            {"pane_id": "p2", "workspace_id": "w1", "tab_id": "t1", "name": "Implement",
             "agent_status": "blocked", "state_change_seq": 2, "state_labels": [],
             "tokens": [], "focused": false}
        ]
    }))
    .unwrap()
}

pub(super) fn key(host: &str, session: &str) -> EndpointKey {
    EndpointKey {
        endpoint_id: host.into(),
        session: session.into(),
    }
}

pub(super) fn context(key: &EndpointKey, boot: &str) -> SnapshotKey {
    SnapshotKey {
        endpoint: key.clone(),
        boot_id: boot.into(),
    }
}

pub(super) fn host<'a>(
    index: usize,
    key: &EndpointKey,
    label: &'a str,
    snapshot: Option<&'a ClientShellSnapshot>,
) -> HostInput<'a> {
    HostInput {
        index,
        key: key.clone(),
        label,
        connected: true,
        collapsed: false,
        snapshot,
    }
}

pub(super) fn space(key: &EndpointKey, boot: &str, workspace: &str) -> NodeId {
    NodeId::Space {
        context: context(key, boot),
        workspace: workspace.into(),
    }
}

pub(super) fn agent(key: &EndpointKey, boot: &str, workspace: &str, pane: &str) -> NodeId {
    NodeId::Agent {
        context: context(key, boot),
        workspace: workspace.into(),
        pane: pane.into(),
    }
}

/// The rows' names, resolved through the hosts they came from.
pub(super) fn labels(hosts: &[HostInput<'_>], rows: &[TreeRow]) -> Vec<String> {
    rows.iter()
        .map(|row| {
            let host = hosts.iter().find(|host| host.index == row.host).unwrap();
            match row.entry {
                Entry::Host => host.label.to_owned(),
                Entry::Space(index) => host.snapshot.unwrap().workspaces[index].label.clone(),
                Entry::Agent(index) => {
                    crate::sidebar::agent_name(&host.snapshot.unwrap().agents[index]).to_owned()
                }
                Entry::Tab(index) => host.snapshot.unwrap().tabs[index].label.clone(),
            }
        })
        .collect()
}

pub(super) fn panes(rows: &[TreeRow]) -> Vec<&str> {
    rows.iter()
        .filter_map(|row| match &row.id {
            NodeId::Agent { pane, .. } => Some(pane.as_str()),
            _ => None,
        })
        .collect()
}

pub(super) fn grouped(
    hosts: &[HostInput<'_>],
    preferences: &TreePreferences,
    query: &str,
) -> Vec<TreeRow> {
    project(
        hosts,
        preferences,
        AgentSort::Grouped,
        query,
        TreeTabs::Agents,
    )
}
