//! The tab's top bar (the repository, its current branch, whether it is
//! live, reload and search) and the column headers, with their resizable
//! edges and the gear that turns on Author, Date and SHA.

use super::GitGraphView;
use crate::{
    browser::checkout_name,
    design::Tone,
    git_graph::{
        geometry::{MESSAGE_MIN, REFS_DEFAULT},
        prefs::{GraphPrefs, GraphPrefsStore},
    },
    panel_resize::RESIZE_HOVER,
};
use gpui::{prelude::*, *};

/// The top bar's height, at the graph's `[ui]`-sized face of 12.
pub(super) const BAR_H: f32 = 40.;
/// How wide a column's grabbable edge is.
const EDGE: f32 = 6.;
/// The refs column's heading starts past the labels' inset.
const REFS_INSET: f32 = 16.;
/// The gear's button, the menu's least width and the room it keeps from
/// the heading's edge.
const GEAR: f32 = 22.;
/// The room each side of the gear in its own place at the headings' end.
const GEAR_MARGIN: f32 = 4.;
/// The narrowest the label column gets when the other columns need room.
const REFS_MIN: f32 = 80.;
const MENU_WIDTH: f32 = 180.;
const MENU_OFFSET: f32 = 6.;
const CHECKBOX: f32 = 16.;
/// The column headings' height, as every list's.
const COLUMNS_H: f32 = 28.;

/// A column whose right edge is being dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ColumnDrag {
    Refs,
    Graph,
    /// The detail panel's left edge.
    Panel,
}

/// The optional columns, in order, with their key and header.
const OPTIONAL: [(&str, &str); 3] = [("author", "Author"), ("date", "Date"), ("sha", "SHA")];

fn toggle(prefs: &mut GraphPrefs, key: &str) {
    match key {
        "author" => prefs.author = !prefs.author,
        "date" => prefs.date = !prefs.date,
        _ => prefs.sha = !prefs.sha,
    }
}

fn is_on(prefs: &GraphPrefs, key: &str) -> bool {
    match key {
        "author" => prefs.author,
        "date" => prefs.date,
        _ => prefs.sha,
    }
}

impl GitGraphView {
    /// The repository's name and current branch, as the top bar shows them.
    fn repository_and_branch(&self) -> (String, Option<String>) {
        let Some(history) = &self.history else {
            return (self.checkout.name().to_owned(), None);
        };
        let name = if history.root.is_empty() {
            self.checkout.name()
        } else {
            checkout_name(&history.root)
        };
        let branch = history
            .head
            .as_ref()
            .and_then(|head| history.chips.get(head))
            .and_then(|chips| chips.iter().find(|chip| chip.current))
            .map(|chip| chip.label.clone());
        (name.to_owned(), branch)
    }

