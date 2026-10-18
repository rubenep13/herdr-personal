//! The menu behind a group's "…" button: tabs other than a terminal to open
//! in the group (a blank browser tab, the focused checkout's review, the
//! repository's orchestrator, the focused workspace's listening ports, its
//! VS Code), closing the tab it shows, its other tabs, or the group itself,
//! and splitting it. A terminal tab comes from the group's "+". Closing a
//! tab here is the tab's own close; closing the group closes no tab: its
//! tabs go to the group that takes its room.
use crate::{
    HerdrWindow,
    browser::{GroupId, Pick},
    controls::Command,
    design::{ItemState, Leading},
    listening_ports::Link,
    menu::Page,
};
use gpui::{prelude::*, *};

/// The most a port's process name takes in its row. Names run to 32
/// characters, and the address beside it is what tells ports apart.
const PROCESS_WIDTH: f32 = 64.;

/// How large a row's icon draws, and the room a row without one keeps so
/// labels line up.
const ICON_SIZE: f32 = 14.;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    NewBrowserTab,
    /// The review tab of the checkout the Git chip tracks.
    Review,
    /// The focused workspace's repository's issues, pull requests, and runs.
    Orchestrator,
    /// The focused workspace's repository's history, drawn as a graph.
    GitGraph,
    /// The workspace's VS Code tab, opened in this group.
    VsCode,
    /// One of the focused workspace's listening ports and where it opens.
    Port {
        number: u16,
        process: String,
        link: Link,
    },
    /// Closes the tab the group shows, as the tab's own close button does.
    Close,
    /// Closes every tab of the group but the one it shows, one after
    /// another, each as its own close button does.
    CloseOthers,
    /// Closes the group; its tabs go to the group that takes its room.
    CloseGroup,
    Split,
}

impl Action {
    /// The row's text, which for a port is where its page opens.
    fn label(&self) -> SharedString {
        match self {
            Self::NewBrowserTab => "New Browser Tab".into(),
            Self::Review => "Review Changes".into(),
            Self::Orchestrator => "Orchestrator".into(),
            Self::GitGraph => "Git Graph".into(),
            Self::VsCode => "VS Code".into(),
            Self::Port { link, .. } => link.label().into(),
            Self::Close => "Close Tab".into(),
            Self::CloseOthers => "Close Others".into(),
            Self::CloseGroup => "Close Group".into(),
            Self::Split => "Split Right".into(),
        }
    }

    fn icon(&self) -> Option<&'static str> {
        match self {
            Self::NewBrowserTab => Some("icons/globe.svg"),
            Self::Review => Some("icons/diff-unified.svg"),
            Self::Orchestrator => Some("icons/pulse.svg"),
            Self::GitGraph => Some("icons/git-graph.svg"),
            Self::VsCode => Some("icons/vscode.svg"),
            Self::Port { .. } => Some("icons/arrow-right.svg"),
            Self::Split => Some("icons/split.svg"),
            Self::Close | Self::CloseOthers | Self::CloseGroup => Some("icons/x.svg"),
        }
    }

    /// Stable for tests: `Port5173` rather than the port's whole value.
    fn selector(&self) -> String {
        match self {
            Self::Port { number, .. } => format!("group-menu-Port{number}"),
            action => format!("group-menu-{action:?}"),
        }
    }

    /// Which run of rows this one belongs to; a rule separates runs.
    fn section(&self) -> u8 {
        match self {
            Self::NewBrowserTab
            | Self::Review
            | Self::GitGraph
            | Self::Orchestrator
            | Self::VsCode => 0,
            Self::Port { .. } => 1,
            Self::Close | Self::CloseOthers | Self::CloseGroup => 2,
            Self::Split => 3,
        }
    }
}

pub(crate) struct GroupMenu {
    group: GroupId,
    selected: Option<usize>,
    /// The panel's own scroll, so a row the keyboard selects past a long
    /// list of ports scrolls into view.
    scroll: ScrollHandle,
}

