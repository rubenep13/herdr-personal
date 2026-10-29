//! The preview beside the list, or under it in a narrow inbox: the selected
//! item or run at a glance, with the actions it offers. Beside the list, its
//! left edge drags to resize it, or shut.

use super::{
    Event, OrchestratorView, PreviewDrag, STACKED_PREVIEW_SHARE,
    list::{diff_squares, source_mark},
    look::{GREEN, RED, age, agent_icon},
    rows::{RunRow, Runs, Tab},
};
use crate::{
    design::Edge,
    orchestrator::{Access, Item},
    release_notes,
};
use gpui::{prelude::*, *};

/// How many lines of a description the preview draws; the item's page has
/// the rest.
const PREVIEW_LINES: usize = 40;
/// The grabbable strip along the panel's left edge, at the face's size of 12.
const HANDLE: f32 = 6.;

impl OrchestratorView {
    pub(super) fn render_preview_pane(&self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = &self.look;
        let view = look.view();
        let body = match self.tab {
            Tab::Runs => self
                .selected
                .as_deref()
                .and_then(|id| self.snapshot.runs.iter().position(|run| run.id == id))
                .map(|run| self.render_run_preview(run, cx)),
            _ => self
                .selected
                .as_deref()
                .and_then(|key| self.item(key))
                .map(|item| self.render_item_preview(item, cx)),
        };
        let stacked = self.preview_stacked();
        view.side_panel("orchestrator-preview-pane", Edge::Right)
            .relative()
            .map(|panel| {
                if stacked {
                    panel
                        .border_l_0()
                        .border_t_1()
                        .w_full()
                        .h(relative(STACKED_PREVIEW_SHARE))
                } else {
                    panel.w(px(self.preview_width)).h_full()
                }
            })
            .child(
                div()
                    .id("orchestrator-preview-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(view.at(16.))
                    .py(view.at(14.))
                    .flex()
                    .flex_col()
                    .gap(view.at(10.))
                    .children(body),
            )
            // Its left edge drags: a strip over the hairline, wide enough
            // to grab. Under the list it keeps the width it was given.
            .when(!stacked, |panel| {
                panel.child(
                    div()
                        .id("orchestrator-preview-handle")
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .w(view.at(HANDLE))
                        .cursor(CursorStyle::ResizeLeftRight)
                        .on_drag(PreviewDrag, |drag, _, _, cx| cx.new(|_| *drag)),
                )
            })
            .into_any_element()
    }

    fn render_item_preview(&self, item: &Item, cx: &mut Context<Self>) -> AnyElement {
        let look = &self.look;
        let theme = &look.theme;
        let key = item.key.canonical();
        let runs = Runs::new(&self.snapshot.runs, &self.snapshot.sessions, &self.live);
        let latest = runs.of_item(&key).into_iter().next();
        let now = chrono::Utc::now();
        let comments = item
            .activity
            .and_then(|activity| activity.comments)
            .unwrap_or(0);
        let meta = format!(
            "{} \u{00b7} updated {} ago{}",
            item.author.as_deref().unwrap_or("unknown"),
            age(item.updated_at, now),
            if comments > 0 {
                format!(" \u{00b7} {comments} comments")
            } else {
                String::new()
            }
        );
        let lines = item
            .description
            .as_deref()
            .filter(|text| !text.trim().is_empty())
            .map(|text| self.preview_markdown.lines(text));
        let writable = self.snapshot.access == Some(Access::ReadWrite);
        let url = item.url.clone();
        let open_key = key.clone();
        let dispatch_key = key.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(source_mark(look, item.key.source.provider))
                    .child(
                        look.mono(item.identifier.clone())
                            .text_color(rgb(theme.muted)),
                    )
                    .child(look.badge(item.state.replace('_', " "), look.state_tone(&item.state)))
                    .when_some(item.priority, |el, priority| {
                        el.child(look.badge(format!("P{priority}"), look.priority_tone(priority)))
                    })
                    .children(
                        item.labels
                            .iter()
                            .take(3)
                            .map(|label| look.chip(label.clone())),
                    ),
            )
            .child(
                div()
                    .text_size(px(look.view().scale().name + 1.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(look.view().title())
                    .child(item.title.clone()),
            )
            .child(look.muted(meta))
            .when_some(item.pull_request.as_ref(), |el, pr| {
                el.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_1()
                        .child(
                            look.mono(pr.head_ref.clone())
                                .text_color(rgb(theme.subtext())),
                        )
                        .child(look.icon("icons/arrow-right.svg", 11., theme.muted))
                        .child(
                            look.mono(pr.base_ref.clone())
                                .text_color(rgb(theme.subtext())),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(look.small())
                        .child(diff_squares(look, pr.additions, pr.deletions))
                        .child(
                            div()
                                .text_color(rgb(look.hue(GREEN)))
                                .child(format!("+{}", pr.additions.unwrap_or(0))),
                        )
                        .child(
                            div()
                                .text_color(rgb(look.hue(RED)))
                                .child(format!("\u{2212}{}", pr.deletions.unwrap_or(0))),
                        ),
                )
            })
            .when(!item.blocked_by.is_empty(), |el| {
                el.child(look.muted(format!("Blocked by {}", item.blocked_by.join(", "))))
            })
            .when_some(latest, |el, latest| {
                el.child(look.label("Latest run"))
                    .child(self.run_card_small(&runs, latest, cx))
            })
            .when_some(lines, |el, lines| {
                let shown = &lines[..lines.len().min(PREVIEW_LINES)];
                el.child(release_notes::render(
                    "orchestrator-preview-description",
                    shown,
                    theme,
                    &look.mono,
                ))
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .when(writable, |el| {
                        el.child(
                            look.button(
                                "orchestrator-preview-dispatch",
                                "Dispatch agent",
                                latest.is_none(),
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.open_dispatch(dispatch_key.clone(), window, cx);
                                },
                            )),
                        )
                    })
                    .child(
                        look.button("orchestrator-preview-open", "Details", false)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_detail(open_key.clone(), cx)
                            })),
                    )
                    .when_some(url, |el, url| {
                        el.child(
                            look.icon_button("orchestrator-preview-web", "icons/external.svg")
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    cx.emit(Event::OpenUrl(url.clone()));
                                })),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_run_preview(&self, run: usize, cx: &mut Context<Self>) -> AnyElement {
        let look = &self.look;
        let runs = Runs::new(&self.snapshot.runs, &self.snapshot.sessions, &self.live);
        let Some(status) = runs.status(run) else {
            return div().into_any_element();
        };
        let found = &self.snapshot.runs[run];
        let item = self.item(&found.item_key);
        let key = found.item_key.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(look.view().scale().name + 1.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(look.view().title())
                    .child(item.map_or_else(|| found.item_key.clone(), |item| item.title.clone())),
            )
            .child(self.run_card_small(&runs, RunRow { run, status }, cx))
            .when(item.is_some(), |el| {
                el.child(
                    look.button("orchestrator-run-item", "Open the item", false)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.tab = Tab::Issues;
                            this.open_detail(key.clone(), cx);
                        })),
                )
            })
            .into_any_element()
    }

    /// A run in a few lines: agent, host, branch, status, and the agent's
    /// own title while Herdr shows it, with Open when it can be reached.
    pub(super) fn run_card_small(
        &self,
        runs: &Runs<'_>,
        row: RunRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let look = &self.look;
        let theme = &look.theme;
        let run = &runs.runs[row.run];
        let live = runs.live(row.run);
        let host = run
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.host.clone())
            .unwrap_or_else(|| "local".into());
        let branch = run
            .workspace
            .as_ref()
            .map(|workspace| workspace.branch.clone())
            .unwrap_or_default();
        let title = live.and_then(|agent| agent.title.clone());
        let index = row.run;
        let view = look.view();
        view.settings()
            .card(format!("orchestrator-run-card-{index}"))
            .px(view.at(12.))
            .py(view.at(10.))
            .gap(view.at(4.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(view.at(10.))
                    .child(look.icon(agent_icon(&run.agent), 16., theme.foreground))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(view.title())
                            .child(format!("{} on {host}", run.agent)),
                    )
                    .child(look.badge(row.status.label(), look.status_tone(row.status))),
            )
            .when(!branch.is_empty(), |el| {
                el.child(look.mono(branch).text_color(view.muted()).truncate())
            })
            .when_some(title, |el, title| {
                el.child(
                    div()
                        .text_size(look.small())
                        .text_color(view.text())
                        .truncate()
                        .child(title),
                )
            })
            .when(live.is_some(), |el| {
                el.child(
                    div().pt(view.at(4.)).child(
                        look.button(("orchestrator-open-run", index), "Open agent", true)
                            .on_click(cx.listener(move |this, _, _, cx| this.open_run(index, cx))),
                    ),
                )
            })
            .into_any_element()
    }
}
