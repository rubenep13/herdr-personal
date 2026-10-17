//! How a tab in a strip looks: plain text on the strip, the chosen one in
//! the text colour over a short line in the accent (under the line when the
//! strip sits at the foot), and what a Herdr tab leads with, as
//! `[tabs] content` asks: nothing, its agent's mark, or the mark and its
//! state.

use super::{HerdrWindow, tab_strip::Placement};
use crate::{
    browser::{GroupId, Pick, Slot},
    config::{TabContent, mix},
    icons::AgentIcon,
};
use gpui::{prelude::*, *};
use herdr_client::protocol::{AgentStatus, ClientShellAgent, ClientShellSnapshot, ClientShellTab};

/// How far the chosen tab's line keeps from the tab's edges, and its weight.
const UNDERLINE_INSET: f32 = 8.;
const UNDERLINE: f32 = 2.;
/// A tab's agent mark on its tile.
const TILE: f32 = 18.;
const TILE_RADIUS: f32 = 5.;
const MARK: f32 = 11.;
const BADGE: f32 = 12.;
/// Space after a tab's lead, before its name.
pub(super) const LEAD_GAP: f32 = 7.;
/// The weight the chosen tab's name draws in.
pub(super) const CHOSEN_WEIGHT: FontWeight = FontWeight::MEDIUM;

/// What a tab shows before its name, which its width makes room for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Lead {
    Nothing,
    /// Its agent's mark, or a terminal.
    Mark,
    /// The mark after its agent's state.
    MarkAndState,
    /// A browser tab's globe or page icon.
    Globe,
}

impl Lead {
    /// The room it takes before the name, its gap included.
    pub(super) fn width(self) -> f32 {
        match self {
            Self::Nothing => 0.,
            Self::Mark => TILE + LEAD_GAP,
            Self::MarkAndState => BADGE + LEAD_GAP + TILE + LEAD_GAP,
            Self::Globe => 12. + 6.,
        }
    }
}

/// The agent a tab runs, if any: the one in its focused pane, else its first.
pub(crate) fn tab_agent<'a>(
    snapshot: &'a ClientShellSnapshot,
    tab: &str,
) -> Option<&'a ClientShellAgent> {
    let mut agents = snapshot.agents.iter().filter(|agent| agent.tab_id == tab);
    let first = agents.next()?;
    let focused = snapshot.focused_pane_id.as_deref();
    Some(
        std::iter::once(first)
            .chain(agents)
            .find(|agent| Some(agent.pane_id.as_str()) == focused)
            .unwrap_or(first),
    )
}

impl HerdrWindow {
    /// Where the tab strips sit: `[tabs] position`, or Herdr's own
    /// `tab_bar_position` when it says to follow Herdr.
    pub(crate) fn tab_placement(&self) -> Placement {
        use crate::{config::TabPlacement, herdr_settings::TabBarPosition};
        match self.config.tabs_style.placement {
            TabPlacement::Subheader => Placement::Subheader,
            TabPlacement::Bottom => Placement::Bottom,
            TabPlacement::Herdr => match self.tab_bar_position() {
                TabBarPosition::Top => Placement::Subheader,
                TabBarPosition::Bottom => Placement::Bottom,
            },
        }
    }

    /// How far below the content column's top what it shows starts: under
    /// the main header and, as the subheader, the tab strip. Cards and toasts
    /// floating over the content start below it, clear of the tabs.
    pub(crate) fn chrome_top(&self) -> f32 {
        let header = if self.zen_layout() {
            0.
        } else {
            crate::design::HEADER_HEIGHT
        };
        let strip = match self.tab_placement() {
            Placement::Subheader => self.tab_strip_height(),
            Placement::Bottom => 0.,
        };
        header + strip
    }

