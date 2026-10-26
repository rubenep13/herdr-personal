//! The panel beside the graph, or under it in a narrow tab: the selected
//! commit's message, author, date, parents and labels, the files it
//! changed, and the diff of the one chosen, as hgg's details panel shows
//! them. Beside the graph it resizes by its left edge.

use super::{GitGraphView, header::ColumnDrag};
use crate::{
    config::Theme,
    design::{ButtonKind, Edge, Tone},
    fonts::StyledFont,
    git_graph::{
        detail::{CommitDetail, FileChange, file_counts, file_label},
        error::Error,
        model::{CommitKind, WORKTREE_OID},
        prefs::PANEL_MIN,
        text::{author_colour, full_date, initials, relative_time},
        worker::Request,
        worktree::{Section, WorktreeStatus},
    },
    panel_resize::RESIZE_HOVER,
    review::{BINARY_NOTE, Kind, Lines, highlight::Span},
};
use gpui::{prelude::*, *};
use std::sync::Arc;

/// Pages read at most looking for a parent that is not loaded yet.
const JUMP_PAGES: usize = 10;
/// The share of the tab the panel may take.
const PANEL_SHARE: f32 = 0.75;
/// The narrowest the graph gets beside the panel, at the face of 12;
/// narrower, the panel goes under it.
const GRAPH_SIDE_MIN: f32 = 400.;
const DIFF_LINE_H: f32 = 18.;
/// The panel's measures at the graph's `[ui]`-sized face of 12: its resize
/// edge, padding and line height, the captions' column, the subject's size
/// over the face's names, a hash's text and the author's avatar.
const RESIZE_EDGE: f32 = 5.;
const PADDING: f32 = 14.;
const LINE: f32 = 20.;
const TERM_WIDTH: f32 = 76.;
const TITLE_OVER: f32 = 3.;
const MONO: f32 = 12.;
const AVATAR: f32 = 22.;
const AVATAR_TEXT: f32 = 9.;

/// The colour a diff line is tinted with: the theme's green for an added
/// line, its red for a removed one, its blue for a hunk header.
pub(crate) fn diff_tint(theme: &Theme, kind: Kind) -> Option<u32> {
    let index = match kind {
        Kind::Added => 2,
        Kind::Removed => 1,
        Kind::Hunk => 4,
        Kind::Context | Kind::Meta => return None,
    };
    Some(theme.ink(theme.palette[index]))
}

/// The commit a panel shows and what has been read of it.
pub(super) struct Panel {
    pub(super) oid: String,
    generation: u64,
    detail: Option<Arc<CommitDetail>>,
    /// The uncommitted changes, when the panel shows the WIP row.
    pub(super) worktree: Option<Arc<WorktreeStatus>>,
    error: Option<Arc<Error>>,
    pub(super) file: Option<usize>,
    diff: Option<Diff>,
    scroll: ScrollHandle,
    diff_scroll: UniformListScrollHandle,
}

/// A diff as the worker read it.
pub(super) struct DiffLines {
    pub(super) lines: Arc<Lines>,
    pub(super) spans: Arc<Vec<Vec<Span>>>,
    pub(super) truncated: bool,
    pub(super) binary: bool,
}

/// The chosen file's diff.
pub(super) struct Diff {
    path: String,
    /// Which side of an uncommitted file it shows.
    section: Option<Section>,
    generation: u64,
    lines: Option<Arc<Lines>>,
    spans: Arc<Vec<Vec<Span>>>,
    truncated: bool,
    /// Not text: it has no lines to show.
    binary: bool,
    error: Option<Arc<Error>>,
}

impl Panel {
    fn new(oid: String, generation: u64) -> Self {
        Self {
            oid,
            generation,
            detail: None,
            worktree: None,
            error: None,
            file: None,
            diff: None,
            scroll: ScrollHandle::new(),
            diff_scroll: UniformListScrollHandle::new(),
        }
    }

    pub(super) fn diff(&self) -> Option<&Diff> {
        self.diff.as_ref()
    }

    pub(super) fn diff_shown(&self) -> bool {
        self.diff.is_some()
    }

    /// The generation of the request still out for it, of either kind.
    pub(super) fn waits_for(&self, generation: u64) -> Option<bool> {
        if self.generation == generation && self.detail.is_none() && self.worktree.is_none() {
            return Some(true);
        }
        self.diff
            .as_ref()
            .filter(|diff| diff.generation == generation && diff.lines.is_none())
            .map(|_| false)
    }
}

