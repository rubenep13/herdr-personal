//! The inbox's filter row when it runs out of room. The search field keeps
//! `FIELD_MIN`; first the order keeps only its icon, then the toggles fold
//! into one Filters button whose menu holds them.

use super::{OrchestratorView, rows::Tab};
use crate::{design::Tone, orchestrator::Provider};
use gpui::{prelude::*, *};

/// The narrowest the search field gets, at `[ui]` 12.
pub(super) const FIELD_MIN: f32 = 180.;
/// The Filters menu's narrowest, at `[ui]` 12.
const MENU_WIDTH: f32 = 180.;
/// How far the Filters menu hangs below its button, at `[ui]` 12.
const MENU_GAP: f32 = 4.;
/// How many label widths a view keeps: its tabs' toggles and the orders.
const WIDTHS_KEPT: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FilterFit {
    /// The toggles and the order's name.
    Full,
    /// The toggles and the order's icon.
    IconSort,
    /// One Filters button for the toggles, and the order's icon.
    Folded,
}

/// The room a filter row needs for [`FilterFit::Full`] and for
/// [`FilterFit::IconSort`]: a search field `field_min` wide, `toggles`, and
/// an order `sort` wide or `sort_icon` wide as an icon, `gap` apart.
fn needs(field_min: f32, toggles: &[f32], sort: f32, sort_icon: f32, gap: f32) -> [f32; 2] {
    let fixed = field_min + toggles.iter().sum::<f32>() + gap * (toggles.len() + 1) as f32;
    [fixed + sort, fixed + sort_icon]
}

/// How a filter row `room` wide holds its controls; see [`needs`].
pub(super) fn filter_fit(
    room: f32,
    field_min: f32,
    toggles: &[f32],
    sort: f32,
    sort_icon: f32,
    gap: f32,
) -> FilterFit {
    let [full, icon_sort] = needs(field_min, toggles, sort, sort_icon, gap);
    if full <= room {
        FilterFit::Full
    } else if icon_sort <= room {
        FilterFit::IconSort
    } else {
        FilterFit::Folded
    }
}

/// A filter the current tab offers: a toggle in the row, or a row of the
/// Filters menu.
struct Toggle {
    id: &'static str,
    label: &'static str,
    on: bool,
    flip: fn(&mut OrchestratorView),
}

impl OrchestratorView {
    /// The current tab's filters, in the row's order.
    fn filter_toggles(&self) -> Vec<Toggle> {
        let filters = self.filters;
        match self.tab {
            Tab::Issues => vec![
                Toggle {
                    id: "orchestrator-beads",
                    label: "Beads",
                    on: filters.provider == Some(Provider::Beads),
                    flip: |this| {
                        this.filters.provider = match this.filters.provider {
                            Some(Provider::Beads) => None,
                            _ => Some(Provider::Beads),
                        };
                    },
                },
                Toggle {
                    id: "orchestrator-has-run",
                    label: "Has agent",
                    on: filters.has_run,
                    flip: |this| this.filters.has_run = !this.filters.has_run,
                },
                Toggle {
                    id: "orchestrator-blocked",
                    label: "Blocked",
                    on: filters.blocked,
                    flip: |this| this.filters.blocked = !this.filters.blocked,
                },
            ],
            Tab::PullRequests => {
                let mut toggles = vec![Toggle {
                    id: "orchestrator-open",
                    label: "Open",
                    on: filters.open_only,
                    flip: |this| this.filters.open_only = !this.filters.open_only,
                }];
                if self.login.is_some() {
                    toggles.push(Toggle {
                        id: "orchestrator-mine",
                        label: "Mine",
                        on: self.mine,
                        flip: |this| this.mine = !this.mine,
                    });
                }
                toggles.push(Toggle {
                    id: "orchestrator-pr-has-run",
                    label: "Has agent",
                    on: filters.has_run,
                    flip: |this| this.filters.has_run = !this.filters.has_run,
                });
                toggles
            }
            Tab::Runs => vec![Toggle {
                id: "orchestrator-active",
                label: "Active",
                on: self.active_runs,
                flip: |this| this.active_runs = !this.active_runs,
            }],
        }
    }

