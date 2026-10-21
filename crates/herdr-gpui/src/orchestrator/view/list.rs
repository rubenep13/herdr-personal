//! The inbox's list: its column heads and virtualized rows for issues, pull
//! requests, and runs, each a click to select and a double-click to open.

use super::{
    OrchestratorView,
    look::{GREEN, Look, RED, YELLOW, age, agent_icon},
    rows::{Group, ItemRow, RunLine, Tab},
};
use crate::{
    design::Tone,
    fonts::StyledFont,
    orchestrator::{Owner, Provider, SyncState},
};
use gpui::{prelude::*, *};
use herdr_client::protocol::AgentStatus;

/// Whether a tab's rows hold two lines: a pull request's author and branch
/// under its title, a run's agent beside what it works on.
fn two_lines(tab: Tab) -> bool {
    tab != Tab::Issues
}

/// How far a list keeps its rows from its sides, at the face's size of 12.
const LIST_INSET: f32 = 8.;

impl OrchestratorView {
    /// The height every row of the shown tab takes, which the virtual list
    /// relies on.
    fn row_height(&self) -> f32 {
        self.look.view().row_height(two_lines(self.tab))
    }

    pub(super) fn render_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let look = &self.look;
        let (count, head): (usize, &[(&str, Option<f32>)]) = match self.tab {
            Tab::Issues => (
                self.issue_rows.len(),
                &[
                    ("Age", Some(32.)),
                    ("Src", Some(22.)),
                    ("Run", Some(30.)),
                    ("Title", None),
                    ("Author", Some(110.)),
                    ("\u{1f4ac}", Some(34.)),
                    ("State", Some(86.)),
                ],
            ),
            Tab::PullRequests => (
                self.pull_request_rows.len(),
                &[
                    ("PR", Some(44.)),
                    ("Run", Some(30.)),
                    ("Title \u{00b7} author \u{00b7} branch", None),
                    ("Diff", Some(44.)),
                    ("Activity", Some(70.)),
                    ("State", Some(60.)),
                    ("", Some(28.)),
                ],
            ),
            Tab::Runs => (self.run_lines.len(), &[]),
        };
        let list = if count == 0 {
            self.render_empty(cx).into_any_element()
        } else {
            uniform_list(
                "orchestrator-rows",
                count,
                cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                    range.map(|index| this.render_row(index, cx)).collect()
                }),
            )
            .track_scroll(&self.scroll)
            .flex_1()
            .min_h_0()
            // Rows keep off the list's sides, as every list's do.
            .px(look.view().at(LIST_INSET))
            .into_any_element()
        };
        div()
            .debug_selector(|| "orchestrator-list".into())
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .when(!head.is_empty(), |el| el.child(column_head(look, head)))
            .child(list)
            .into_any_element()
    }

    fn render_empty(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let look = &self.look;
        let what = match self.tab {
            Tab::Issues => "open issues",
            Tab::PullRequests => "pull requests",
            Tab::Runs => "runs",
        };
        let loading = self.snapshot.error.is_none()
            && (self.snapshot.repo.is_none()
                || self
                    .snapshot
                    .sources
                    .iter()
                    .any(|status| matches!(status.state, SyncState::Syncing)));
        let text = if loading && self.query.trim().is_empty() {
            "Loading\u{2026}".to_owned()
        } else if self.query.trim().is_empty() {
            format!("No {what} yet")
        } else {
            format!("No {what} match \u{201c}{}\u{201d}", self.query.trim())
        };
        look.view()
            .empty("orchestrator-empty", Some("icons/inbox.svg"), text)
            .flex_1()
            .when(!self.query.trim().is_empty(), |el| {
                el.child(
                    look.button("orchestrator-clear", "Clear filter", false)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.search.update(cx, |search, cx| search.clear(cx));
                        })),
                )
            })
    }

    fn render_row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        match self.tab {
            Tab::Issues => self.render_issue_row(index, cx),
            Tab::PullRequests => self.render_pull_request_row(index, cx),
            Tab::Runs => self.render_run_line(index, cx),
        }
    }

    /// A row's shell: selection, click to select, double-click to open.
    fn row_shell(
        &self,
        id: (&'static str, usize),
        key: String,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let selected = self.selected.as_deref() == Some(key.as_str());
        let id: ElementId = id.into();
        let selector = id.to_string();
        self.look
            .view()
            .row(id, selector, two_lines(self.tab), selected)
            .w_full()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.selected = Some(key.clone());
                    if event.click_count == 2 {
                        this.open_detail(key.clone(), cx);
                    }
                    cx.notify();
                }),
            )
    }

    fn render_issue_row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.issue_rows.get(index) else {
            return div().h(px(self.row_height())).into_any_element();
        };
        let look = &self.look;
        let theme = &look.theme;
        let item = &self.snapshot.items[row.item];
        let key = item.key.canonical();
        let now = chrono::Utc::now();
        let expander = row.children.map(|open| {
            let key = key.clone();
            div()
                .id(("orchestrator-expand", index))
                .child(look.icon(
                    if open {
                        "icons/chevron-down.svg"
                    } else {
                        "icons/chevron-right.svg"
                    },
                    10.,
                    theme.muted,
                ))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_children(key.clone(), cx);
                    }),
                )
        });
        self.row_shell(("orchestrator-issue", index), key, cx)
            .child(
                div()
                    .flex_none()
                    .w(px(32.))
                    .text_size(look.small())
                    .text_color(rgb(theme.muted))
                    .child(age(item.updated_at, now)),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(22.))
                    .child(source_mark(look, item.key.source.provider)),
            )
            .child(self.run_mark(row))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl(px(f32::from(row.depth) * 18.))
                    .child(div().flex_none().w(px(10.)).children(expander))
                    .child(
                        look.mono(item.identifier.clone())
                            .flex_none()
                            .text_color(rgb(theme.muted)),
                    )
                    .when_some(item.priority, |el, priority| {
                        el.child(look.badge(format!("P{priority}"), look.priority_tone(priority)))
                    })
                    .child(div().min_w_0().truncate().child(item.title.clone()))
                    .children(
                        item.labels
                            .iter()
                            .take(2)
                            .map(|label| look.chip(label.clone())),
                    )
                    .when(!item.blocked_by.is_empty(), |el| {
                        el.child(look.icon("icons/lock.svg", 11., look.hue(YELLOW)))
                    }),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(110.))
                    .truncate()
                    .text_size(look.small())
                    .text_color(rgb(theme.subtext()))
                    .child(item.author.clone().unwrap_or_default()),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(34.))
                    .text_size(look.small())
                    .text_color(rgb(theme.muted))
                    .child(
                        item.activity
                            .and_then(|activity| activity.comments)
                            .filter(|count| *count > 0)
                            .map(|count| count.to_string())
                            .unwrap_or_default(),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(86.))
                    .flex()
                    .child(look.badge(item.state.replace('_', " "), look.state_tone(&item.state))),
            )
            .into_any_element()
    }

    fn render_pull_request_row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.pull_request_rows.get(index) else {
            return div().h(px(self.row_height())).into_any_element();
        };
        let look = &self.look;
        let theme = &look.theme;
        let item = &self.snapshot.items[row.item];
        let Some(pr) = &item.pull_request else {
            return div().h(px(self.row_height())).into_any_element();
        };
        let fork = pr
            .head_repository
            .as_deref()
            .is_some_and(|head| head != item.key.source.repository);
        let activity = item.activity.unwrap_or_default();
        self.row_shell(("orchestrator-pr", index), item.key.canonical(), cx)
            .child(
                look.mono(item.identifier.clone())
                    .flex_none()
                    .w(px(44.))
                    .text_color(rgb(theme.muted)),
            )
            .child(self.run_mark(row))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().truncate().child(item.title.clone()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_size(look.tiny())
                            .text_color(rgb(theme.muted))
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(rgb(theme.subtext()))
                                    .child(item.author.clone().unwrap_or_default()),
                            )
                            .child("\u{00b7}")
                            .child(look.icon("icons/git-branch.svg", 10., theme.muted))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_font(&look.mono)
                                    .child(pr.head_ref.clone()),
                            )
                            .when(fork, |el| el.child(look.badge("fork", Tone::Accent))),
                    ),
            )
            .child(diff_squares(look, pr.additions, pr.deletions))
            .child(
                div()
                    .flex_none()
                    .w(px(70.))
                    .text_size(look.tiny())
                    .text_color(rgb(theme.muted))
                    .child(format!(
                        "\u{1f4ac}{} \u{2387}{}",
                        activity.comments.unwrap_or(0),
                        activity.commits.unwrap_or(0)
                    )),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(60.))
                    .flex()
                    .child(look.badge(item.state.clone(), look.state_tone(&item.state))),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(28.))
                    .text_size(look.small())
                    .text_color(rgb(theme.muted))
                    .child(age(item.updated_at, chrono::Utc::now())),
            )
            .into_any_element()
    }

    fn render_run_line(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let look = &self.look;
        let theme = &look.theme;
        let Some(line) = self.run_lines.get(index) else {
            return div().h(px(self.row_height())).into_any_element();
        };
        let row = match line {
            RunLine::Group { group, count } => {
                // Each group in the state its runs share, as the sidebar
                // draws it: waiting red, working yellow, idle hollow green.
                let status = match group {
                    Group::Attention => AgentStatus::Blocked,
                    Group::Working => AgentStatus::Working,
                    Group::Idle => AgentStatus::Idle,
                    Group::Finished => AgentStatus::Done,
                };
                let view = look.view();
                return div()
                    .w_full()
                    .h(px(self.row_height()))
                    .flex()
                    .items_end()
                    .gap(view.at(8.))
                    .px(view.at(10.))
                    .pb(view.at(6.))
                    .child(
                        view.status_dot(look.indicators.color(status), status == AgentStatus::Idle),
                    )
                    .child(look.label(group.label()))
                    .child(look.muted(count.to_string()))
                    .into_any_element();
            }
            RunLine::Run(row) => *row,
        };
        let run = &self.snapshot.runs[row.run];
        let item = self.item(&run.item_key);
        let host: SharedString = run
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.host.clone())
            .unwrap_or_else(|| "local".into())
            .into();
        let run_index = row.run;
        self.row_shell(("orchestrator-run", index), run.id.clone(), cx)
            .child(look.icon(agent_icon(&run.agent), 15., theme.foreground))
            .child(
                look.mono(item.map_or_else(
                    || super::rows::key_label(&run.item_key),
                    |item| item.identifier.clone(),
                ))
                .flex_none()
                .w(px(80.))
                .text_color(rgb(theme.muted)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .when(item.is_none(), |el| el.text_color(rgb(theme.muted)))
                    .child(item.map_or_else(
                        || "Not listed (closed or filtered)".to_owned(),
                        |item| item.title.clone(),
                    )),
            )
            .child(
                div()
                    .debug_selector(move || format!("orchestrator-run-host-column-{index}"))
                    .flex_none()
                    .w(px(100.))
                    .flex()
                    .child(
                        look.chip(host.clone())
                            .id(("orchestrator-run-host", index))
                            .debug_selector(move || format!("orchestrator-run-host-{index}"))
                            .max_w_full()
                            .min_w_0()
                            .truncate()
                            .tooltip(look.view().menu().tooltip(host)),
                    ),
            )
            .child(
                look.mono(
                    run.workspace
                        .as_ref()
                        .map(|workspace| workspace.branch.clone())
                        .unwrap_or_default(),
                )
                .debug_selector(move || format!("orchestrator-run-branch-{index}"))
                .flex_none()
                .w(px(220.))
                .truncate()
                .text_color(rgb(theme.subtext())),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(100.))
                    .flex()
                    .items_center()
                    .child(look.badge(row.status.label(), look.status_tone(row.status))),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(36.))
                    .text_size(look.small())
                    .text_color(rgb(theme.muted))
                    .child(age(run.started(), chrono::Utc::now())),
            )
            .child(div().flex_none().w(px(100.)).flex().child(look.badge(
                run.owner.as_str(),
                if run.owner == Owner::HerdrGpui {
                    Tone::Accent
                } else {
                    Tone::Neutral
                },
            )))
            .child(
                look.icon_button(("orchestrator-run-open", index), "icons/arrow-right.svg")
                    .debug_selector(move || format!("orchestrator-run-open-{index}"))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.open_run(run_index, cx);
                        }),
                    ),
            )
            .into_any_element()
    }

    /// The agent and status of an item's newest run.
    fn run_mark(&self, row: &ItemRow) -> Div {
        let look = &self.look;
        div()
            .flex_none()
            .w(px(30.))
            .flex()
            .items_center()
            .gap_1()
            .when_some(row.run, |el, run| {
                let agent = &self.snapshot.runs[run.run].agent;
                el.child(look.icon(agent_icon(agent), 13., look.theme.foreground))
                    .child(look.dot(look.status_hue(run.status), 7.))
            })
    }
}