impl GitGraphView {
    /// Shows `row`'s commit in the panel, opening it if needed.
    pub(crate) fn open_detail(&mut self, row: usize, cx: &mut Context<Self>) {
        let Some(commit) = self
            .history
            .as_ref()
            .and_then(|history| history.commits.get(row))
            .cloned()
        else {
            return;
        };
        self.select(row);
        if self
            .panel
            .as_ref()
            .is_some_and(|panel| panel.oid == commit.oid)
        {
            cx.notify();
            return;
        }
        let generation = if commit.kind == CommitKind::Worktree {
            self.send(Request::Worktree)
        } else {
            self.send(Request::Detail(commit.oid.clone()))
        };
        self.panel = Some(Panel::new(commit.oid, generation));
        cx.notify();
    }

    pub(super) fn close_detail(&mut self, cx: &mut Context<Self>) -> bool {
        if self.panel.take().is_none() {
            return false;
        }
        cx.notify();
        true
    }

    /// Asks for the diff of the panel's file `index`: a commit's, or an
    /// uncommitted one's in its section.
    pub(super) fn choose_file(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some((oid, section, file)) = self.panel.as_ref().and_then(|panel| {
            let (section, file) = match &panel.worktree {
                Some(status) => {
                    let (section, file) = status.files.get(index)?;
                    (Some(*section), file.clone())
                }
                None => (None, panel.detail.as_ref()?.files.get(index)?.clone()),
            };
            Some((panel.oid.clone(), section, file))
        }) else {
            return;
        };
        let path = file.git_path.clone();
        let old_path = file.git_old_path.clone();
        let generation = match section {
            Some(section) => self.send(Request::WorktreeDiff {
                section,
                path,
                old_path,
            }),
            None => self.send(Request::Diff {
                oid,
                path,
                old_path,
            }),
        };
        if let Some(panel) = self.panel.as_mut() {
            panel.file = Some(index);
            panel.diff = Some(Diff {
                path: file.git_path,
                section,
                generation,
                lines: None,
                spans: Arc::default(),
                truncated: false,
                binary: false,
                error: None,
            });
        }
        cx.notify();
    }

    /// Selects `oid`'s row, reading up to `JUMP_PAGES` more pages to find it.
    pub(crate) fn jump_to(&mut self, oid: &str, cx: &mut Context<Self>) {
        let Some(history) = self.history.clone() else {
            return;
        };
        match history.row_of(oid) {
            Some(row) => self.arrive(row, cx),
            None if history.has_more => {
                self.jump = Some((oid.to_owned(), JUMP_PAGES - 1));
                self.send(Request::More);
            }
            None => {}
        }
    }

    /// Selects `row` after a jump, following it in the panel.
    fn arrive(&mut self, row: usize, cx: &mut Context<Self>) {
        if self.panel.is_some() {
            self.open_detail(row, cx);
        } else {
            self.select_row(row, cx);
        }
    }

    /// After a page arrives: the commit a jump looks for, or the next page.
    pub(super) fn continue_jump(&mut self, cx: &mut Context<Self>) {
        let (Some((oid, left)), Some(history)) = (self.jump.take(), self.history.clone()) else {
            return;
        };
        if let Some(row) = history.row_of(&oid) {
            self.arrive(row, cx);
        } else if left > 0 && history.has_more {
            self.jump = Some((oid, left - 1));
            self.send(Request::More);
        }
    }

    pub(super) fn receive_detail(
        &mut self,
        generation: u64,
        detail: Result<Arc<CommitDetail>, Arc<Error>>,
    ) -> bool {
        let Some(panel) = self.panel.as_mut() else {
            return false;
        };
        if generation < panel.generation {
            return false;
        }
        if let Ok(detail) = &detail
            && detail.commit.oid != panel.oid
        {
            return false;
        }
        match detail {
            Ok(detail) => panel.detail = Some(detail),
            Err(error) => panel.error = Some(error),
        }
        true
    }

    pub(super) fn receive_worktree(
        &mut self,
        generation: u64,
        status: Arc<WorktreeStatus>,
    ) -> bool {
        let Some(panel) = self
            .panel
            .as_mut()
            .filter(|panel| panel.oid == WORKTREE_OID)
        else {
            return false;
        };
        if generation < panel.generation {
            return false;
        }
        panel.worktree = Some(status);
        true
    }

