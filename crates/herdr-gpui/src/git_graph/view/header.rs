//! The tab's top bar (the repository, its current branch, whether it is
//! live, reload and search) and the column headers, with their resizable
//! edges and the gear that turns on Author, Date and SHA.

use super::GitGraphView;
use crate::{
    browser::checkout_name,
    config::mix,
    git_graph::{
        geometry::{AUTHOR_W, DATE_W, HEADER_H, MESSAGE_MIN, REFS_DEFAULT, SHA_W},
        prefs::{GraphPrefs, GraphPrefsStore},
    },
    panel_resize::RESIZE_HOVER,
};
use gpui::{prelude::*, *};

/// The top bar's height.
const BAR_H: f32 = 40.;
/// How wide a column's grabbable edge is.
const EDGE: f32 = 6.;

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

    pub(super) fn render_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.look.theme;
        let (name, branch) = self.repository_and_branch();
        let button = |id: &'static str, icon: &'static str, enabled: bool| {
            div()
                .id(id)
                .debug_selector(move || id.to_owned())
                .flex_none()
                .size(px(28.))
                .rounded(px(4.))
                .flex()
                .items_center()
                .justify_center()
                .when(enabled, |el| {
                    el.cursor_pointer()
                        .hover(|style| style.bg(rgb(theme.active)))
                })
                .child(svg().size(px(14.)).path(icon).text_color(rgb(if enabled {
                    theme.subtext()
                } else {
                    theme.muted
                })))
        };
        let live = match self.connection {
            super::Connection::Online => theme.ink(theme.palette[2]),
            super::Connection::Offline => theme.muted,
        };
        div()
            .debug_selector(|| "graph-bar".to_owned())
            .flex_none()
            .h(px(BAR_H))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .bg(rgb(theme.surface))
            .border_b_1()
            .border_color(rgb(theme.active))
            .line_height(px(BAR_H))
            .child(
                div()
                    .flex_none()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(theme.foreground))
                    .child(name),
            )
            .when_some(branch, |bar, branch| {
                bar.child(
                    div()
                        .debug_selector(|| "graph-branch".to_owned())
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_color(rgb(theme.subtext()))
                        .child(
                            svg()
                                .flex_none()
                                .size(px(13.))
                                .path("icons/graph-check.svg")
                                .text_color(rgb(theme.subtext())),
                        )
                        .child(div().min_w_0().truncate().child(branch)),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .debug_selector(|| "graph-live".to_owned())
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.5))
                    .text_color(rgb(theme.muted))
                    .child(div().size(px(7.)).rounded_full().bg(rgb(live)))
                    .child(self.indicator()),
            )
            .child(
                button("graph-reload", "icons/graph-refresh.svg", true)
                    .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
            )
            .child(button("graph-search", "icons/search.svg", false))
            .into_any_element()
    }

    pub(super) fn render_columns(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.look.theme;
        let prefs = self.prefs;
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
                .w(px(EDGE))
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
        div()
            .debug_selector(|| "graph-cols".to_owned())
            .relative()
            .flex_none()
            .h(px(HEADER_H))
            .w_full()
            .flex()
            .bg(rgb(theme.surface))
            .border_b_1()
            .border_color(rgb(theme.active))
            .text_color(rgb(theme.muted))
            .text_size(px(10.5))
            .line_height(px(HEADER_H))
            .font_weight(FontWeight::SEMIBOLD)
            .child(
                column("refs", "BRANCH / TAG")
                    .flex_none()
                    .w(px(prefs.refs_width))
                    .pl(px(16.))
                    .child(edge(ColumnDrag::Refs, "graph-col-refs-resize")),
            )
            .child(
                column("graph", "GRAPH")
                    .flex_none()
                    .w(px(self.graph_width()))
                    .border_l_1()
                    .border_color(rgb(theme.active))
                    .child(edge(ColumnDrag::Graph, "graph-col-graph-resize")),
            )
            .child(
                column("msg", "COMMIT MESSAGE")
                    .flex_1()
                    .min_w(px(MESSAGE_MIN))
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
            )
            .child(
                div()
                    .id("graph-col-gear")
                    .debug_selector(|| "graph-col-gear".to_owned())
                    .absolute()
                    .right(px(4.))
                    .top(px(2.))
                    .size(px(22.))
                    .rounded(px(3.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(theme.active)))
                    .child(
                        svg()
                            .size(px(14.))
                            .path("icons/graph-settings.svg")
                            .text_color(rgb(theme.muted)),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.column_menu = !this.column_menu;
                        cx.notify();
                    })),
            )
            .when(self.column_menu, |header| {
                header.child(self.render_column_menu())
            })
            .into_any_element()
    }

    /// The gear's menu: one checkbox per optional column.
    fn render_column_menu(&self) -> AnyElement {
        let theme = &self.look.theme;
        let prefs = self.prefs;
        let widths = [AUTHOR_W, DATE_W, SHA_W];
        deferred(
            div()
                .id("graph-col-menu")
                .debug_selector(|| "graph-col-menu".to_owned())
                // Clicks here never reach the rows underneath.
                .occlude()
                .absolute()
                .right(px(6.))
                .top(px(HEADER_H + 2.))
                .min_w(px(160.))
                .p(px(6.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(theme.active))
                .bg(rgb(mix(theme.surface, theme.foreground, 4)))
                .shadow_lg()
                .flex()
                .flex_col()
                .text_size(px(12.5))
                .line_height(px(24.))
                .font_weight(FontWeight::NORMAL)
                .child(
                    div()
                        .px(px(6.))
                        .text_size(px(11.))
                        .text_color(rgb(theme.muted))
                        .child("COLUMNS"),
                )
                .children(OPTIONAL.into_iter().zip(widths).map(|((key, label), _)| {
                    let on = is_on(&prefs, key);
                    div()
                        .id(SharedString::from(format!("graph-col-menu-{key}")))
                        .debug_selector(move || format!("graph-col-menu-{key}"))
                        .px(px(6.))
                        .rounded(px(4.))
                        .flex()
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .text_color(rgb(theme.foreground))
                        .hover(|style| style.bg(rgb(theme.active)))
                        .child(
                            div()
                                .size(px(14.))
                                .rounded(px(3.))
                                .border_1()
                                .border_color(rgb(theme.subtext()))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(on, |box_| {
                                    box_.child(
                                        svg()
                                            .size(px(11.))
                                            .path("icons/graph-check.svg")
                                            .text_color(rgb(theme.foreground)),
                                    )
                                }),
                        )
                        .child(label)
                        .on_click(move |_, _, cx| {
                            GraphPrefsStore::update(cx, |prefs| toggle(prefs, key));
                        })
                })),
        )
        .into_any_element()
    }

    /// Follows a dragged column edge: `x` is the pointer, from the view's left.
    pub(super) fn drag_column(&self, drag: ColumnDrag, x: f32, cx: &mut App) {
        let refs = self.prefs.refs_width;
        GraphPrefsStore::update(cx, |prefs| match drag {
            ColumnDrag::Refs => prefs.refs_width = x,
            ColumnDrag::Graph => prefs.graph_width = Some(x - refs),
            ColumnDrag::Panel => {}
        });
    }
}