/// The panel child that draws `actions[index]`: the "Listening" heading
/// before the first port is a child of its own.
fn child_index(actions: &[Action], index: usize) -> usize {
    let heading = actions
        .iter()
        .take(index + 1)
        .any(|action| matches!(action, Action::Port { .. }));
    index + usize::from(heading)
}

impl HerdrWindow {
    /// The rows worth offering for `group`. Review needs a checkout; ports
    /// come from the last scan, so they follow the status bar's. Closing
    /// belongs to a split: a lone group has nowhere else to keep its tabs.
    fn group_actions(&self, group: GroupId, cx: &App) -> Vec<Action> {
        let mut actions = vec![Action::NewBrowserTab];
        if self.git.tracked().is_some() {
            actions.push(Action::Review);
        }
        if self.can_open_git_graph() {
            actions.push(Action::GitGraph);
        }
        if self.can_open_orchestrator() {
            actions.push(Action::Orchestrator);
        }
        if self.code_offered(cx) {
            actions.push(Action::VsCode);
        }
        if let Some((_, _, listed)) = self.focused_listening_ports() {
            actions.extend(listed.ports.iter().filter_map(|port| {
                Some(Action::Port {
                    number: port.number,
                    process: port.process.clone(),
                    link: port.link(listed.origin)?,
                })
            }));
        }
        if self.is_split() {
            let pick = self.group_pick(group);
            if pick.is_some() {
                actions.push(Action::Close);
            }
            if self
                .group_tabs(group, cx)
                .iter()
                .any(|tab| Some(tab) != pick.as_ref())
            {
                actions.push(Action::CloseOthers);
            }
            actions.push(Action::CloseGroup);
        }
        actions.push(Action::Split);
        actions
    }

    pub(crate) fn open_group_menu(
        &mut self,
        group: GroupId,
        anchor: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.open_menu(window, cx) {
            return;
        }
        self.menu.anchor = anchor;
        self.menu.page = Some(Page::Group);
        self.menu.group = Some(GroupMenu {
            group,
            selected: None,
            scroll: ScrollHandle::new(),
        });
    }

    fn activate_group_menu(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        let Some(group) = self.menu.group.as_ref().map(|menu| menu.group) else {
            return;
        };
        self.dismiss_menu(window, cx);
        self.activate_group(group, window, cx);
        match action {
            Action::NewBrowserTab => self.open_browser_tab_in(group, window, cx),
            Action::Review => self.open_review(window, cx),
            Action::Orchestrator => self.open_orchestrator(window, cx),
            Action::GitGraph => self.open_git_graph(window, cx),
            Action::VsCode => self.open_code(Some(group), window, cx),
            Action::Port { link, .. } => {
                let Some((endpoint, workspace)) = self
                    .focused_listening_ports()
                    .map(|(endpoint, workspace, _)| (endpoint.to_owned(), workspace.to_owned()))
                else {
                    return;
                };
                self.open_port_link(&endpoint, &workspace, &link, window, cx);
            }
            Action::Close => self.close_group_tab(group, window, cx),
            Action::CloseOthers => {
                let keep = self.group_pick(group);
                let others: Vec<Pick> = self
                    .group_tabs(group, cx)
                    .into_iter()
                    .filter(|tab| Some(tab) != keep.as_ref())
                    .collect();
                self.close_tabs(others, window, cx);
            }
            Action::CloseGroup => self.close_group(group, window, cx),
            Action::Split => self.split_group(group, window, cx),
        }
    }

    /// Closes the tab `group` shows, for Close Tab: a page at once, a Herdr
    /// tab through its confirmation.
    pub(crate) fn close_group_tab(
        &mut self,
        group: GroupId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.group_pick(group) {
            Some(Pick::Herdr(tab)) => self.open_tab_close(&tab, window, cx),
            Some(Pick::Page(id)) => self.close_browser_tab(id, window, cx),
            None => {}
        }
    }

