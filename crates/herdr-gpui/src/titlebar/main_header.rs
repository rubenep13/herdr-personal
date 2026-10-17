//! The content column's header: Back and Forward, the focused tab with its
//! agent's mark and where it lives, the daemon's status text, git, the
//! account, and the window's controls. It moves the window as a title bar
//! does, and keeps the sidebar's toggle while the sidebar cannot.

use super::{LEADING, NAVIGATION_MIN_WIDTH, movable, status};
use crate::{
    HerdrWindow,
    design::{HEADER_HEIGHT, TypeScale},
    icons::AgentIcon,
};
use gpui::{prelude::*, *};

/// The narrowest header that still names the focused tab; below it the
/// header keeps its controls and drops the name.
const TITLE_MIN_WIDTH: f32 = 440.;
const TILE: f32 = 24.;
const TILE_RADIUS: f32 = 7.;
const MARK: f32 = 15.;

/// What the header names: the focused tab and where it lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HeaderPlace {
    pub(crate) title: String,
    pub(crate) host: String,
    pub(crate) space: String,
    /// The agent the tab runs; a terminal has none.
    pub(crate) agent: Option<AgentIcon>,
}

impl HerdrWindow {
    /// The selected host's focused tab, in its focused space.
    pub(crate) fn header_place(&self) -> Option<HeaderPlace> {
        let snapshot = self.live.snapshot.as_deref()?;
        let space_id = snapshot.focused_workspace_id.as_deref()?;
        let space = snapshot
            .workspaces
            .iter()
            .find(|space| space.workspace_id == space_id)?;
        let in_space = |id: &str| {
            snapshot
                .tabs
                .iter()
                .find(|tab| tab.tab_id == id && tab.workspace_id == space_id)
        };
        let tab = snapshot
            .focused_tab_id
            .as_deref()
            .and_then(in_space)
            .or_else(|| in_space(&space.active_tab_id))?;
        let agent = crate::window::tab_agent(snapshot, &tab.tab_id)
            .map(|agent| AgentIcon::from_identity(agent.agent.as_deref()));
        Some(HeaderPlace {
            title: tab.label.clone(),
            host: self
                .endpoints
                .get(self.selected_endpoint)
                .map_or_else(String::new, |endpoint| endpoint.label.clone()),
            space: space.label.clone(),
            agent,
        })
    }

    /// The tile, the focused tab's name, and `host › space` after it.
    fn header_title(&self) -> Option<Div> {
        let place = self.header_place()?;
        let theme = &self.theme;
        let scale = TypeScale::of(&self.config.ui);
        let (mark, color) = match place.agent {
            Some(icon) => (
                icon.path(),
                theme.ink(icon.brand().unwrap_or(theme.foreground)),
            ),
            None => ("icons/terminal.svg", theme.muted),
        };
        Some(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .ml(px(6.))
                .min_w_0()
                .flex_shrink_1()
                .child(
                    div()
                        .debug_selector(|| "main-header-tile".into())
                        .size(px(TILE))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(TILE_RADIUS))
                        .bg(rgb(theme.active))
                        .child(svg().path(mark).size(px(MARK)).text_color(rgb(color))),
                )
                .child(
                    div()
                        .debug_selector(|| "main-header-title".into())
                        .min_w_0()
                        .flex_shrink_1()
                        .truncate()
                        .text_size(px(scale.name + 1.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(place.title),
                )
                .child(
                    div()
                        .debug_selector(|| "main-header-crumbs".into())
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .min_w_0()
                        // The path gives way before the name does.
                        .flex_shrink(100.)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(scale.secondary + 0.5))
                        .text_color(rgb(theme.muted))
                        .child(div().flex_none().child(place.host))
                        .child(
                            div()
                                .flex_none()
                                .text_color(rgb(theme.active))
                                .child("\u{203a}"),
                        )
                        .child(div().min_w_0().truncate().child(place.space)),
                ),
        )
    }

    /// The header atop the content column.
    pub(crate) fn main_header(&self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let viewport = f32::from(window.viewport_size().width);
        let column = self
            .sidebar_mode()
            .width(self.sidebar_width, viewport)
            .unwrap_or(0.);
        // What the header has to share, beside the sidebar.
        let room = viewport - column;
        // The traffic lights sit over whatever starts the window; a sidebar
        // as wide as their room keeps them off this header.
        let clearance = (LEADING - column).max(8.);
        let header = div()
            .id("titlebar")
            .debug_selector(|| "titlebar".into())
            .flex()
            .flex_none()
            .items_center()
            .w_full()
            .h(px(HEADER_HEIGHT))
            .gap(px(4.))
            .bg(self.fill().panel(self.theme.background))
            .border_b_1()
            .border_color(rgb(self.theme.active))
            .child(div().flex_none().w(px(clearance)).h_full())
            // The sidebar's own header has these while it shows them.
            .when(!self.sidebar_header_has_toggle(window), |header| {
                header.child(self.sidebar_toggle(cx))
            })
            .when(
                cfg!(target_os = "linux") && !self.sidebar_tools_shown(),
                |header| header.child(self.application_menu_button(cx)),
            )
            .when(room >= NAVIGATION_MIN_WIDTH, |header| {
                header.child(self.navigation(cx))
            })
            .when(room >= TITLE_MIN_WIDTH, |header| {
                header.children(self.header_title())
            })
            .child(
                div()
                    .debug_selector(|| "titlebar-center".into())
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .when(self.config.usage.topbar, |center| {
                        center.child(status::render(&self.live, &self.config.ui, &self.theme))
                    }),
            )
            .child(self.titlebar_end(window, cx));
        movable(header, window)
    }
}