    /// A diff for the panel's chosen file; `key` is the commit, path and
    /// section it answers, unknown for a failure.
    pub(super) fn receive_diff(
        &mut self,
        generation: u64,
        key: Option<(&str, &str, Option<Section>)>,
        diff: Result<DiffLines, Arc<Error>>,
    ) -> bool {
        let Some(panel) = self.panel.as_mut() else {
            return false;
        };
        let oid = panel.oid.as_str();
        let Some(shown) = panel.diff.as_mut() else {
            return false;
        };
        if generation < shown.generation
            || key.is_some_and(|(answered, path, section)| {
                answered != oid || path != shown.path || section != shown.section
            })
        {
            return false;
        }
        match diff {
            Ok(read) => {
                shown.lines = Some(read.lines);
                shown.spans = read.spans;
                shown.truncated = read.truncated;
                shown.binary = read.binary;
            }
            Err(error) => shown.error = Some(error),
        }
        true
    }

    /// The panel's width in a tab `tab` wide: chosen, within 320 px and
    /// three quarters of the tab.
    pub(super) fn panel_width(&self) -> f32 {
        let tab = self.width.get();
        let least = self.look.at(PANEL_MIN);
        let most = if tab > 0. {
            (tab * PANEL_SHARE).max(least)
        } else {
            f32::MAX
        };
        self.look.at(self.prefs.panel_width).clamp(least, most)
    }

    /// Whether the panel goes under the graph: beside it, at the width it
    /// was given, the graph would get narrower than `GRAPH_SIDE_MIN`.
    /// Unknown before the first frame.
    pub(super) fn detail_stacked(&self) -> bool {
        let tab = self.width.get();
        self.panel.is_some()
            && tab > 0.
            && tab - self.look.at(self.prefs.panel_width.max(PANEL_MIN))
                < f32::from(self.look.view().at(GRAPH_SIDE_MIN))
    }

    /// Follows the panel's dragged edge: `x` from the tab's left, in a tab
    /// `tab` wide. The graph keeps `GRAPH_SIDE_MIN` beside it.
    pub(super) fn drag_panel(&self, x: f32, tab: f32, cx: &mut App) {
        let graph = f32::from(self.look.view().at(GRAPH_SIDE_MIN));
        let least = self.look.at(PANEL_MIN);
        let most = (tab * PANEL_SHARE).min(tab - graph).max(least);
        // Saved as at the face of 12, as the columns are.
        let width = (tab - x).clamp(least, most) / self.look.scale();
        crate::git_graph::prefs::GraphPrefsStore::update(cx, |prefs| prefs.panel_width = width);
    }

    #[cfg(test)]
    pub(crate) fn close_detail_for_test(&mut self, cx: &mut Context<Self>) {
        self.close_detail(cx);
    }

    #[cfg(test)]
    pub(crate) fn last_detail(&self) -> u64 {
        self.panel.as_ref().map_or(0, |panel| panel.generation)
    }

