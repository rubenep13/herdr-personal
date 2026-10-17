//! The Tree layout: each host's spaces in the user's order, each space's
//! agents nested under it. The spaces list draws the space rows as it does
//! for every layout; this adds what only the tree has: the agents under
//! each space, folding a space, and dragging a row among its siblings,
//! which only ever changes this client's own order.

use crate::{
    HerdrWindow,
    sidebar::{
        cell::{Fold, RowContext},
        label_text,
        layout::SidebarLook,
        tree::{
            self, EndpointKey, Entry, HostInput, NodeId, SnapshotKey, TreePreferences, TreeRow,
            storage::TreeState,
        },
    },
    window::Flash,
};
use gpui::{prelude::*, *};
use herdr_client::protocol::ClientShellSnapshot;
use std::collections::HashMap;

/// The projected tree, grouped the way the spaces list draws it.
pub(in crate::sidebar) struct TreeLayout {
    /// Each listed host's spaces, by endpoint index. A host with no space to
    /// show is still listed.
    hosts: HashMap<usize, Vec<TreeSpace>>,
    /// A search is narrowing the tree, which then shows inside folds and
    /// takes no drags.
    searching: bool,
}

struct TreeSpace {
    row: TreeRow,
    /// Its agents, then its agentless tabs when the tree lists them.
    children: Vec<TreeRow>,
}

impl TreeLayout {
    fn new(rows: Vec<TreeRow>, searching: bool) -> Self {
        let mut hosts: HashMap<usize, Vec<TreeSpace>> = HashMap::new();
        for row in rows {
            let spaces = hosts.entry(row.host).or_default();
            match row.entry {
                Entry::Host => {}
                Entry::Space(_) => spaces.push(TreeSpace {
                    row,
                    children: Vec::new(),
                }),
                Entry::Agent(_) | Entry::Tab(_) => {
                    if let Some(space) = spaces.last_mut() {
                        space.children.push(row);
                    }
                }
            }
        }
        Self { hosts, searching }
    }

    pub(in crate::sidebar) fn lists(&self, host: usize) -> bool {
        self.hosts.contains_key(&host)
    }

    /// The host's space rows, in the shape the spaces list iterates: none is
    /// indented or heads a repository group, since the user's order is not
    /// the daemon's.
    pub(in crate::sidebar) fn entries(&self, host: usize) -> Vec<(usize, bool, Option<String>)> {
        self.hosts
            .get(&host)
            .into_iter()
            .flatten()
            .filter_map(|space| match space.row.entry {
                Entry::Space(index) => Some((index, false, None)),
                _ => None,
            })
            .collect()
    }

    fn space(&self, host: usize, workspace: usize) -> Option<&TreeSpace> {
        self.hosts
            .get(&host)?
            .iter()
            .find(|space| space.row.entry == Entry::Space(workspace))
    }

    /// A folded space's name with how many of its agents want attention, so
    /// the count shows without unfolding it or moving the rows below.
    pub(in crate::sidebar) fn label(
        &self,
        host: usize,
        workspace: usize,
        name: &str,
    ) -> Option<String> {
        let space = self.space(host, workspace)?;
        (space.row.collapsed && space.row.attention > 0)
            .then(|| format!("{name} \u{00b7} {}", space.row.attention))
    }
}

/// A row being dragged among its siblings.
#[derive(Clone)]
pub(crate) struct TreeDrag {
    node: NodeId,
    label: SharedString,
    foreground: u32,
    surface: u32,
}

impl Render for TreeDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(8.))
            .py(px(2.))
            .rounded(px(4.))
            .bg(rgb(self.surface))
            .text_color(rgb(self.foreground))
            .child(label_text(&self.label))
    }
}

/// Whether `dragged` may be dropped on `target`: a sibling, never itself.
fn droppable(dragged: &NodeId, target: &NodeId) -> bool {
    dragged != target && dragged.parent().is_some() && dragged.parent() == target.parent()
}

