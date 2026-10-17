//! The personal tree: each host's spaces, and each space's agents under it,
//! then, when asked, the space's tabs that no agent runs in.
//!
//! A pure projection of the snapshots the window already holds, with no GPUI,
//! transport, or filesystem access. Hosts arrive in the device order the
//! sidebar already keeps; spaces and agents follow the user's own order for
//! the exact daemon boot it was made against, so an ID the daemon reuses after
//! a restart never inherits another entity's place. Reordering here never
//! reaches the daemon: `workspace.move` would rewrite the order every client
//! shares.

use super::agents::sorted_agents;
use crate::{config::TreeTabs, preferences::AgentSort};
use herdr_client::{
    ConnectTarget,
    protocol::{AgentStatus, ClientShellAgent, ClientShellSnapshot},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub(crate) mod preferences;
pub(crate) mod storage;

pub(crate) use preferences::{Error, TreePreferences};

/// One daemon: the sidebar's device, and the session it is attached to.
/// Workspace and pane IDs are only unique within one.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct EndpointKey {
    pub(crate) endpoint_id: String,
    pub(crate) session: String,
}

impl EndpointKey {
    pub(crate) fn new(endpoint_id: &str, target: &ConnectTarget) -> Self {
        let session = match target {
            ConnectTarget::Local => "local".to_owned(),
            ConnectTarget::Session {
                name,
                development: false,
            } => format!("session:{name}"),
            ConnectTarget::Session {
                name,
                development: true,
            } => format!("dev-session:{name}"),
            ConnectTarget::Socket(path) => format!("socket:{}", path.display()),
            ConnectTarget::Ssh { session, .. } | ConnectTarget::Wsl { session, .. } => {
                format!("remote:{session}")
            }
            #[cfg(feature = "cloud")]
            ConnectTarget::Cloud { session, .. } => format!("remote:{session}"),
        };
        Self {
            endpoint_id: endpoint_id.to_owned(),
            session,
        }
    }
}

/// One boot of one daemon: the only scope in which the daemon's public IDs
/// are known to name the same entity. Herdr 0.9.4 restores public IDs after a
/// restart, but hands a deleted highest workspace ID to the next workspace
/// (`identity::` live tests in `herdr-client`), so an ID alone is not enough.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct SnapshotKey {
    pub(crate) endpoint: EndpointKey,
    pub(crate) boot_id: String,
}

/// A row's identity, complete enough that a handler never resolves its
/// target by label or row position.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum NodeId {
    /// An agent by its pane, in the workspace the snapshot places it.
    Agent {
        context: SnapshotKey,
        workspace: String,
        pane: String,
    },
    /// A host by its persistent endpoint ID, as the device order keys it.
    Host(String),
    Space {
        context: SnapshotKey,
        workspace: String,
    },
    /// A tab no agent runs in. It keeps Herdr's order and is never moved.
    Tab {
        context: SnapshotKey,
        workspace: String,
        tab: String,
    },
}

impl NodeId {
    /// The ID that tells it from its siblings; hosts are not ranked here.
    pub(crate) fn sibling_id(&self) -> Option<&str> {
        match self {
            Self::Agent { pane, .. } => Some(pane),
            Self::Host(_) => None,
            Self::Space { workspace, .. } => Some(workspace),
            Self::Tab { tab, .. } => Some(tab),
        }
    }

    /// Its sibling named `id`.
    pub(crate) fn sibling(&self, id: &str) -> Option<Self> {
        match self {
            Self::Agent {
                context, workspace, ..
            } => Some(Self::Agent {
                context: context.clone(),
                workspace: workspace.clone(),
                pane: id.to_owned(),
            }),
            Self::Host(_) => None,
            Self::Space { context, .. } => Some(Self::Space {
                context: context.clone(),
                workspace: id.to_owned(),
            }),
            Self::Tab {
                context, workspace, ..
            } => Some(Self::Tab {
                context: context.clone(),
                workspace: workspace.clone(),
                tab: id.to_owned(),
            }),
        }
    }

