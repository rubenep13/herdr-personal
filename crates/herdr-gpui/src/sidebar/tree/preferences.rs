//! The user's own order and folds for the tree, as this client stores them.
//!
//! Space and agent ranks, and space folds, belong to one daemon boot: after a
//! restart the daemon may hand a deleted entity's ID to a new one, so they are
//! dropped when that endpoint reports another boot, and the daemon's order
//! applies again. Host folds are keyed by the persistent endpoint ID, like the
//! device order. Nothing here holds a snapshot or terminal content.

use super::{EndpointKey, HostInput, NodeId, SnapshotKey, children};
use crate::preferences::AgentSort;
use herdr_client::protocol::ClientShellSnapshot;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Bounds on what one file holds: far more than one person arranges, small
/// enough that a damaged or hostile file cannot make every render slow.
pub(crate) const MAX_LISTS: usize = 4096;
pub(crate) const MAX_IDS: usize = 4096;
pub(crate) const MAX_ID_BYTES: usize = 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("Agents are sorted by attention; choose the grouped order to arrange them.")]
    AttentionSort,
    #[error("Hosts keep the device order; drag a host's header to move it.")]
    HostOrder,
    #[error("Only rows under the same parent can be reordered.")]
    NotSiblings,
    #[error("A plugin's agent view orders these agents.")]
    PluginView,
    #[error("The saved sidebar order holds more entries than this client keeps.")]
    TooManyEntries,
    #[error("That row is no longer listed.")]
    Unlisted,
    #[error("Tabs without an agent keep Herdr's order.")]
    Unmovable,
    #[error("The saved sidebar order has no version.")]
    Unversioned,
    #[error("The saved sidebar order has version {0}, which this client cannot read.")]
    UnknownVersion(u64),
}

/// A sibling order: the IDs the user arranged, first to last.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SpaceOrder {
    pub(crate) context: SnapshotKey,
    pub(crate) workspaces: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AgentOrder {
    pub(crate) context: SnapshotKey,
    pub(crate) workspace: String,
    pub(crate) panes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct SpaceRef {
    pub(crate) context: SnapshotKey,
    pub(crate) workspace: String,
}

/// Version 1 of the stored tree.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TreePreferences {
    #[serde(default)]
    pub(crate) agent_orders: Vec<AgentOrder>,
    #[serde(default)]
    pub(crate) collapsed_hosts: BTreeSet<String>,
    #[serde(default)]
    pub(crate) collapsed_spaces: BTreeSet<SpaceRef>,
    #[serde(default)]
    pub(crate) space_orders: Vec<SpaceOrder>,
}

/// Lookups over [`TreePreferences`] for one projection, built once rather
/// than searched per row.
pub(crate) struct Index<'a> {
    pub(super) agents: HashMap<(&'a SnapshotKey, &'a str), &'a [String]>,
    pub(super) collapsed: HashSet<(&'a SnapshotKey, &'a str)>,
    pub(super) spaces: HashMap<&'a SnapshotKey, &'a [String]>,
}

