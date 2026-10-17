//! The Git graph in a tab: opening it for the focused workspace's repository,
//! restoring it after a restart, and feeding it the window's look from the
//! tick. The view itself is an entity that knows nothing of tabs.

use super::{GitGraphView, GraphCheckout, GraphLook, view::Connection};
use crate::{
    HerdrWindow, NavigationTarget,
    browser::{Location, Pick, Slot, Store, Tab, TabId},
    teleport::Host,
    window::Flash,
};
use gpui::{prelude::*, *};

/// An open Git graph tab's view, and the host its repository is on.
pub(crate) struct GitGraph {
    view: Entity<GitGraphView>,
    endpoint: String,
}

#[cfg(test)]
impl GitGraph {
    pub(crate) fn view(&self) -> &Entity<GitGraphView> {
        &self.view
    }
}

impl HerdrWindow {
    /// Opens the focused workspace's Git graph tab, or brings back the one
    /// already open.
    pub(crate) fn open_git_graph(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, workspace)) = self.browser_key() else {
            self.show_flash(Flash::warning("Open a workspace first"), cx);
            return;
        };
        let folder = self.focused_checkout();
        self.open_git_graph_in(workspace, folder, window, cx);
    }

    /// Opens workspace `workspace`'s Git graph tab, showing the repository
    /// `folder` is in. Another workspace than the focused one is switched
    /// to, and its tab shows once it is.
    pub(crate) fn open_git_graph_in(
        &mut self,
        workspace: String,
        folder: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.orchestrator_host() {
            self.show_flash(
                Flash::warning("The Git graph needs a local or SSH host"),
                cx,
            );
            return;
        }
        let scope = crate::browser::scope(&self.endpoints[self.selected_endpoint]);
        let Some(repo) = folder.and_then(|path| GraphCheckout::new(path).ok()) else {
            self.show_flash(Flash::warning("This workspace has no folder to read"), cx);
            return;
        };
        let existing = cx.try_global::<Store>().and_then(|store| {
            store
                .in_workspace(&scope, &workspace)
                .find(|tab| matches!(&tab.location, Some(Location::GitGraph { .. })))
                .map(|tab| tab.id)
        });
        let opened = existing.or_else(|| {
            Store::update(cx, |store| {
                store.open(scope, &workspace, Some(Location::GitGraph { repo }), None)
            })
        });
        let Some(id) = opened else {
            self.show_flash(Flash::warning("Too many tabs are open"), cx);
            return;
        };
        self.dismiss_menu(window, cx);
        if self
            .browser_key()
            .is_none_or(|(_, focused)| focused != workspace)
        {
            self.navigate(NavigationTarget::Workspace(&workspace), cx);
            return;
        }
        self.ensure_git_graph(id, cx);
        // A workspace has one graph tab: asked for again, it moves to the
        // group in use.
        match self.ensure_layout().map(|layout| layout.active()) {
            Some(group) => self.move_tab_to_group(Pick::Page(id), group, window, cx),
            None => self.show_browser_tab(id, window, cx),
        }
        if let Some(graph) = self.git_graphs.get(&id) {
            let focus = graph.view.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        }
    }

    /// Whether the focused workspace's repository can be drawn: a host
    /// scripts run on, and a workspace with a folder.
    pub(crate) fn can_open_git_graph(&self) -> bool {
        self.orchestrator_host()
            && self.browser_key().is_some()
            && self.focused_checkout().is_some()
    }

    /// The graph's look: the UI font's family at the terminal font's size,
    /// so Cmd-+ and Cmd-- scale it with the terminals.
    fn graph_look(&self) -> GraphLook {
        let mut ui = self.config.ui.clone();
        ui.size = self.config.terminal.size;
        GraphLook {
            theme: self.theme.clone(),
            ui,
            mono: self.config.terminal.clone(),
        }
    }

    /// Whether the host at `index` can be asked anything: an SSH host only
    /// while it is connected; this machine always, since Git runs here
    /// whether or not its daemon answers.
    fn graph_connection(&self, index: usize) -> Connection {
        let Some(endpoint) = self.endpoints.get(index) else {
            return Connection::Offline;
        };
        if !endpoint.connection.target.is_remote() {
            return Connection::Online;
        }
        let live = if index == self.selected_endpoint {
            &self.live
        } else {
            &endpoint.live
        };
        if live.status.is_connected() {
            Connection::Online
        } else {
            Connection::Offline
        }
    }

    /// Makes `id`'s view from its tab, if the window has none yet.
    fn ensure_git_graph(&mut self, id: TabId, cx: &mut Context<Self>) {
        if self.git_graphs.contains_key(&id) {
            return;
        }
        let Some(Location::GitGraph { repo }) = cx
            .try_global::<Store>()
            .and_then(|store| store.get(id))
            .and_then(|tab| tab.location.clone())
        else {
            return;
        };
        let endpoint = &self.endpoints[self.selected_endpoint];
        let host = Host::new(&endpoint.connection.target).map_err(super::Error::from);
        let remote = endpoint
            .connection
            .target
            .is_remote()
            .then(|| endpoint.label.clone());
        let endpoint = endpoint.id.clone();
        let connection = self.graph_connection(self.selected_endpoint);
        let look = self.graph_look();
        let view = cx.new(|cx| GitGraphView::new(host, repo, look, connection, cx).remote(remote));
        self.git_graphs.insert(id, GitGraph { view, endpoint });
    }

    /// Runs every window tick: views for restored tabs are made, closed
    /// tabs' views go (dropping their workers, which cancels their scripts),
    /// and every view gets its worker's news and the window's current look.
    pub(crate) fn poll_git_graphs(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(store) = cx.try_global::<Store>() else {
            self.git_graphs.clear();
            return;
        };
        self.git_graphs.retain(|id, _| store.get(*id).is_some());
        if let Some((scope, workspace)) = self.browser_key() {
            let missing: Vec<TabId> = store
                .in_workspace(&scope, &workspace)
                .filter(|tab| {
                    matches!(tab.location, Some(Location::GitGraph { .. }))
                        && !self.git_graphs.contains_key(&tab.id)
                })
                .map(|tab| tab.id)
                .collect();
            for id in missing {
                self.ensure_git_graph(id, cx);
            }
        }
        if self.git_graphs.is_empty() {
            return;
        }
        let look = self.graph_look();
        for graph in self.git_graphs.values() {
            let index = self.endpoints.iter().position(|e| e.id == graph.endpoint);
            let connection =
                index.map_or(Connection::Offline, |index| self.graph_connection(index));
            graph.view.update(cx, |view, cx| {
                view.set_connection(connection, cx);
                view.poll(cx);
                view.set_look(look.clone(), cx);
            });
        }
    }

    /// A Git graph tab, drawn in `slot` where a page would be.
    pub(crate) fn render_git_graph_tab(&self, slot: Slot, tab: &Tab) -> AnyElement {
        let body = match self.git_graphs.get(&tab.id) {
            Some(graph) => graph.view.clone().into_any_element(),
            None => div().into_any_element(),
        };
        div()
            .id(SharedString::from(slot.selector("git-graph-tab")))
            .size_full()
            .min_w_0()
            .child(body)
            .into_any_element()
    }
}