    pub(crate) fn parent(&self) -> Option<Self> {
        match self {
            Self::Agent {
                context, workspace, ..
            }
            | Self::Tab {
                context, workspace, ..
            } => Some(Self::Space {
                context: context.clone(),
                workspace: workspace.clone(),
            }),
            Self::Host(_) => None,
            Self::Space { context, .. } => Some(Self::Host(context.endpoint.endpoint_id.clone())),
        }
    }
}

/// A host as the window lists it, in device order.
pub(crate) struct HostInput<'a> {
    /// The endpoint's index in the window, handed back on every row.
    pub(crate) index: usize,
    pub(crate) key: EndpointKey,
    pub(crate) label: &'a str,
    pub(crate) connected: bool,
    pub(crate) collapsed: bool,
    /// The last snapshot, kept through a disconnection and marked stale.
    pub(crate) snapshot: Option<&'a ClientShellSnapshot>,
}

/// What a row shows, by index into its host's snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Entry {
    Agent(usize),
    Host,
    Space(usize),
    /// A tab no agent runs in, by its index in the snapshot's tabs.
    Tab(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TreeRow {
    pub(crate) id: NodeId,
    /// The host's endpoint index, from [`HostInput::index`].
    pub(crate) host: usize,
    pub(crate) entry: Entry,
    /// Agents under the node that are blocked or done, whether shown or folded.
    pub(crate) attention: usize,
    /// A folded host or space. A search shows matches inside it regardless.
    pub(crate) collapsed: bool,
    /// The host is not connected: the rows are its last known state.
    pub(crate) stale: bool,
}

/// The tree's rows, top to bottom. `query` keeps the nodes whose name holds
/// it, with their ancestors, and their descendants for a host or space.
/// `tabs` decides whether a space's agentless tabs follow its agents.
pub(crate) fn project(
    hosts: &[HostInput<'_>],
    preferences: &TreePreferences,
    sort: AgentSort,
    query: &str,
    tabs: TreeTabs,
) -> Vec<TreeRow> {
    let query = query.trim().to_lowercase();
    let matches = |text: &str| query.is_empty() || text.to_lowercase().contains(&query);
    let orders = preferences.index();
    let mut rows = Vec::new();
    for host in hosts {
        let host_matches = matches(host.label);
        let header = rows.len();
        rows.push(TreeRow {
            id: NodeId::Host(host.key.endpoint_id.clone()),
            host: host.index,
            entry: Entry::Host,
            attention: 0,
            collapsed: host.collapsed,
            stale: !host.connected,
        });
        let mut attention = 0;
        let mut shown = false;
        if let Some(snapshot) = host.snapshot {
            let context = SnapshotKey {
                endpoint: host.key.clone(),
                boot_id: snapshot.boot_id.clone(),
            };
            let mut agents = agents_by_space(snapshot, sort);
            let mut terminals = match tabs {
                TreeTabs::All => agentless_tabs(snapshot),
                TreeTabs::Agents => HashMap::new(),
            };
            for (position, space) in ordered_spaces(snapshot, &context, &orders) {
                let workspace = space.workspace_id.as_str();
                let mut listed = agents.remove(workspace).unwrap_or_default();
                if manual(snapshot, sort)
                    && let Some(order) = orders.agents.get(&(&context, workspace))
                {
                    rank(&mut listed, order, |(_, agent)| agent.pane_id.as_str());
                }
                let space_attention = listed
                    .iter()
                    .filter(|(_, agent)| needs_attention(agent.agent_status))
                    .count();
                attention += space_attention;
                let space_matches = host_matches || matches(&space.label);
                let found: Vec<_> = listed
                    .into_iter()
                    .filter(|(_, agent)| {
                        space_matches
                            || super::agents::agent_names(agent)
                                .into_iter()
                                .flatten()
                                .any(&matches)
                    })
                    .collect();
                let found_tabs: Vec<_> = terminals
                    .remove(workspace)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|(_, tab)| space_matches || matches(&tab.label))
                    .collect();
                if !space_matches && found.is_empty() && found_tabs.is_empty() {
                    continue;
                }
                shown = true;
                let id = NodeId::Space {
                    context: context.clone(),
                    workspace: workspace.to_owned(),
                };
                let collapsed = orders.collapsed.contains(&(&context, workspace));
                let open = !collapsed || !query.is_empty();
                if host.collapsed && query.is_empty() {
                    continue;
                }
                rows.push(TreeRow {
                    id,
                    host: host.index,
                    entry: Entry::Space(position),
                    attention: space_attention,
                    collapsed,
                    stale: !host.connected,
                });
                if !open {
                    continue;
                }
                rows.extend(found.into_iter().map(|(index, agent)| TreeRow {
                    id: NodeId::Agent {
                        context: context.clone(),
                        workspace: workspace.to_owned(),
                        pane: agent.pane_id.clone(),
                    },
                    host: host.index,
                    entry: Entry::Agent(index),
                    attention: usize::from(needs_attention(agent.agent_status)),
                    collapsed: false,
                    stale: !host.connected,
                }));
                rows.extend(found_tabs.into_iter().map(|(index, tab)| TreeRow {
                    id: NodeId::Tab {
                        context: context.clone(),
                        workspace: workspace.to_owned(),
                        tab: tab.tab_id.clone(),
                    },
                    host: host.index,
                    entry: Entry::Tab(index),
                    attention: 0,
                    collapsed: false,
                    stale: !host.connected,
                }));
            }
        }
        if !host_matches && !shown {
            rows.truncate(header);
            continue;
        }
        rows[header].attention = attention;
    }
    rows
}

/// The IDs of a node's children in the order the tree shows them, ignoring
/// any search and fold. `None` for a node the hosts do not list.
pub(crate) fn children(
    hosts: &[HostInput<'_>],
    preferences: &TreePreferences,
    sort: AgentSort,
    parent: &NodeId,
) -> Option<Vec<String>> {
    let orders = preferences.index();
    let (context, workspace) = match parent {
        NodeId::Agent { .. } | NodeId::Tab { .. } => return None,
        NodeId::Host(_) => (parent_context(hosts, parent)?, None),
        NodeId::Space { context, workspace } => (context.clone(), Some(workspace.as_str())),
    };
    let snapshot = hosts
        .iter()
        .find(|host| host.key == context.endpoint)?
        .snapshot
        .filter(|snapshot| snapshot.boot_id == context.boot_id)?;
    let spaces = ordered_spaces(snapshot, &context, &orders);
    let Some(workspace) = workspace else {
        return Some(
            spaces
                .into_iter()
                .map(|(_, space)| space.workspace_id.clone())
                .collect(),
        );
    };
    spaces
        .iter()
        .any(|(_, space)| space.workspace_id == workspace)
        .then_some(())?;
    let mut listed = agents_by_space(snapshot, sort)
        .remove(workspace)
        .unwrap_or_default();
    if manual(snapshot, sort)
        && let Some(order) = orders.agents.get(&(&context, workspace))
    {
        rank(&mut listed, order, |(_, agent)| agent.pane_id.as_str());
    }
    Some(
        listed
            .into_iter()
            .map(|(_, agent)| agent.pane_id.clone())
            .collect(),
    )
}

/// The context a host's spaces are ranked in: its key and current boot.
fn parent_context(hosts: &[HostInput<'_>], host: &NodeId) -> Option<SnapshotKey> {
    let host = hosts
        .iter()
        .find(|input| NodeId::Host(input.key.endpoint_id.clone()) == *host)?;
    Some(SnapshotKey {
        endpoint: host.key.clone(),
        boot_id: host.snapshot?.boot_id.clone(),
    })
}

/// Whether the user's own agent order applies: never under a plugin's view,
/// whose order is authoritative, nor while attention sorts the agents.
pub(crate) fn manual(snapshot: &ClientShellSnapshot, sort: AgentSort) -> bool {
    snapshot.agent_view_label.is_none() && sort == AgentSort::Grouped
}

/// The spaces in the user's order, each with its index in the snapshot.
fn ordered_spaces<'a>(
    snapshot: &'a ClientShellSnapshot,
    context: &SnapshotKey,
    orders: &preferences::Index<'_>,
) -> Vec<(usize, &'a herdr_client::protocol::ClientShellWorkspace)> {
    let mut spaces: Vec<_> = snapshot.workspaces.iter().enumerate().collect();
    if let Some(order) = orders.spaces.get(context) {
        rank(&mut spaces, order, |(_, space)| space.workspace_id.as_str());
    }
    spaces
}

/// Each space's agents in the order the agents panel would list them, so a
/// plugin view keeps its membership and relative order. An agent whose pane
/// or tab the snapshot places in another workspace has no place in the tree
/// rather than a guessed one; one whose workspace is gone has no parent.
fn agents_by_space(
    snapshot: &ClientShellSnapshot,
    sort: AgentSort,
) -> HashMap<&str, Vec<(usize, &ClientShellAgent)>> {
    let panes: HashMap<_, _> = snapshot
        .panes
        .iter()
        .map(|pane| (pane.pane_id.as_str(), pane))
        .collect();
    let tabs: HashMap<_, _> = snapshot
        .tabs
        .iter()
        .map(|tab| (tab.tab_id.as_str(), tab))
        .collect();
    let index: HashMap<_, _> = snapshot
        .agents
        .iter()
        .enumerate()
        .map(|(index, agent)| (agent.pane_id.as_str(), index))
        .rev()
        .collect();
    let mut seen = HashSet::new();
    let mut spaces: HashMap<&str, Vec<_>> = HashMap::new();
    for agent in sorted_agents(snapshot, sort) {
        // The agent's own workspace places it. A pane or tab the snapshot
        // lists elsewhere contradicts it, and then it has no place at all.
        let placed = panes.get(agent.pane_id.as_str()).is_none_or(|pane| {
            pane.workspace_id == agent.workspace_id && pane.tab_id == agent.tab_id
        }) && tabs
            .get(agent.tab_id.as_str())
            .is_none_or(|tab| tab.workspace_id == agent.workspace_id);
        if !placed || !seen.insert(agent.pane_id.as_str()) {
            continue;
        }
        let Some(&position) = index.get(agent.pane_id.as_str()) else {
            continue;
        };
        spaces
            .entry(agent.workspace_id.as_str())
            .or_default()
            .push((position, agent));
    }
    spaces
}

/// Each space's tabs that no agent of the snapshot runs in, in the
/// snapshot's order with their index. Agents are counted before any plugin
/// view filters them, so a tab whose agent a view hides is not a terminal.
fn agentless_tabs(
    snapshot: &ClientShellSnapshot,
) -> HashMap<&str, Vec<(usize, &herdr_client::protocol::ClientShellTab)>> {
    let busy: HashSet<_> = snapshot
        .agents
        .iter()
        .map(|agent| agent.tab_id.as_str())
        .collect();
    let mut spaces: HashMap<&str, Vec<_>> = HashMap::new();
    for (index, tab) in snapshot.tabs.iter().enumerate() {
        if !busy.contains(tab.tab_id.as_str()) {
            spaces
                .entry(tab.workspace_id.as_str())
                .or_default()
                .push((index, tab));
        }
    }
    spaces
}

/// Sorts `items` by their place in `order`, keeping the rest after them in
/// the order they came, so a sibling created since the user arranged them
/// lists last. The first of repeated IDs wins.
fn rank<T>(items: &mut [T], order: &[String], id: impl Fn(&T) -> &str) {
    let ranks: HashMap<_, _> = order
        .iter()
        .enumerate()
        .rev()
        .map(|(rank, id)| (id.as_str(), rank))
        .collect();
    items.sort_by_key(|item| ranks.get(id(item)).copied().unwrap_or(usize::MAX));
}

pub(crate) fn needs_attention(status: AgentStatus) -> bool {
    matches!(status, AgentStatus::Blocked | AgentStatus::Done)
}

#[cfg(test)]
mod tests;
