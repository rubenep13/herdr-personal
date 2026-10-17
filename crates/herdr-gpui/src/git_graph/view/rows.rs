//! A row of the graph and its cells, and the column headers above them:
//! BRANCH / TAG · GRAPH · COMMIT MESSAGE, then Author, Date and SHA when
//! they are on.

use super::GitGraphView;
use crate::{
    config::mix,
    git_graph::{
        geometry::{AUTHOR_W, DATE_W, MESSAGE_MIN, ROW_H, SHA_W, lane_colour},
        history::History,
        model::{Commit, CommitKind},
        text::{full_date, relative_time},
    },
    usage::Hint,
};
use gpui::{prelude::*, *};

/// The message cell's tint, while the pointer is over its row, and when
/// its row is selected.
const MESSAGE_TINT: u32 = 9;
const MESSAGE_HOVER: u32 = 15;
const MESSAGE_SELECTED: u32 = 30;

/// Text shown on hover.
fn hint(
    text: impl Into<SharedString>,
    foreground: u32,
    surface: u32,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| {
        cx.new(|_| Hint {
            text: text.clone(),
            foreground,
            surface,
        })
        .into()
    }
}

impl GitGraphView {
    /// The optional columns that are on, with their header and width.
    pub(super) fn optional_columns(&self) -> Vec<(&'static str, &'static str, f32)> {
        [
            (self.prefs.author, "author", "Author", AUTHOR_W),
            (self.prefs.date, "date", "Date", DATE_W),
            (self.prefs.sha, "sha", "SHA", SHA_W),
        ]
        .into_iter()
        .filter(|(on, ..)| *on)
        .map(|(_, key, label, width)| (key, label, width))
        .collect()
    }

    pub(super) fn render_row(
        &mut self,
        history: &History,
        row: usize,
        now: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.look.theme.clone();
        let (Some(commit), Some(node)) = (history.commits.get(row), history.layout.nodes.get(row))
        else {
            return div().h(px(ROW_H)).into_any_element();
        };
        let colour = if commit.kind == CommitKind::Worktree {
            theme.subtext()
        } else {
            lane_colour(&theme, node.colour)
        };
        let chips = history
            .chips
            .get(&commit.oid)
            .filter(|chips| !chips.is_empty());
        let (refs, whole) = match chips {
            Some(chips) => self.render_refs(row, chips, colour, window, cx),
            None => (
                div()
                    .flex_none()
                    .w(px(self.prefs.refs_width))
                    .h_full()
                    .into_any_element(),
                None,
            ),
        };
        let graph = self.render_graph_cell(
            history,
            commit,
            row,
            self.graph_width(),
            colour,
            chips.is_some(),
        );
        // A relative-time marker where the time bucket changes, as GitKraken
        // does; the newest group has none.
        let when = relative_time(commit.authored, now);
        let marker = row
            .checked_sub(1)
            .and_then(|previous| history.commits.get(previous))
            .filter(|previous| !when.is_empty() && relative_time(previous.authored, now) != when)
            .map(|_| self.render_marker(&when, commit.authored));
        div()
            .id(("graph-row", row))
            .debug_selector(move || format!("graph-row-{row}"))
            .group("graph-row")
            .relative()
            .h(px(ROW_H))
            .w_full()
            .flex()
            .items_center()
            .on_click(cx.listener(move |this, _, window, cx| {
                window.focus(&this.focus_handle, cx);
                this.open_detail(row, cx);
            }))
            .child(refs)
            .child(graph)
            .child(self.render_message_cell(history, commit, row, colour))
            .children(self.render_optional_cells(commit, now))
            .children(marker)
            .children(whole)
            .into_any_element()
    }