    /// The line under the chosen tab: the accent in the group in use and a
    /// quiet one elsewhere, so a split shows which group has the keyboard.
    pub(crate) fn tab_underline(&self, group: GroupId) -> u32 {
        if !self.is_split() || self.active_group() == Some(group) {
            self.theme.primary()
        } else {
            self.theme.muted
        }
    }

    /// Dresses `tab` as a strip's tab: its text, its hover, and the chosen
    /// one's line. `key` names the line for probes.
    pub(crate) fn tab_shell(
        &self,
        tab: Stateful<Div>,
        slot: Slot,
        key: &str,
        selected: bool,
    ) -> Stateful<Div> {
        let theme = &self.theme;
        let hover = theme.subtext();
        let line = selected.then(|| {
            let selector = slot.selector(&format!("tab-underline-{key}"));
            div()
                .debug_selector(move || selector)
                .absolute()
                .left(px(UNDERLINE_INSET))
                .right(px(UNDERLINE_INSET))
                .h(px(UNDERLINE))
                .rounded(px(UNDERLINE))
                .bg(rgb(self.tab_underline(slot.id)))
                // Over the strip's edge line, so the chosen tab reads as
                // opening onto what it shows.
                .map(|line| match self.tab_placement() {
                    Placement::Subheader => line.bottom(px(-1.)),
                    Placement::Bottom => line.top(px(-1.)),
                })
        });
        tab.relative()
            .h_full()
            .flex_none()
            .flex()
            .items_center()
            .cursor_pointer()
            .map(|tab| {
                if selected {
                    tab.text_color(rgb(theme.foreground))
                        .font_weight(CHOSEN_WEIGHT)
                } else {
                    tab.text_color(rgb(theme.muted))
                        .hover(move |style| style.text_color(rgb(hover)))
                }
            })
            .children(line)
    }

    /// What a Herdr tab leads with under `[tabs] content`, and its width.
    pub(super) fn tab_lead_kind(&self, tab: &ClientShellTab) -> Lead {
        match self.config.tabs_style.content {
            TabContent::Name => Lead::Nothing,
            TabContent::Agent => Lead::Mark,
            TabContent::Status => {
                let agent = self
                    .live
                    .snapshot
                    .as_deref()
                    .and_then(|snapshot| tab_agent(snapshot, &tab.tab_id));
                // A terminal reports no state, so it never shows one.
                if agent.is_some() {
                    Lead::MarkAndState
                } else {
                    Lead::Mark
                }
            }
        }
    }

