//! Drawing an overview tab: its header and search, the activity chart, the
//! totals, and the device table, all from state the tick prepared.

use super::{Device, Link, Tally, chart, table};
use crate::{
    HerdrWindow,
    browser::{Slot, Tab},
    config::Theme,
    design::{ButtonKind, ViewLook},
    fonts::StyledFont,
    sidebar::Indicators,
};
use gpui::{prelude::*, *};
use herdr_client::protocol::AgentStatus;

/// What every part of the page, and the device picker's activity card,
/// draws with.
pub(crate) struct Look<'a> {
    pub theme: &'a Theme,
    pub ui: &'a crate::config::FontConfig,
    pub mono: &'a crate::config::FontConfig,
    pub indicators: Indicators,
    /// The redesigned views' pieces the page builds from.
    pub view: ViewLook,
}

/// The page's margins and the room between its blocks, at `[ui] size = 12`.
const PAGE_PADDING_X: f32 = 22.;
const PAGE_PADDING_Y: f32 = 18.;
const BLOCK_GAP: f32 = 14.;
/// The search field's width when the bar has room for it, and its narrowest:
/// the bar's context gives way first.
const SEARCH_WIDTH: f32 = 240.;
const SEARCH_MIN: f32 = 140.;
/// The totals' narrowest in one row of four; narrower, they go two by two.
const CARD_MIN: f32 = 150.;
const TOTAL_PADDING: f32 = 14.;
const TOTAL_GAP: f32 = 4.;
/// A total's figure, this much over the `[ui]` size.
const FIGURE_OVER: f32 = 10.;

impl Look<'_> {
    pub fn small(&self) -> Pixels {
        px(self.ui.size - 1.)
    }

    pub fn status(&self, status: AgentStatus) -> u32 {
        self.indicators.color(status)
    }

    /// Numbers, sizes and addresses, in the terminal face.
    pub fn mono(&self, text: impl Into<SharedString>) -> Div {
        div()
            .text_font(self.mono)
            .text_size(px(self.mono.size - 1.))
            .text_color(rgb(self.theme.muted))
            .whitespace_nowrap()
            .child(text.into())
    }

    /// A status dot as the sidebar draws it: idle hollow, the rest filled.
    pub fn dot(&self, status: AgentStatus) -> Div {
        self.view
            .status_dot(self.status(status), status == AgentStatus::Idle)
    }

    pub fn link(&self, link: Link) -> Div {
        self.view.status_dot(
            match link {
                Link::Online => crate::menu::online(self.theme),
                Link::Connecting => self.status(AgentStatus::Working),
                Link::Offline | Link::Disabled => self.theme.muted,
            },
            false,
        )
    }

    pub fn legend(&self) -> Div {
        let item = |status: AgentStatus, label: &'static str| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(self.dot(status))
                .child(label)
        };
        div()
            .flex()
            .flex_wrap()
            .gap_3()
            .text_size(self.small())
            .text_color(rgb(self.theme.subtext()))
            .child(item(AgentStatus::Working, "Working"))
            .child(item(AgentStatus::Blocked, "Blocked"))
            .child(item(AgentStatus::Done, "Done"))
            .child(item(AgentStatus::Idle, "Idle"))
    }
}