    pub(super) fn render_bar(&self, searching: bool, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.look.theme;
        let view = self.look.view();
        let (name, branch) = self.repository_and_branch();
        let live = match self.liveness() {
            super::Liveness::Live => theme.ink(theme.palette[2]),
            super::Liveness::Reconnecting => theme.ink(theme.palette[3]),
            super::Liveness::Offline => theme.muted,
        };
        view.bar("graph-bar")
            .child(view.bar_title(Some("icons/git-graph.svg"), name, None))
            .when_some(branch, |bar, branch| {
                bar.child(
                    div()
                        .debug_selector(|| "graph-branch".to_owned())
                        .min_w_0()
                        .flex_shrink(1.)
                        .flex()
                        .items_center()
                        .gap(view.at(5.))
                        .child(
                            svg()
                                .flex_none()
                                .size(view.at(11.))
                                .path("icons/git-branch.svg")
                                .text_color(view.muted()),
                        )
                        .child(
                            view.settings()
                                .chip("graph-branch-chip".to_owned(), branch, Tone::Neutral)
                                .flex_shrink(1.)
                                .min_w_0()
                                .truncate(),
                        ),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .debug_selector(|| "graph-live".to_owned())
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(view.at(6.))
                    .mr(view.at(4.))
                    .text_size(px(view.scale().detail))
                    .text_color(view.muted())
                    .child(view.status_dot(live, false))
                    .child(self.indicator()),
            )
            .child(
                view.icon_button(
                    "graph-reload",
                    "graph-reload".to_owned(),
                    "icons/refresh.svg",
                    false,
                    true,
                )
                .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
            )
            // A question mark in a circle, where it was a written one.
            .child(
                view.icon_button(
                    "graph-help-button",
                    "graph-help-button".to_owned(),
                    "icons/help.svg",
                    self.help,
                    true,
                )
                .on_click(cx.listener(|this, _, _, cx| this.toggle_help(cx))),
            )
            .child(match self.render_search_field(searching) {
                Some(field) => field,
                None => view
                    .icon_button(
                        "graph-search",
                        "graph-search".to_owned(),
                        "icons/search.svg",
                        false,
                        true,
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.open_search(window, cx)))
                    .into_any_element(),
            })
            .into_any_element()
    }

    /// The gear's own place at the headings' end, which every row keeps
    /// empty so the columns line up.
    pub(super) fn gear_place(&self) -> f32 {
        f32::from(self.look.view().at(GEAR + 2. * GEAR_MARGIN))
    }

    /// The label column's width: the one chosen, narrowed down to
    /// `REFS_MIN` when the graph's side of the tab has no room for the
    /// other columns, its labels then fitting fewer.
    pub(super) fn refs_width(&self) -> f32 {
        let chosen = self.look.at(self.prefs.refs_width);
        let tab = self.width.get();
        if tab <= 0. {
            return chosen;
        }
        let side = if self.panel.is_some() && !self.detail_stacked() {
            tab - self.panel_width()
        } else {
            tab
        };
        let optional: f32 = self
            .optional_columns()
            .iter()
            .map(|(_, _, width)| width)
            .sum();
        let others = self.graph_width() + self.look.at(MESSAGE_MIN) + optional + self.gear_place();
        (side - others).clamp(self.look.at(REFS_MIN).min(chosen), chosen)
    }

    pub(super) fn render_columns(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.look.theme;
        let view = self.look.view();
        let column = |key: &'static str, label: &'static str| {
            div()
                .debug_selector(move || format!("graph-col-{key}"))
                .relative()
                .h_full()
                .flex()
                .items_center()
                .px_2()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(label)
        };
        let edge = |drag: ColumnDrag, key: &'static str| {
            div()
                .id(key)
                .debug_selector(move || key.to_owned())
                .absolute()
                .top_0()
                .right_0()
                .h_full()
                .w(view.at(EDGE))
                .cursor(CursorStyle::ResizeLeftRight)
                .hover(|style| style.bg(rgba(RESIZE_HOVER)))
                .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                    cx.stop_propagation();
                    if event.click_count == 2 {
                        GraphPrefsStore::update(cx, |prefs| match drag {
                            ColumnDrag::Refs => prefs.refs_width = REFS_DEFAULT,
                            ColumnDrag::Graph => prefs.graph_width = None,
                            ColumnDrag::Panel => {}
                        });
                    }
                })
                .on_drag(drag, |_, _, _, cx| cx.new(|_| EmptyView))
        };
        // The columns keep their widths, which the rows share; the headings
        // wear the lists' column style. The gear has its own place at the
        // end, so a heading that runs out of room ends under nothing.
        let titles = div()
            .debug_selector(|| "graph-col-titles".to_owned())
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .overflow_hidden()
            .child(
                column("refs", "Branch / tag")
                    .flex_none()
                    .w(px(self.refs_width()))
                    .pl(view.at(REFS_INSET))
                    .child(edge(ColumnDrag::Refs, "graph-col-refs-resize")),
            )
            .child(
                column("graph", "Graph")
                    .flex_none()
                    .w(px(self.graph_width()))
                    .border_l_1()
                    .border_color(rgb(theme.active))
                    .child(edge(ColumnDrag::Graph, "graph-col-graph-resize")),
            )
            .child(
                column("msg", "Commit message")
                    .flex_1()
                    .min_w(px(self.look.at(MESSAGE_MIN)))
                    .border_l_1()
                    .border_color(rgb(theme.active)),
            )
            .children(
                self.optional_columns()
                    .into_iter()
                    .map(|(key, label, width)| {
                        column(key, label)
                            .flex_none()
                            .w(px(width))
                            .border_l_1()
                            .border_color(rgb(theme.active))
                    }),
            );
        view.columns("graph-cols")
            .relative()
            .w_full()
            .px_0()
            .gap_0()
            .bg(view.background())
            .border_b_1()
            .border_color(view.line())
            .child(titles)
            .child(
                div()
                    .flex_none()
                    .w(px(self.gear_place()))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        view.icon_button(
                            "graph-col-gear",
                            "graph-col-gear".to_owned(),
                            "icons/settings.svg",
                            self.column_menu,
                            true,
                        )
                        .size(view.at(GEAR))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.column_menu = !this.column_menu;
                            cx.notify();
                        })),
                    ),
            )
            .when(self.column_menu, |header| {
                header.child(self.render_column_menu())
            })
            .into_any_element()
    }

    /// The gear's menu: one checkbox per optional column.
    fn render_column_menu(&self) -> AnyElement {
        let theme = &self.look.theme;
        let view = self.look.view();
        let prefs = self.prefs;
        let (colours, ring) = (crate::toggles::colors(theme, true), theme.muted);
        deferred(
            view.menu_panel("graph-col-menu")
                // Clicks here never reach the rows underneath.
                .occlude()
                .absolute()
                .right(view.at(MENU_OFFSET))
                .top(view.at(COLUMNS_H + 2.))
                .min_w(view.at(MENU_WIDTH))
                .font_weight(FontWeight::NORMAL)
                .child(
                    view.menu()
                        .section("graph-col-menu-section".to_owned(), "Columns"),
                )
                .children(OPTIONAL.into_iter().map(|(key, label)| {
                    let on = is_on(&prefs, key);
                    view.row(
                        SharedString::from(format!("graph-col-menu-{key}")),
                        format!("graph-col-menu-{key}"),
                        false,
                        false,
                    )
                    .child(crate::toggles::checkbox_in(
                        colours,
                        ring,
                        f32::from(view.at(CHECKBOX)),
                        on,
                    ))
                    .child(label)
                    .on_click(move |_, _, cx| {
                        GraphPrefsStore::update(cx, |prefs| toggle(prefs, key));
                    })
                })),
        )
        .into_any_element()
    }

    /// Follows a dragged column edge: `x` is the pointer, from the view's
    /// left. Widths are saved as at the face of 12, so every size shares
    /// them.
    pub(super) fn drag_column(&self, drag: ColumnDrag, x: f32, cx: &mut App) {
        let refs = self.refs_width();
        let scale = self.look.scale();
        GraphPrefsStore::update(cx, |prefs| match drag {
            ColumnDrag::Refs => prefs.refs_width = x / scale,
            ColumnDrag::Graph => prefs.graph_width = Some((x - refs) / scale),
            ColumnDrag::Panel => {}
        });
    }
}