    pub(super) fn render_panel(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let panel = self.panel.as_ref()?;
        let view = self.look.view();
        let edge = div()
            .id("graph-detail-resize")
            .debug_selector(|| "graph-detail-resize".to_owned())
            .absolute()
            .top_0()
            .left_0()
            .h_full()
            .w(view.at(RESIZE_EDGE))
            .cursor(CursorStyle::ResizeLeftRight)
            .hover(|style| style.bg(rgba(RESIZE_HOVER)))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_drag(ColumnDrag::Panel, |_, _, _, cx| cx.new(|_| EmptyView));
        let close = view
            .icon_button(
                "graph-detail-close",
                "graph-detail-close".to_owned(),
                "icons/close.svg",
                false,
                true,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_detail(cx);
            }));
        let title = if panel.oid == WORKTREE_OID {
            "Uncommitted changes"
        } else {
            "Commit"
        };
        let body = match (&panel.detail, &panel.error) {
            _ if panel.oid == WORKTREE_OID => match (&panel.worktree, &panel.error) {
                (Some(status), _) => self.render_worktree(panel, status, cx),
                (None, Some(error)) => self.panel_note(&crate::git_graph::view::error_title(
                    error,
                    self.remote.as_deref(),
                )),
                (None, None) => self.panel_note("Loading changes\u{2026}"),
            },
            (Some(detail), _) => self.render_commit(panel, detail, cx),
            (None, Some(error)) => self.panel_note(&crate::git_graph::view::error_title(
                error,
                self.remote.as_deref(),
            )),
            (None, None) => self.panel_note("Loading commit\u{2026}"),
        };
        // Under the graph it takes half the height and keeps the width it
        // was given for when there is room beside.
        let stacked = self.detail_stacked();
        Some(
            view.side_panel("graph-detail", Edge::Right)
                .id("graph-detail")
                .relative()
                .map(|panel| {
                    if stacked {
                        panel.border_l_0().border_t_1().w_full().h(relative(0.5))
                    } else {
                        panel.w(px(self.panel_width())).h_full()
                    }
                })
                .line_height(view.at(LINE))
                .child(
                    view.side_header("graph-detail-header", title, None)
                        .child(close),
                )
                .child(body)
                .when(!stacked, |panel| panel.child(edge))
                .into_any_element(),
        )
    }

    pub(super) fn panel_note(&self, text: &str) -> AnyElement {
        let view = self.look.view();
        div()
            .px(view.at(PADDING))
            .py(view.at(12.))
            .text_color(view.muted())
            .child(text.to_owned())
            .into_any_element()
    }

    fn render_commit(
        &self,
        panel: &Panel,
        detail: &CommitDetail,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = &self.look.theme;
        let view = self.look.view();
        let commit = &detail.commit;
        let now = chrono::Utc::now().timestamp();
        let key = if commit.email.is_empty() {
            &commit.author
        } else {
            &commit.email
        };
        let term = |label: &str| {
            div()
                .flex_none()
                .w(view.at(TERM_WIDTH))
                .text_size(px(view.scale().secondary))
                .text_color(view.muted())
                .child(label.to_owned())
        };
        let row = |label: &str, value: AnyElement| {
            div()
                .flex()
                .items_start()
                .gap_2()
                .min_w_0()
                .child(term(label))
                .child(div().flex_1().min_w_0().child(value))
        };
        let oid = commit.oid.clone();
        let copy = view
            .button(
                "graph-detail-copy",
                "graph-detail-copy".to_owned(),
                Some("icons/copy.svg"),
                "Copy",
                ButtonKind::Normal,
                true,
            )
            .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(oid.clone()))
            });
        let history = self.history.clone();
        let parents = div()
            .flex()
            .flex_col()
            .children(commit.parents.iter().enumerate().map(|(index, parent)| {
                let target = parent.clone();
                let subject = history
                    .as_ref()
                    .and_then(|history| {
                        let row = history.row_of(parent)?;
                        Some(history.commits.get(row)?.subject.clone())
                    })
                    .unwrap_or_default();
                div()
                    .flex()
                    .gap_2()
                    .min_w_0()
                    .child(
                        div()
                            .id(("graph-parent", index))
                            .debug_selector(move || format!("graph-parent-{index}"))
                            .flex_none()
                            .text_font(&self.look.mono)
                            .text_color(rgb(theme.ink(theme.palette[4])))
                            .cursor_pointer()
                            .child(parent.chars().take(7).collect::<String>())
                            .on_click(cx.listener(move |this, _, _, cx| this.jump_to(&target, cx))),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_color(rgb(theme.muted))
                            .child(subject),
                    )
            }));
        let parents = if commit.parents.is_empty() {
            div()
                .text_color(rgb(theme.muted))
                .child("None (root commit)")
                .into_any_element()
        } else {
            parents.into_any_element()
        };
        let labels: Vec<String> = history
            .as_ref()
            .and_then(|history| history.chips.get(&commit.oid))
            .map(|chips| {
                chips
                    .iter()
                    .map(|chip| {
                        if chip.current && chip.label != "HEAD" {
                            format!("HEAD \u{2192} {}", chip.label)
                        } else {
                            chip.label.clone()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let additions: u32 = detail.files.iter().filter_map(|file| file.additions).sum();
        let deletions: u32 = detail.files.iter().filter_map(|file| file.deletions).sum();
        let files = div()
            .flex()
            .flex_col()
            .children(detail.files.iter().enumerate().map(|(index, file)| {
                self.render_file(index, file, panel.file == Some(index), true, cx)
            }));
        let info = div()
            .id("graph-detail-info")
            .px(view.at(PADDING))
            .pb(view.at(12.))
            .flex()
            .flex_col()
            .gap_2()
            .overflow_y_scroll()
            .track_scroll(&panel.scroll)
            .when(panel.diff.is_some(), |el| el.max_h(relative(0.45)))
            .when(panel.diff.is_none(), |el| el.flex_1())
            .child(
                div()
                    .text_size(px(view.scale().name + TITLE_OVER))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(view.title())
                    .child(commit.subject.clone()),
            )
            .when(!detail.body.is_empty(), |el| {
                el.child(
                    view.dialog()
                        .well("graph-detail-body".to_owned(), Some(&self.look.mono))
                        .whitespace_normal()
                        .child(detail.body.clone()),
                )
            })
            .child(row(
                "Commit",
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_font(&self.look.mono)
                            .text_size(view.at(MONO))
                            .child(commit.oid.clone()),
                    )
                    .child(copy)
                    .into_any_element(),
            ))
            .child(row(
                "Author",
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .size(view.at(AVATAR))
                            .rounded_full()
                            .bg(author_colour(key))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(white())
                            .text_size(view.at(AVATAR_TEXT))
                            .font_weight(FontWeight::BOLD)
                            .child(initials(&commit.author)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .child(if commit.email.is_empty() {
                                commit.author.clone()
                            } else {
                                format!("{} <{}>", commit.author, commit.email)
                            }),
                    )
                    .into_any_element(),
            ))
            .child(row(
                "Date",
                div()
                    .child(format!(
                        "{} \u{00b7} {}",
                        full_date(commit.authored),
                        relative_time(commit.authored, now)
                    ))
                    .into_any_element(),
            ))
            .child(row(
                if commit.parents.len() > 1 {
                    "Parents"
                } else {
                    "Parent"
                },
                parents,
            ))
            .when(!labels.is_empty(), |el| {
                el.child(row(
                    "Refs",
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(view.at(4.))
                        .children(labels.into_iter().enumerate().map(|(index, label)| {
                            view.settings().chip(
                                format!("graph-detail-ref-{index}"),
                                label,
                                Tone::Neutral,
                            )
                        }))
                        .into_any_element(),
                ))
            })
            .child(
                div()
                    .mt(view.at(8.))
                    .flex()
                    .gap(view.at(10.))
                    .text_size(px(view.scale().detail))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(view.muted())
                    .child(if detail.files.len() == 1 {
                        "1 file".to_owned()
                    } else {
                        format!("{} files", detail.files.len())
                    })
                    .child(
                        div()
                            .text_color(rgb(theme.ink(theme.palette[2])))
                            .child(format!("+{additions}")),
                    )
                    .child(
                        div()
                            .text_color(rgb(theme.ink(theme.palette[1])))
                            .child(format!("\u{2212}{deletions}")),
                    ),
            )
            .child(if detail.files.is_empty() {
                div()
                    .text_color(view.muted())
                    .child("This commit changes no files.")
                    .into_any_element()
            } else {
                files.into_any_element()
            });
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(info)
            .children(
                panel
                    .diff
                    .as_ref()
                    .map(|diff| self.render_diff(panel, diff, cx)),
            )
            .into_any_element()
    }

    /// One file of a list, by its letter and name, with its line counts
    /// when `counts`; a click shows its diff.
    pub(super) fn render_file(
        &self,
        index: usize,
        file: &FileChange,
        chosen: bool,
        counts: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = &self.look.theme;
        let letter_colour = match file.kind {
            'A' | '?' => theme.ink(theme.palette[2]),
            'D' | 'U' => theme.ink(theme.palette[1]),
            'R' | 'C' => theme.ink(theme.palette[4]),
            _ => theme.ink(theme.palette[3]),
        };
        let view = self.look.view();
        view.row(
            ("graph-file", index),
            format!("graph-file-{index}"),
            false,
            chosen,
        )
        .w_full()
        .child(
            div()
                .flex_none()
                .w(view.at(14.))
                .text_font(&self.look.mono)
                .font_weight(FontWeight::BOLD)
                .text_size(view.at(11.))
                .text_color(rgb(letter_colour))
                .child(file.kind.to_string()),
        )
        .child(div().flex_1().min_w_0().truncate().child(file_label(file)))
        .when(counts, |el| {
            el.child(
                div()
                    .flex_none()
                    .text_font(&self.look.mono)
                    .text_size(view.at(11.))
                    .text_color(view.muted())
                    .child(file_counts(file)),
            )
        })
        .on_click(cx.listener(move |this, _, _, cx| this.choose_file(index, cx)))
        .into_any_element()
    }

    pub(super) fn render_diff(
        &self,
        panel: &Panel,
        diff: &Diff,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = &self.look.theme;
        let path = match diff.section {
            Some(section) => format!("{} \u{00b7} {}", section.title(), diff.path),
            None => diff.path.clone(),
        };
        let title = div()
            .flex_none()
            .px_4()
            .py_1()
            .text_font(&self.look.mono)
            .text_size(px(12.))
            .text_color(rgb(theme.subtext()))
            .truncate()
            .child(path);
        let body = match (&diff.lines, &diff.error) {
            (Some(_), _) if diff.binary => div()
                .debug_selector(|| "graph-diff-binary".to_owned())
                .child(self.panel_note(BINARY_NOTE))
                .into_any_element(),
            (Some(lines), _) if lines.is_empty() => self.panel_note("No text changes."),
            (Some(lines), _) => uniform_list(
                "graph-diff",
                lines.len(),
                cx.processor(|this, range: std::ops::Range<usize>, _, _| {
                    let Some(diff) = this.panel.as_ref().and_then(|panel| panel.diff.as_ref())
                    else {
                        return Vec::new();
                    };
                    let Some(lines) = diff.lines.clone() else {
                        return Vec::new();
                    };
                    range
                        .map(|index| this.render_diff_line(&lines, &diff.spans, index))
                        .collect()
                }),
            )
            .track_scroll(&panel.diff_scroll)
            .flex_1()
            .min_h_0()
            .into_any_element(),
            (None, Some(error)) => self.panel_note(&crate::git_graph::view::error_title(
                error,
                self.remote.as_deref(),
            )),
            (None, None) => self.panel_note("Loading diff\u{2026}"),
        };
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(rgb(theme.active))
            .child(title)
            .child(body)
            .when(diff.truncated, |el| {
                el.child(
                    div()
                        .debug_selector(|| "graph-diff-truncated".to_owned())
                        .flex_none()
                        .px_4()
                        .py_1()
                        .text_size(px(12.))
                        .text_color(rgb(theme.muted))
                        .child(format!(
                            "Diff cut at {} lines.",
                            crate::git_graph::detail::MAX_DIFF_LINES
                        )),
                )
            })
            .into_any_element()
    }

    fn render_diff_line(&self, lines: &Lines, spans: &[Vec<Span>], index: usize) -> AnyElement {
        let theme = &self.look.theme;
        let Some(line) = lines.get(index) else {
            return div().h(px(DIFF_LINE_H)).into_any_element();
        };
        let tint = diff_tint(theme, line.kind);
        let number = |value: Option<u32>| {
            div()
                .flex_none()
                .w(px(42.))
                .pr(px(6.))
                .flex()
                .justify_end()
                .text_color(rgb(theme.muted))
                .child(value.map(|value| value.to_string()).unwrap_or_default())
        };
        let sign = match line.kind {
            Kind::Added => "+",
            Kind::Removed => "-",
            Kind::Context => " ",
            Kind::Hunk | Kind::Meta => "",
        };
        let text = lines.text_of(line);
        let code = if matches!(line.kind, Kind::Hunk | Kind::Meta) {
            StyledText::new(text.to_owned())
        } else {
            crate::review::styled_code(
                theme,
                text,
                spans.get(index).map_or(&[][..], Vec::as_slice),
                &[],
                None,
                None,
            )
        };
        let background = tint.map(|tint| {
            let mut wash: Hsla = rgb(tint).into();
            wash.a = if line.kind == Kind::Hunk { 0.12 } else { 0.14 };
            wash
        });
        div()
            .debug_selector(move || format!("graph-diff-line-{index}"))
            .h(px(DIFF_LINE_H))
            .flex()
            .whitespace_nowrap()
            .text_font(&self.look.mono)
            .text_size(px(12.))
            .line_height(px(DIFF_LINE_H))
            .when_some(background, |el, wash| el.bg(wash))
            .when(line.kind == Kind::Hunk, |el| {
                el.text_color(rgb(theme.ink(theme.palette[4])))
            })
            .child(number(line.old))
            .child(number(line.new))
            .child(div().pl_1().pr_2().child(sign))
            .child(code)
            .into_any_element()
    }
}