impl HerdrWindow {
    /// The theme, faces, and the sidebar's own status colours.
    pub(crate) fn overview_look(&self, cx: &App) -> Look<'_> {
        Look {
            theme: &self.theme,
            ui: &self.config.ui,
            mono: &self.config.terminal,
            indicators: Indicators::new(
                self.settings.shared.as_ref(),
                self.config.status_icons,
                matches!(
                    cx.window_appearance(),
                    WindowAppearance::Light | WindowAppearance::VibrantLight
                ),
                &self.theme,
            ),
            view: self.view_look(),
        }
    }

    /// An overview tab, drawn in `slot` where a page would be.
    pub(crate) fn render_devices_tab(
        &self,
        slot: Slot,
        tab: &Tab,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = tab.id;
        let Some(page) = self.devices_overview.pages.get(&id) else {
            return div()
                .id(SharedString::from(slot.selector("devices-loading")))
                .size_full()
                .into_any_element();
        };
        let look = self.overview_look(cx);
        let devices = self.overview_devices();
        let query = page.search.read(cx).text().to_owned();
        let shown: Vec<&Device> = devices
            .iter()
            .filter(|device| device.matches(&query))
            .collect();
        let focus = page.focus.clone();
        let search_focus = page.search.read(cx).focus.clone();
        // The page's own width, not the window's: a split group is narrower.
        let width = page.width.get();
        let padding = f32::from(look.view.at(PAGE_PADDING_X)) * 2.;
        let row_of_four = f32::from(look.view.at(CARD_MIN)) * 4.
            + f32::from(look.view.at(BLOCK_GAP)) * 3.
            + padding;
        let card = (width > 0.).then_some(width - padding);
        let [beside, middle_beside, middle_under] = chart::Shape::cuts(look.view);
        div()
            .id(SharedString::from(slot.selector("devices-tab")))
            .debug_selector(move || slot.selector("devices-tab"))
            .relative()
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .track_focus(&focus)
            .bg(look.view.background())
            .text_color(rgb(look.theme.foreground))
            .text_font(look.ui)
            .text_size(px(look.ui.size))
            // The page holds the keyboard while used, so typing never reaches
            // a terminal in another group.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, _, window, cx| {
                    if !search_focus.is_focused(window) {
                        window.focus(&focus, cx);
                    }
                }),
            )
            .child(page.width.probe([
                row_of_four,
                beside + padding,
                middle_beside + padding,
                middle_under + padding,
            ]))
            .child(self.overview_bar(
                &look,
                &devices,
                page,
                search_focus_now(page, window, cx),
                cx,
            ))
            .child(
                div()
                    .id(SharedString::from(slot.selector("devices-scroll")))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&page.scroll)
                    .child(
                        div()
                            .px(look.view.at(PAGE_PADDING_X))
                            .py(look.view.at(PAGE_PADDING_Y))
                            .flex()
                            .flex_col()
                            .gap(look.view.at(BLOCK_GAP))
                            .child(totals(&look, &devices, width <= 0. || width >= row_of_four))
                            .child(chart::activity(
                                &look,
                                &self.devices_overview.history,
                                &shown,
                                Tally::sum(&devices),
                                chart::Shape::of(look.view, card),
                                page.lanes,
                                slot,
                                id,
                                cx,
                            ))
                            .child(table::devices(
                                &look,
                                &shown,
                                &self.system_load,
                                slot,
                                id,
                                &page.expanded,
                                cx,
                            )),
                    ),
            )
            .into_any_element()
    }

    /// The bar: the page's title and what it counts, its search, and Add
    /// Device, faded with the reason in its tooltip when no device can be
    /// added.
    fn overview_bar(
        &self,
        look: &Look,
        devices: &[Device],
        page: &super::Page,
        search_focused: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let view = look.view;
        let agents = Tally::sum(devices).total();
        let unavailable = self.device_setup_unavailable();
        let noun =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        view.bar("devices-bar")
            .child(
                view.bar_title(
                    Some("icons/devices.svg"),
                    "Devices overview",
                    Some(
                        format!(
                            "{} \u{b7} {}",
                            noun(devices.len(), "device", "devices"),
                            noun(agents, "agent", "agents")
                        )
                        .into(),
                    ),
                ),
            )
            .child(div().flex_1())
            // The field gives way before the button.
            .child(
                view.field(
                    "devices-search".into(),
                    search_focused,
                    Some("icons/search.svg"),
                )
                .w(view.at(SEARCH_WIDTH))
                .min_w(view.at(SEARCH_MIN))
                .flex_shrink(1.)
                .child(div().flex_1().min_w_0().child(page.search.clone())),
            )
            .child(
                view.button(
                    "devices-add",
                    "devices-add".into(),
                    None,
                    "Add Device",
                    ButtonKind::Primary,
                    unavailable.is_none(),
                )
                .when_some(unavailable, |button, reason| {
                    button.tooltip(self.menu_look().tooltip(reason))
                })
                .when(unavailable.is_none(), |button| {
                    button.on_click(cx.listener(|this, _, window, cx| {
                        this.open_device_setup(window, cx);
                    }))
                }),
            )
    }
}

/// Whether the page's search has the keyboard, for its field's ring.
fn search_focus_now(page: &super::Page, window: &Window, cx: &App) -> bool {
    page.search.read(cx).focus.is_focused(window)
}

/// Working, blocked, and idle agents on every device, and how many devices
/// are online: in one row, or two by two when `in_row` is false.
fn totals(look: &Look, devices: &[Device], in_row: bool) -> Div {
    let theme = look.theme;
    let tally = Tally::sum(devices);
    let online = devices
        .iter()
        .filter(|device| device.link == Link::Online)
        .count();
    let unreachable = devices
        .iter()
        .filter(|device| matches!(device.link, Link::Offline | Link::Connecting))
        .count();
    let cells: [(&str, String, String, Option<AgentStatus>); 4] = [
        (
            "Working",
            tally.working.to_string(),
            format!("of {} agents", tally.total()),
            Some(AgentStatus::Working),
        ),
        (
            "Blocked",
            tally.blocked.to_string(),
            "waiting for you".into(),
            Some(AgentStatus::Blocked),
        ),
        (
            "Idle or done",
            (tally.idle + tally.done).to_string(),
            "ready for work".into(),
            Some(AgentStatus::Idle),
        ),
        (
            "Devices online",
            format!("{online}/{}", devices.len()),
            if unreachable == 0 {
                "all reachable".into()
            } else {
                format!("{unreachable} not connected")
            },
            None,
        ),
    ];
    let view = look.view;
    let scale = view.scale();
    let mut cards: Vec<Div> = cells
        .into_iter()
        .enumerate()
        .map(|(index, (title, value, detail, status))| {
            view.settings()
                .card(format!("devices-total-{index}"))
                // A card's detail line truncates before the row goes two by
                // two.
                .flex_1()
                .min_w_0()
                .p(view.at(TOTAL_PADDING))
                .gap(view.at(TOTAL_GAP))
                .child(
                    div()
                        .truncate()
                        .text_size(px(scale.secondary))
                        .text_color(rgb(theme.subtext()))
                        .child(title),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(view.at(8.))
                        .when_some(status, |row, status| row.child(look.dot(status)))
                        .child(
                            div()
                                .text_size(px(look.ui.size + FIGURE_OVER))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(view.title())
                                .child(value),
                        ),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(scale.detail))
                        .text_color(view.muted())
                        .child(detail),
                )
        })
        .collect();
    let gap = view.at(BLOCK_GAP);
    let totals = div().debug_selector(|| "devices-totals".into());
    if in_row {
        return totals.flex().gap(gap).children(cards);
    }
    let second = cards.split_off(2);
    let row = |cards: Vec<Div>| div().flex().gap(gap).children(cards);
    totals
        .flex()
        .flex_col()
        .gap(gap)
        .child(row(cards))
        .child(row(second))
}