impl TreePreferences {
    pub(super) fn index(&self) -> Index<'_> {
        Index {
            agents: self
                .agent_orders
                .iter()
                .map(|order| {
                    (
                        (&order.context, order.workspace.as_str()),
                        order.panes.as_slice(),
                    )
                })
                .collect(),
            collapsed: self
                .collapsed_spaces
                .iter()
                .map(|space| (&space.context, space.workspace.as_str()))
                .collect(),
            spaces: self
                .space_orders
                .iter()
                .map(|order| (&order.context, order.workspaces.as_slice()))
                .collect(),
        }
    }

    /// Whether every list and ID is within this client's bounds, so a file
    /// that is not is refused rather than trusted.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        let lists = self.agent_orders.len()
            + self.space_orders.len()
            + self.collapsed_spaces.len()
            + self.collapsed_hosts.len();
        let key = |key: &SnapshotKey| {
            [
                &key.boot_id,
                &key.endpoint.endpoint_id,
                &key.endpoint.session,
            ]
            .into_iter()
            .all(|id| bounded(id))
        };
        let bounded_list =
            |ids: &[String]| ids.len() <= MAX_IDS && ids.iter().all(|id| bounded(id));
        let valid = lists <= MAX_LISTS
            && self.collapsed_hosts.iter().all(|id| bounded(id))
            && self.agent_orders.iter().all(|order| {
                key(&order.context) && bounded(&order.workspace) && bounded_list(&order.panes)
            })
            && self
                .space_orders
                .iter()
                .all(|order| key(&order.context) && bounded_list(&order.workspaces))
            && self
                .collapsed_spaces
                .iter()
                .all(|space| key(&space.context) && bounded(&space.workspace));
        if valid {
            Ok(())
        } else {
            Err(Error::TooManyEntries)
        }
    }

    /// Folds or unfolds a space. Returns whether anything changed.
    pub(crate) fn set_space_collapsed(
        &mut self,
        context: &SnapshotKey,
        workspace: &str,
        collapsed: bool,
    ) -> bool {
        let space = SpaceRef {
            context: context.clone(),
            workspace: workspace.to_owned(),
        };
        if collapsed {
            self.collapsed_spaces.insert(space)
        } else {
            self.collapsed_spaces.remove(&space)
        }
    }

    /// Records whether a host is folded. Returns whether anything changed.
    pub(crate) fn set_host_collapsed(&mut self, endpoint_id: &str, collapsed: bool) -> bool {
        if collapsed {
            self.collapsed_hosts.insert(endpoint_id.to_owned())
        } else {
            self.collapsed_hosts.remove(endpoint_id)
        }
    }

    /// Whether anything is stored for `endpoint`'s spaces or agents.
    pub(crate) fn mentions(&self, endpoint: &EndpointKey) -> bool {
        self.space_orders
            .iter()
            .map(|order| &order.context)
            .chain(self.agent_orders.iter().map(|order| &order.context))
            .chain(self.collapsed_spaces.iter().map(|space| &space.context))
            .any(|context| context.endpoint == *endpoint)
    }

    /// Forgets what no longer applies to `endpoint` now that it reports
    /// `snapshot`: every rank and fold of another boot of the same daemon,
    /// IDs the snapshot no longer lists, and duplicates. Returns whether
    /// anything changed.
    pub(crate) fn reconcile(
        &mut self,
        endpoint: &EndpointKey,
        snapshot: &ClientShellSnapshot,
    ) -> bool {
        let before = self.clone();
        let current = |context: &SnapshotKey| context.boot_id == snapshot.boot_id;
        let ours = |context: &SnapshotKey| context.endpoint == *endpoint;
        let spaces: HashSet<_> = snapshot
            .workspaces
            .iter()
            .map(|space| space.workspace_id.as_str())
            .collect();
        let panes: HashSet<_> = snapshot
            .agents
            .iter()
            .map(|agent| agent.pane_id.as_str())
            .collect();
        self.space_orders.retain_mut(|order| {
            if !ours(&order.context) {
                return true;
            }
            if !current(&order.context) {
                return false;
            }
            keep_listed(&mut order.workspaces, &spaces);
            !order.workspaces.is_empty()
        });
        self.agent_orders.retain_mut(|order| {
            if !ours(&order.context) {
                return true;
            }
            if !current(&order.context) || !spaces.contains(order.workspace.as_str()) {
                return false;
            }
            keep_listed(&mut order.panes, &panes);
            !order.panes.is_empty()
        });
        let mut seen = HashSet::new();
        self.agent_orders
            .retain(|order| seen.insert((order.context.clone(), order.workspace.clone())));
        let mut seen = HashSet::new();
        self.space_orders
            .retain(|order| seen.insert(order.context.clone()));
        self.collapsed_spaces.retain(|space| {
            !ours(&space.context)
                || (current(&space.context) && spaces.contains(space.workspace.as_str()))
        });
        *self != before
    }

    /// Moves `node` just before or after `anchor`, its sibling, in the order
    /// the tree shows them now. Spaces and agents only; the daemon's order
    /// is never touched.
    pub(crate) fn move_sibling(
        &mut self,
        hosts: &[HostInput<'_>],
        sort: AgentSort,
        node: &NodeId,
        anchor: &NodeId,
        before: bool,
    ) -> Result<(), Error> {
        if [node, anchor]
            .iter()
            .any(|id| matches!(id, NodeId::Tab { .. }))
        {
            return Err(Error::Unmovable);
        }
        let parent = node.parent().ok_or(Error::HostOrder)?;
        if anchor.parent().as_ref() != Some(&parent) || node == anchor {
            return Err(Error::NotSiblings);
        }
        if let NodeId::Agent { context, .. } = node {
            let snapshot = hosts
                .iter()
                .find(|host| host.key == context.endpoint)
                .and_then(|host| host.snapshot)
                .ok_or(Error::Unlisted)?;
            if snapshot.agent_view_label.is_some() {
                return Err(Error::PluginView);
            }
            if sort != AgentSort::Grouped {
                return Err(Error::AttentionSort);
            }
        }
        let (Some(moved), Some(target)) = (node.sibling_id(), anchor.sibling_id()) else {
            return Err(Error::HostOrder);
        };
        let mut order = children(hosts, self, sort, &parent).ok_or(Error::Unlisted)?;
        let from = order
            .iter()
            .position(|id| id == moved)
            .ok_or(Error::Unlisted)?;
        let id = order.remove(from);
        let to = order
            .iter()
            .position(|id| id == target)
            .ok_or(Error::Unlisted)?;
        order.insert(if before { to } else { to + 1 }, id);
        if order.len() > MAX_IDS || !order.iter().all(|id| bounded(id)) {
            return Err(Error::TooManyEntries);
        }
        match node {
            NodeId::Space { context, .. } => {
                if let Some(saved) = self
                    .space_orders
                    .iter_mut()
                    .find(|saved| saved.context == *context)
                {
                    saved.workspaces = order;
                    return Ok(());
                }
                self.room()?;
                self.space_orders.push(SpaceOrder {
                    context: context.clone(),
                    workspaces: order,
                });
            }
            NodeId::Agent {
                context, workspace, ..
            } => {
                if let Some(saved) = self
                    .agent_orders
                    .iter_mut()
                    .find(|saved| saved.context == *context && saved.workspace == *workspace)
                {
                    saved.panes = order;
                    return Ok(());
                }
                self.room()?;
                self.agent_orders.push(AgentOrder {
                    context: context.clone(),
                    workspace: workspace.clone(),
                    panes: order,
                });
            }
            NodeId::Host(_) => return Err(Error::HostOrder),
            NodeId::Tab { .. } => return Err(Error::Unmovable),
        }
        Ok(())
    }

    /// Whether another list fits within [`MAX_LISTS`].
    fn room(&self) -> Result<(), Error> {
        let lists = self.agent_orders.len()
            + self.space_orders.len()
            + self.collapsed_spaces.len()
            + self.collapsed_hosts.len();
        if lists < MAX_LISTS {
            Ok(())
        } else {
            Err(Error::TooManyEntries)
        }
    }
}

fn bounded(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_ID_BYTES
}

/// Keeps the first of each listed ID, in order.
fn keep_listed(ids: &mut Vec<String>, listed: &HashSet<&str>) {
    let mut seen = HashSet::new();
    ids.retain(|id| listed.contains(id.as_str()) && seen.insert(id.clone()));
}