    fn render_message_cell(
        &self,
        history: &History,
        commit: &Commit,
        row: usize,
        colour: u32,
    ) -> AnyElement {
        let theme = &self.look.theme;
        let tint = if self.selected_row() == Some(row) {
            MESSAGE_SELECTED
        } else {
            MESSAGE_TINT
        };
        let cell = div()
            .id(("graph-msg", row))
            .debug_selector(move || format!("graph-msg-{row}"))
            .flex_1()
            .min_w(px(MESSAGE_MIN))
            .h(px(ROW_H - 2.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(8.))
            .overflow_hidden()
            .whitespace_nowrap()
            .border_l_3()
            .border_color(rgb(colour))
            .bg(rgb(mix(theme.background, colour, tint)))
            .group_hover("graph-row", |style| {
                style.bg(rgb(mix(theme.background, colour, tint.max(MESSAGE_HOVER))))
            });
        if commit.kind == CommitKind::Worktree {
            let counts = history.worktree.unwrap_or_default();
            let pill = |key: &'static str, count: usize, what: &str, ink: u32| {
                (count > 0).then(|| {
                    div()
                        .debug_selector(move || format!("graph-wip-{key}-{row}"))
                        .flex_none()
                        .h(px(18.))
                        .px(px(7.))
                        .rounded(px(9.))
                        .bg(rgb(mix(theme.background, theme.foreground, 6)))
                        .text_color(rgb(ink))
                        .text_size(px(11.))
                        .line_height(px(18.))
                        .child(format!("{count} {what}"))
                })
            };
            return cell
                .child(
                    div()
                        .flex_none()
                        .italic()
                        .text_color(rgb(theme.subtext()))
                        .child("// WIP"),
                )
                .children(pill(
                    "conflicted",
                    counts.conflicted,
                    "conflicted",
                    theme.ink(theme.palette[1]),
                ))
                .children(pill(
                    "staged",
                    counts.staged,
                    "staged",
                    theme.ink(theme.palette[2]),
                ))
                .children(pill(
                    "unstaged",
                    counts.unstaged,
                    "unstaged",
                    theme.ink(theme.palette[3]),
                ))
                .children(pill(
                    "untracked",
                    counts.untracked,
                    "untracked",
                    theme.subtext(),
                ))
                .into_any_element();
        }
        let who = if commit.email.is_empty() {
            commit.author.clone()
        } else {
            format!("{} <{}>", commit.author, commit.email)
        };
        let short: String = commit.oid.chars().take(7).collect();
        let body = if commit.description.is_empty() {
            commit.subject.clone()
        } else {
            format!("{}\n{}", commit.subject, commit.description)
        };
        let text = format!(
            "{body}\n\n{who}\n{} \u{00b7} {short}",
            full_date(commit.authored)
        );
        cell.child(
            div()
                .debug_selector(move || format!("graph-subject-{row}"))
                .flex_none()
                .max_w_full()
                .truncate()
                .text_color(rgb(theme.foreground))
                .child(commit.subject.clone()),
        )
        .when(!commit.description.is_empty(), |cell| {
            cell.child(
                div()
                    .debug_selector(move || format!("graph-desc-{row}"))
                    .min_w_0()
                    .truncate()
                    .text_color(rgb(theme.muted))
                    .child(commit.description.clone()),
            )
        })
        .tooltip(hint(text, theme.foreground, theme.surface))
        .into_any_element()
    }

    fn render_optional_cells(&self, commit: &Commit, now: i64) -> Vec<AnyElement> {
        let theme = &self.look.theme;
        let wip = commit.kind == CommitKind::Worktree;
        self.optional_columns()
            .into_iter()
            .map(|(key, _, width)| {
                let (text, title, mono) = match key {
                    "author" => (commit.author.clone(), commit.email.clone(), false),
                    "date" => (
                        relative_time(commit.authored, now),
                        full_date(commit.authored),
                        false,
                    ),
                    _ if wip => (String::new(), String::new(), true),
                    _ => (
                        commit.oid.chars().take(7).collect(),
                        commit.oid.clone(),
                        true,
                    ),
                };
                div()
                    .id(SharedString::from(format!("graph-{key}-{}", commit.oid)))
                    .flex_none()
                    .w(px(width))
                    .px_2()
                    .truncate()
                    .text_size(px(if mono { 11.5 } else { 12. }))
                    .text_color(rgb(if mono { theme.muted } else { theme.subtext() }))
                    .when(mono, |cell| {
                        use crate::fonts::StyledFont;
                        cell.text_font(&self.look.mono)
                    })
                    .child(text)
                    .when(!title.is_empty(), |cell| {
                        cell.tooltip(hint(title, theme.foreground, theme.surface))
                    })
                    .into_any_element()
            })
            .collect()
    }

    /// A pill on the top edge of the first row of a time bucket.
    fn render_marker(&self, when: &str, at: i64) -> AnyElement {
        let theme = &self.look.theme;
        let tail: f32 = self
            .optional_columns()
            .iter()
            .map(|(_, _, width)| width)
            .sum();
        div()
            .id(SharedString::from(format!("graph-when-{at}")))
            .absolute()
            .top(px(-9.))
            .right(px(tail + 8.))
            .h(px(18.))
            .px(px(7.))
            .rounded(px(3.))
            .bg(rgb(mix(theme.background, theme.foreground, 20)))
            .text_color(rgb(theme.foreground))
            .text_size(px(11.))
            .line_height(px(18.))
            .child(when.to_owned())
            .tooltip(hint(full_date(at), theme.foreground, theme.surface))
            .into_any_element()
    }
}