impl HerdrWindow {
    /// The hosts the tree lists, in device order, each with the state it
    /// draws from: the selected host's live copy, the others' own.
    pub(crate) fn tree_hosts(&self) -> Vec<HostInput<'_>> {
        let multi = self.endpoints.len() > 1;
        self.displayed_devices()
            .into_iter()
            .filter(|(_, endpoint)| self.device_visible(&endpoint.id))
            .map(|(index, endpoint)| {
                let live = if index == self.selected_endpoint {
                    &self.live
                } else {
                    &endpoint.live
                };
                HostInput {
                    index,
                    key: EndpointKey::new(&endpoint.id, &endpoint.connection.target),
                    label: &endpoint.label,
                    connected: live.status.is_connected(),
                    collapsed: multi && endpoint.collapsed,
                    snapshot: live.snapshot.as_deref(),
                }
            })
            .collect()
    }

    /// The tree's rows for `query`, in the order it draws them.
    pub(crate) fn tree_rows(&self, query: &str, cx: &App) -> Vec<TreeRow> {
        let default = TreePreferences::default();
        let preferences = TreeState::of(cx).map_or(&default, TreeState::preferences);
        tree::project(
            &self.tree_hosts(),
            preferences,
            self.agent_sort,
            query,
            self.config.sidebar_style.tree_tabs,
        )
    }

    pub(in crate::sidebar) fn tree_layout(&self, query: &str, cx: &App) -> TreeLayout {
        TreeLayout::new(self.tree_rows(query, cx), !query.trim().is_empty())
    }

    /// The fold control of space `workspace` of endpoint `host`, while it has
    /// agents to hide or is folded.
    pub(in crate::sidebar) fn tree_fold(
        &self,
        tree: &TreeLayout,
        host: usize,
        workspace: usize,
        cx: &mut Context<Self>,
    ) -> Option<Fold> {
        let space = tree.space(host, workspace)?;
        let NodeId::Space {
            context,
            workspace: id,
        } = &space.row.id
        else {
            return None;
        };
        if !space.row.collapsed && space.children.is_empty() {
            return None;
        }
        let collapsed = space.row.collapsed;
        let (context, id) = (context.clone(), id.clone());
        Some(Fold {
            id: SharedString::from(format!("tree-fold-{}-{id}", context.endpoint.endpoint_id))
                .into(),
            index: workspace,
            collapsed,
            toggle: Box::new(cx.listener(move |_, _, _, cx| {
                cx.stop_propagation();
                TreeState::update(cx, |preferences| {
                    (
                        preferences.set_space_collapsed(&context, &id, !collapsed),
                        (),
                    )
                });
                cx.notify();
            })),
        })
    }

    /// Appends the agents of space `workspace` of endpoint `host` under its
    /// row, one level in, and returns the list with how many rows it added
    /// and which of them is the focused agent, for the reveal.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::sidebar) fn append_tree_agents(
        &self,
        mut list: Stateful<Div>,
        tree: &TreeLayout,
        (host, workspace): (usize, usize),
        snapshot: &ClientShellSnapshot,
        row_cx: &RowContext,
        look: SidebarLook,
        cx: &mut Context<Self>,
    ) -> (Stateful<Div>, usize, Option<usize>) {
        let Some(space) = tree.space(host, workspace) else {
            return (list, 0, None);
        };
        let nested = RowContext {
            indicators: row_cx.indicators,
            font: row_cx.font,
            theme: row_cx.theme,
            look,
            width: row_cx.width,
            nest: row_cx.nest + look.nest_indent(),
            mark: row_cx.mark,
            host: None,
        };
        let draggable = !tree.searching && tree::manual(snapshot, self.agent_sort);
        let mut count = 0;
        let mut focused = None;
        for row in &space.children {
            let Entry::Agent(index) = row.entry else {
                continue;
            };
            let Some(agent) = snapshot.agents.get(index) else {
                continue;
            };
            let Some(lines) = self.configured_agent_lines(agent, snapshot, None) else {
                continue;
            };
            if host == self.selected_endpoint && agent.focused {
                focused = Some(count);
            }
            let gap = self.agent_row_gap(count);
            let cell = self.agent_cell(host, agent, snapshot, &nested, lines, gap, cx);
            let label = crate::sidebar::agent_name(agent);
            list = list.child(self.tree_drag(cell, draggable.then_some(&row.id), label, cx));
            count += 1;
        }
        (list, count, focused)
    }

    /// The space row of `workspace`, dragged among its siblings while no
    /// search narrows the tree.
    pub(in crate::sidebar) fn tree_space_drag(
        &self,
        row: Stateful<Div>,
        tree: &TreeLayout,
        (host, workspace): (usize, usize),
        label: &str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let node = (!tree.searching)
            .then(|| tree.space(host, workspace))
            .flatten()
            .map(|space| &space.row.id);
        self.tree_drag(row, node, label, cx)
    }

    /// Lets `row` be dragged onto a sibling, and a sibling onto it. `None`
    /// leaves it as it is.
    fn tree_drag(
        &self,
        row: Stateful<Div>,
        node: Option<&NodeId>,
        label: &str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Some(node) = node else {
            return row;
        };
        let drag = TreeDrag {
            node: node.clone(),
            label: SharedString::from(label.to_owned()),
            foreground: self.theme.foreground,
            surface: self.theme.surface,
        };
        let (over, accepts, target) = (node.clone(), node.clone(), node.clone());
        let highlight = self.theme.active;
        row.on_drag(drag, |drag, _, _, cx| cx.new(|_| drag.clone()))
            .drag_over::<TreeDrag>(move |style, drag, _, _| {
                if droppable(&drag.node, &over) {
                    style.bg(rgb(highlight))
                } else {
                    style
                }
            })
            .can_drop(move |value, _, _| {
                value
                    .downcast_ref::<TreeDrag>()
                    .is_some_and(|drag| droppable(&drag.node, &accepts))
            })
            .on_drop(cx.listener(move |this, drag: &TreeDrag, _, cx| {
                this.drop_tree_node(&drag.node, &target, cx);
            }))
    }

    /// Moves `node` into `target`'s place among their siblings: after it when
    /// moving down the list, before it when moving up.
    pub(crate) fn drop_tree_node(
        &mut self,
        node: &NodeId,
        target: &NodeId,
        cx: &mut Context<Self>,
    ) {
        let Some(parent) = node.parent() else {
            return;
        };
        let result = {
            let hosts = self.tree_hosts();
            let default = TreePreferences::default();
            let preferences = TreeState::of(cx).map_or(&default, TreeState::preferences);
            let order = tree::children(&hosts, preferences, self.agent_sort, &parent);
            let place = |node: &NodeId| {
                order
                    .as_ref()?
                    .iter()
                    .position(|id| Some(id.as_str()) == node.sibling_id())
            };
            let before = match (place(node), place(target)) {
                (Some(from), Some(to)) => from > to,
                _ => true,
            };
            TreeState::update(cx, |preferences| {
                let result =
                    preferences.move_sibling(&hosts, self.agent_sort, node, target, before);
                (result.is_ok(), result)
            })
        };
        if let Err(error) = result {
            self.show_flash(Flash::warning(error.to_string()), cx);
        }
        cx.notify();
    }

    /// Moves the focused space, or the focused agent, one place up or down
    /// among its siblings, as the menu's commands do.
    pub(crate) fn move_tree_node(&mut self, agent: bool, up: bool, cx: &mut Context<Self>) {
        if !self.config.layout.mode.nests_agents() {
            self.show_flash(Flash::warning("Moving rows needs the Tree layout."), cx);
            return;
        }
        let neighbor = {
            let hosts = self.tree_hosts();
            let default = TreePreferences::default();
            let preferences = TreeState::of(cx).map_or(&default, TreeState::preferences);
            focused_node(&hosts, self.selected_endpoint, agent).and_then(|node| {
                let order = tree::children(&hosts, preferences, self.agent_sort, &node.parent()?)?;
                let at = order
                    .iter()
                    .position(|id| Some(id.as_str()) == node.sibling_id())?;
                let next = if up { at.checked_sub(1)? } else { at + 1 };
                let anchor = node.sibling(order.get(next)?)?;
                Some((node, anchor))
            })
        };
        // Already first or last, or nothing focused: nothing moves.
        if let Some((node, anchor)) = neighbor {
            self.drop_tree_node(&node, &anchor, cx);
        }
    }

    /// The storage notice, when the stored order is not what the user last
    /// arranged, as a muted line above the tree that a click dismisses.
    pub(in crate::sidebar) fn tree_notice(
        &self,
        look: SidebarLook,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let notice = TreeState::of(cx)?.notice()?.text();
        Some(
            div()
                .id("tree-notice")
                .debug_selector(|| "tree-notice".into())
                .w_full()
                .px(px(look.content_x()))
                .text_color(rgb(self.theme.muted))
                .cursor_pointer()
                .child(label_text(&notice))
                .on_click(cx.listener(|_, _, _, cx| {
                    TreeState::dismiss(cx);
                    cx.notify();
                })),
        )
    }

    /// A muted line naming the plugin view that filters a host's agents,
    /// whose order also holds while it does.
    pub(in crate::sidebar) fn tree_filter(
        &self,
        host: &str,
        snapshot: &ClientShellSnapshot,
        row_cx: &RowContext,
    ) -> Option<Div> {
        let label = snapshot.agent_view_label.as_deref()?;
        Some(
            div()
                .debug_selector(|| format!("tree-filter-{host}"))
                .w_full()
                .pl(px(row_cx.look.content_x() + row_cx.nest))
                .text_color(rgb(self.theme.muted))
                .truncate()
                .child(label_text(&format!("view: {label}"))),
        )
    }
}

/// The focused space or agent of endpoint `selected`, by its full identity.
fn focused_node(hosts: &[HostInput<'_>], selected: usize, agent: bool) -> Option<NodeId> {
    let host = hosts.iter().find(|host| host.index == selected)?;
    let snapshot = host.snapshot?;
    let context = SnapshotKey {
        endpoint: host.key.clone(),
        boot_id: snapshot.boot_id.clone(),
    };
    if agent {
        let pane = snapshot.focused_pane_id.as_deref()?;
        let agent = snapshot.agents.iter().find(|agent| agent.pane_id == pane)?;
        return Some(NodeId::Agent {
            context,
            workspace: agent.workspace_id.clone(),
            pane: pane.to_owned(),
        });
    }
    Some(NodeId::Space {
        context,
        workspace: snapshot.focused_workspace_id.clone()?,
    })
}