    /// A Herdr tab's lead: its state badge, then its agent's mark or a
    /// terminal on a small tile.
    pub(super) fn tab_lead(&self, slot: Slot, tab: &ClientShellTab) -> Option<Div> {
        let lead = self.tab_lead_kind(tab);
        if lead == Lead::Nothing {
            return None;
        }
        let theme = &self.theme;
        let snapshot = self.live.snapshot.as_deref()?;
        let agent = tab_agent(snapshot, &tab.tab_id);
        let (mark, color) = match agent {
            Some(agent) => {
                let icon = AgentIcon::from_identity(agent.agent.as_deref());
                (
                    icon.path(),
                    theme.ink(icon.brand().unwrap_or(theme.foreground)),
                )
            }
            None => ("icons/terminal.svg", theme.muted),
        };
        let id = &tab.tab_id;
        let logo = {
            let selector = slot.selector(&format!("tab-logo-{id}"));
            div()
                .debug_selector(move || selector)
                .size(px(TILE))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(TILE_RADIUS))
                .bg(rgb(mix(theme.background, theme.active, 70)))
                .child(svg().path(mark).size(px(MARK)).text_color(rgb(color)))
        };
        let state = agent.filter(|_| lead == Lead::MarkAndState).map(|_| {
            let selector = slot.selector(&format!("tab-status-{id}"));
            let status = tab.agent_status;
            div()
                .debug_selector(move || selector)
                .size(px(BADGE))
                .flex_none()
                .child(
                    svg()
                        .path(badge_path(status))
                        .size(px(BADGE))
                        .text_color(rgb(self.status_color(status))),
                )
        });
        Some(
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(LEAD_GAP))
                .children(state)
                .child(logo),
        )
    }

    /// A Herdr tab in `slot`'s strip: its lead, its name, its zoom mark, and
    /// a close button that shows on the chosen tab or under the pointer.
    pub(super) fn herdr_tab(
        &self,
        slot: Slot,
        tab: &ClientShellTab,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let id = tab.tab_id.clone();
        let (context_id, close_id, click_id) = (id.clone(), id.clone(), id.clone());
        let group = SharedString::from(slot.selector(&format!("tab-{id}")));
        let (muted, foreground, active) =
            (self.theme.muted, self.theme.foreground, self.theme.active);
        let selector = slot.selector(&format!("tab-{id}"));
        let tab_element = div()
            .id(SharedString::from(format!("tab-{id}")))
            .debug_selector(move || selector)
            .group(group.clone())
            .pl(px(12.))
            .pr(px(3.))
            .min_w(px(crate::TAB_WIDTH))
            .map(|element| self.grow_tab(element, slot.id, &Pick::Herdr(id.clone())))
            .gap(px(super::tab_strip::CLOSE_GAP))
            .child(
                div()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .gap(px(LEAD_GAP))
                    .children(self.tab_lead(slot, tab))
                    .child(div().min_w_0().truncate().child(tab.label.clone()))
                    // Herdr's " Z": one pane fills the tab, the rest are
                    // hidden behind it.
                    .when(tab.zoomed, |title| {
                        let selector = slot.selector(&format!("tab-zoom-{id}"));
                        title.child(
                            svg()
                                .path("icons/zoom.svg")
                                .debug_selector(move || selector)
                                .flex_none()
                                .size(px(super::tab_strip::ZOOM_ICON))
                                .text_color(rgb(muted)),
                        )
                    }),
            )
            .child({
                let selector = slot.selector(&format!("close-tab-{id}"));
                let icon = slot.selector(&format!("close-tab-icon-{id}"));
                div()
                    .id("close-tab")
                    .debug_selector(move || selector)
                    .size(px(18.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(crate::config::corners::CONTROL))
                    .hover(move |style| style.bg(rgb(active)))
                    // Out of sight until the tab is chosen or pointed at, yet
                    // holding its room so the strip never shifts.
                    .when(!selected, |close| {
                        close
                            .map(|mut close| {
                                close.style().visibility = Some(Visibility::Hidden);
                                close
                            })
                            .group_hover(group, |mut style| {
                                style.visibility = Some(Visibility::Visible);
                                style
                            })
                    })
                    .child(
                        svg()
                            .path("icons/close.svg")
                            .debug_selector(move || icon)
                            .size(px(12.))
                            .text_color(rgb(if selected { foreground } else { muted })),
                    )
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.close_strip_tab(Pick::Herdr(close_id.clone()), window, cx);
                    }))
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_tab_menu(&context_id, Some(slot.id), event.position, window, cx);
                    this.menu.opening_right_click = this.menu.page == Some(crate::menu::Page::Tab);
                }),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.choose_herdr_tab(slot.id, &click_id, window, cx);
            }));
        self.tab_shell(tab_element, slot, &tab.tab_id, selected)
    }

    /// A tab's state colour, as the sidebar's badges draw it.
    fn status_color(&self, status: AgentStatus) -> u32 {
        let indicators = crate::sidebar::Indicators::new(
            self.settings.shared.as_ref(),
            self.config.status_icons,
            self.theme_light,
            &self.theme,
        );
        self.theme.ink(indicators.color(status))
    }
}

/// The badge for `status`, as the Tree's agent rows draw it.
fn badge_path(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Blocked => "icons/status-badge-blocked.svg",
        status => crate::icons::status_icon(crate::config::StatusIcons::Rail, status),
    }
}