    pub(crate) fn group_menu_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.menu.group.as_ref().map(|menu| menu.group) else {
            return;
        };
        let actions = self.group_actions(group, cx);
        let Some(menu) = &mut self.menu.group else {
            return;
        };
        let key = event.keystroke.key.as_str();
        cx.stop_propagation();
        window.prevent_default();
        let count = actions.len();
        match key {
            "escape" => self.dismiss_menu(window, cx),
            "up" | "down" => {
                let selected = match (menu.selected, key) {
                    (None, "up") => count - 1,
                    (None, _) => 0,
                    (Some(i), "up") => (i + count - 1) % count,
                    (Some(i), _) => (i + 1) % count,
                };
                menu.selected = Some(selected);
                menu.scroll.scroll_to_item(child_index(&actions, selected));
                cx.notify();
            }
            "enter" => {
                if let Some(action) = menu.selected.and_then(|index| actions.get(index)).cloned() {
                    self.activate_group_menu(action, window, cx);
                }
            }
            _ => {}
        }
    }

    /// Fills the menu's `panel` with its rows. They are the panel's own
    /// children, which the panel's scroll handle addresses by index.
    pub(crate) fn render_group_menu(
        &self,
        panel: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Some(menu) = &self.menu.group else {
            return panel;
        };
        let theme = &self.theme;
        let look = self.menu_look();
        let mut panel = panel.track_scroll(&menu.scroll).flex().flex_col();
        let mut section = None;
        for (index, action) in self.group_actions(menu.group, cx).into_iter().enumerate() {
            let starts_section = section.is_some_and(|last| last != action.section());
            let first_port = matches!(action, Action::Port { .. }) && section != Some(1);
            section = Some(action.section());
            if first_port {
                // One child, the separator inside it, so the scroll handle's
                // count of children stays what `child_index` expects.
                panel = panel.child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .when(starts_section, |heading| {
                            heading.child(
                                look.separator()
                                    .debug_selector(|| "group-menu-separator-listening".into()),
                            )
                        })
                        .child(look.section("group-menu-section-listening".into(), "Listening")),
                );
            }
            let detail: SharedString = match &action {
                Action::NewBrowserTab => self
                    .keymap()
                    .primary(Command::NewBrowserTab)
                    .to_owned()
                    .into(),
                // It has no default key; one bound to open_code shows.
                Action::VsCode => self.keymap().primary(Command::OpenCode).to_owned().into(),
                Action::Port { process, .. } => process.clone().into(),
                _ => "".into(),
            };
            let selector = action.selector();
            let (label, process) = (format!("{selector}-label"), format!("{selector}-detail"));
            let capped = matches!(action, Action::Port { .. });
            // An open tunnel can be closed from its row, as from its chip.
            let close = match &action {
                Action::Port {
                    number,
                    link: Link::Tunnel(key),
                    ..
                } if self.tunnels.is_open(key) => Some(crate::listening_ports::close_button(
                    format!("group-menu-close-Port{number}").into(),
                    key,
                    ICON_SIZE,
                    theme,
                    cx,
                )),
                _ => None,
            };
            let state = ItemState {
                selected: menu.selected == Some(index),
                ..ItemState::default()
            };
            let leading = action.icon().map_or(Leading::Empty, |icon| {
                Leading::Icon(icon, format!("{selector}-icon"))
            });
            let separated = starts_section && !first_port;
            let row = look
                .row(
                    ("group-menu-action", index),
                    selector.clone(),
                    leading,
                    state,
                )
                .child(look.label(label, action.label()))
                .when(!detail.is_empty(), |line| {
                    line.child(
                        look.detail(process, detail)
                            .when(capped, |detail| detail.max_w(px(PROCESS_WIDTH)).truncate()),
                    )
                })
                .children(close)
                .on_hover(cx.listener(move |this, hovered, _, cx| {
                    if *hovered && let Some(menu) = &mut this.menu.group {
                        menu.selected = Some(index);
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.activate_group_menu(action.clone(), window, cx)
                }));
            // As with the heading, the separator rides inside the row's child.
            panel = panel.child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .when(separated, |run| {
                        run.child(
                            look.separator()
                                .debug_selector(move || format!("group-menu-separator-{index}")),
                        )
                    })
                    .child(row),
            );
        }
        panel
    }
}

#[cfg(test)]
mod tests;