pub(super) fn source_mark(look: &Look, provider: Provider) -> AnyElement {
    match provider {
        Provider::Github => look
            .icon("icons/github.svg", 13., look.theme.subtext())
            .into_any_element(),
        Provider::Beads => look.badge("bd", Tone::Merged).into_any_element(),
        Provider::Gitlab => look.badge("gl", Tone::Danger).into_any_element(),
    }
}

/// The list's column headings, in the lists' style, lined up with the
/// rows' cells.
fn column_head(look: &Look, cells: &[(&str, Option<f32>)]) -> Div {
    look.view()
        .columns("orchestrator-columns")
        .children(cells.iter().map(|(text, width)| {
            match width {
                Some(width) => div()
                    .flex_none()
                    .w(px(*width))
                    .whitespace_nowrap()
                    .child(SharedString::from(text.to_string())),
                None => div().flex_1().child(SharedString::from(text.to_string())),
            }
        }))
}

/// Five squares, green for added lines and red for removed, in proportion.
pub(super) fn diff_squares(look: &Look, additions: Option<u64>, deletions: Option<u64>) -> Div {
    let (added, removed) = (additions.unwrap_or(0), deletions.unwrap_or(0));
    let total = added + removed;
    let green = (added * 5 + total / 2)
        .checked_div(total)
        .map_or(0, |green| green.min(5));
    let known = additions.is_some() || deletions.is_some();
    div()
        .flex_none()
        .w(px(44.))
        .flex()
        .items_center()
        .gap(px(1.))
        .children((0..5).map(move |index| {
            let color = if !known {
                look.theme.active
            } else if index < green {
                look.hue(GREEN)
            } else {
                look.hue(RED)
            };
            div().size(px(7.)).rounded(px(1.)).bg(rgb(color))
        }))
}