    /// The shaped width of a control's `label`, cached until the look
    /// changes.
    fn label_width(&mut self, label: &'static str, window: &Window) -> f32 {
        if let Some(width) = self.label_widths.get(label) {
            return *width;
        }
        if self.label_widths.len() >= WIDTHS_KEPT {
            self.label_widths.clear();
        }
        let run = TextRun {
            len: label.len(),
            font: self.look.ui.font(),
            color: black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let width = f32::from(
            window
                .text_system()
                .shape_line(label.into(), self.look.view().control_text(), &[run], None)
                .width,
        )
        .ceil();
        self.label_widths.insert(label, width);
        width
    }

    /// How the filter row fits the inbox's width at its last layout, and
    /// the inbox widths at which that changes, for the inbox's probe.
    pub(super) fn filter_layout(&mut self, window: &Window) -> (FilterFit, [f32; 2]) {
        let view = self.look.view();
        let labels: Vec<_> = self
            .filter_toggles()
            .iter()
            .map(|toggle| (toggle.label, toggle.on))
            .collect();
        let toggles: Vec<f32> = labels
            .into_iter()
            .map(|(label, on)| view.pill_width(self.label_width(label, window), on))
            .collect();
        let (sort, sort_icon) = if self.tab == Tab::Runs {
            (0., 0.)
        } else {
            let label = self.label_width(self.sort.label(), window);
            (view.dropdown_width(label), view.dropdown_width(0.))
        };
        let field = f32::from(view.at(FIELD_MIN));
        let gap = view.control_gap();
        let inset = view.filters_padding() * 2.;
        let width = self.width.get();
        let fit = if width > 0. {
            filter_fit(width - inset, field, &toggles, sort, sort_icon, gap)
        } else {
            FilterFit::Full
        };
        let cuts = needs(field, &toggles, sort, sort_icon, gap).map(|need| need + inset);
        (fit, cuts)
    }

    /// The current tab's filters as toggles in the row, or folded into the
    /// Filters button.
    pub(super) fn render_filter_toggles(
        &self,
        fit: FilterFit,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let look = &self.look;
        let toggles = self.filter_toggles();
        if fit == FilterFit::Folded {
            return vec![self.render_filters_button(toggles, cx).into_any_element()];
        }
        toggles
            .into_iter()
            .map(|toggle| {
                let flip = toggle.flip;
                look.toggle(toggle.id, toggle.label, toggle.on)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        flip(this);
                        this.refresh_rows();
                        cx.notify();
                    }))
                    .into_any_element()
            })
            .collect()
    }

    /// One button for the folded filters, counting the ones on; its menu
    /// holds a row for each.
    fn render_filters_button(&self, toggles: Vec<Toggle>, cx: &mut Context<Self>) -> Div {
        let view = self.look.view();
        let on = toggles.iter().filter(|toggle| toggle.on).count();
        let button = view
            .pill(
                "orchestrator-filters-button",
                "orchestrator-filters-button".to_owned(),
                "Filters",
                false,
            )
            .when(on > 0, |button| {
                button.child(view.settings().chip(
                    "orchestrator-filters-count".to_owned(),
                    on.to_string(),
                    Tone::Neutral,
                ))
            })
            .on_click(cx.listener(|this, _, _, cx| {
                this.filters_open = !this.filters_open;
                this.sort_open = false;
                cx.notify();
            }));
        div()
            .relative()
            .flex_none()
            .child(button)
            .when(self.filters_open, |anchor| {
                anchor.child(self.render_filters_menu(toggles, cx))
            })
    }

    /// The folded filters' menu, hanging from its button: a row for each,
    /// checked while on. A row flips its filter and keeps the menu open.
    fn render_filters_menu(&self, toggles: Vec<Toggle>, cx: &mut Context<Self>) -> Div {
        let view = self.look.view();
        div()
            .absolute()
            .top_full()
            .left_0()
            .mt(view.at(MENU_GAP))
            .child(
                deferred(
                    anchored().snap_to_window_with_margin(px(8.)).child(
                        view.menu_panel("orchestrator-filters-menu")
                            .min_w(view.at(MENU_WIDTH))
                            .occlude()
                            .children(toggles.into_iter().enumerate().map(|(n, toggle)| {
                                let flip = toggle.flip;
                                view.choice(
                                    ("orchestrator-filters-menu", n),
                                    format!("orchestrator-filters-menu-{n}"),
                                    toggle.label,
                                    false,
                                    toggle.on,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        flip(this);
                                        this.refresh_rows();
                                        cx.notify();
                                    },
                                ))
                            })),
                    ),
                )
                .with_priority(1),
            )
    }
}
