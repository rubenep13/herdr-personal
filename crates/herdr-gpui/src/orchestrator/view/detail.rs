//! One item's page: its description (Markdown, drawn as text), its runs, and
//! its raw details, with the overview in a column beside the description.

use super::{
    Detail, DetailTab, Event, OrchestratorView,
    dispatch::Confirm,
    list::source_mark,
    look::{age, agent_icon},
    rows::Tab,
    rows::{RunRow, Runs},
};
use crate::{
    design::{ButtonKind, ButtonSize, Edge, Tone},
    orchestrator::{Access, Item, Owner},
    release_notes,
};
use gpui::{prelude::*, *};

/// The page's measures at the orchestrator's face of 12: the title's size
/// over the face's and a tab's count.
const TITLE_OVER: f32 = 4.;
const COUNT_TEXT: f32 = 11.;
/// The overview column, its keys, the details' keys and a run's message
/// field at its narrowest.
const SIDE_WIDTH: f32 = 300.;
const KEY_WIDTH: f32 = 84.;
const DETAILS_KEY_WIDTH: f32 = 140.;
const MESSAGE_MIN: f32 = 160.;

impl OrchestratorView {
    pub(super) fn render_detail(
        &mut self,
        detail: Detail,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let typing = self.message.read(cx).focus.is_focused(window);
        let Some(item) = self.item(&detail.key).cloned() else {
            self.detail = None;
            return div().into_any_element();
        };
        let runs = Runs::new(&self.snapshot.runs, &self.snapshot.sessions, &self.live);
        let item_runs = runs.of_item(&detail.key);
        let body = match detail.tab {
            DetailTab::Description => div()
                .flex_1()
                .min_h_0()
                .flex()
                .child(self.render_description(&item))
                .child(self.render_overview(&item, &runs, &item_runs, cx))
                .into_any_element(),
            DetailTab::Agent => self.render_runs(&runs, &item_runs, typing, cx),
            DetailTab::Details => self.render_details(&item, cx),
            DetailTab::Conversation => self.render_conversation(cx),
            DetailTab::Checks => self.render_checks(&item),
        };
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(self.render_detail_header(&item, detail.tab, item_runs.len(), cx))
            .child(body)
            .into_any_element()
    }

    fn render_detail_header(
        &self,
        item: &Item,
        active: DetailTab,
        runs: usize,
        cx: &mut Context<Self>,
    ) -> Div {
        let look = &self.look;
        let theme = &look.theme;
        let (list, list_icon, list_tab) = match item.pull_request {
            Some(_) => ("Pull requests", "icons/git-branch.svg", Tab::PullRequests),
            None => ("Issues", "icons/note.svg", Tab::Issues),
        };
        let repo = self
            .snapshot
            .repo
            .as_ref()
            .and_then(|info| info.remote.as_ref())
            .map(|remote| remote.source.repository.clone())
            .unwrap_or_else(|| item.key.source.repository.clone());
        let key = item.key.canonical();
        let url = item.url.clone();
        let writable = self.snapshot.access == Some(Access::ReadWrite);
        let pull_request = item.pull_request.is_some();
        let review_key = item.key.canonical();
        let view = look.view();
        let menu = view.menu();
        let mergeable = self.mergeable();
        let merge = view
            .settings()
            .button(
                "orchestrator-detail-merge",
                "orchestrator-detail-merge".to_owned(),
                div()
                    .flex()
                    .items_center()
                    .gap(view.at(6.))
                    .child("Merge")
                    .child(
                        svg()
                            .path("icons/chevron-down.svg")
                            .size(view.at(13.))
                            .flex_none()
                            .text_color(view.title()),
                    ),
                ButtonKind::Normal,
                ButtonSize::Small,
                true,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.merge_open = !this.merge_open;
                cx.notify();
            }));
        let bar = view
            .bar("orchestrator-detail-bar")
            .pl(view.at(8.))
            .child(self.breadcrumbs(&repo, list, list_icon, list_tab, item, cx))
            .child(div().flex_1())
            .when(mergeable, |el| el.child(merge))
            .when(writable && pull_request, |el| {
                el.child(
                    look.button("orchestrator-detail-review", "Review with agent", true)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_review(review_key.clone(), window, cx);
                        })),
                )
            })
            .when(writable && !pull_request, |el| {
                el.child(
                    look.button("orchestrator-detail-dispatch", "Dispatch agent", runs == 0)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_dispatch(key.clone(), window, cx);
                        })),
                )
            })
            .when_some(url, |el, url| {
                el.child(
                    look.icon_button("orchestrator-detail-web", "icons/external.svg")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.emit(Event::OpenUrl(url.clone()));
                        })),
                )
            });
        let heading = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(view.at(8.))
            .px(view.at(16.))
            .pt(view.at(14.))
            .pb(view.at(10.))
            .child(
                div()
                    .debug_selector(|| "orchestrator-detail-title".into())
                    .min_w_0()
                    .truncate()
                    .text_size(px(look.ui.size + TITLE_OVER))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(view.title())
                    .child(item.title.clone()),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(view.at(6.))
                    .child(
                        look.mono(item.identifier.clone())
                            .flex_none()
                            .text_color(view.muted()),
                    )
                    .child(source_mark(look, item.key.source.provider))
                    .child(look.badge(item.state.replace('_', " "), look.state_tone(&item.state)))
                    .when_some(item.priority, |el, priority| {
                        el.child(look.badge(format!("P{priority}"), look.priority_tone(priority)))
                    })
                    .when_some(item.pull_request.as_ref(), |el, pr| {
                        el.child(look.mono(pr.head_ref.clone()).text_color(view.text()))
                            .child(look.icon("icons/arrow-right.svg", 11., theme.muted))
                            .child(look.mono(pr.base_ref.clone()).text_color(view.text()))
                    })
                    .when_some(item.parent_id.clone(), |el, parent| {
                        el.child(look.muted(format!("child of {parent}")))
                    }),
            );
        let tabs = menu
            .tabs("orchestrator-detail-tabs")
            .px(view.at(12.))
            .children(DetailTab::tabs(pull_request).iter().copied().map(|tab| {
                let selector = format!("orchestrator-detail-{}", tab.label());
                menu.tab(
                    SharedString::from(selector.clone()),
                    selector,
                    "orchestrator-detail-tab-underline",
                    tab == active,
                )
                .gap(view.at(6.))
                .child(tab.label())
                .when(tab == DetailTab::Agent && runs > 0, |el| {
                    el.child(
                        div()
                            .text_size(view.at(COUNT_TEXT))
                            .text_color(view.muted())
                            .child(runs.to_string()),
                    )
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(detail) = &mut this.detail {
                        detail.tab = tab;
                    }
                    this.merge_open = false;
                    this.follow_pull_request();
                    cx.notify();
                }))
            }));
        div()
            .flex_none()
            .flex()
            .flex_col()
            .child(bar)
            .child(heading)
            .child(tabs)
    }

    /// Back, then where the page sits: the repository, the list it came
    /// from, and the item itself. Every step but the last goes back there.
    fn breadcrumbs(
        &self,
        repo: &str,
        list: &'static str,
        list_icon: &'static str,
        list_tab: Tab,
        item: &Item,
        cx: &mut Context<Self>,
    ) -> Div {
        let look = &self.look;
        let view = look.view();
        let separator = || div().flex_none().text_color(view.muted()).child("/");
        let step = |id: &'static str, icon: &'static str, text: String| {
            let hover = view.title();
            div()
                .id(id)
                .debug_selector(move || id.into())
                .flex_none()
                .flex()
                .items_center()
                .gap(view.at(5.))
                .px(view.at(4.))
                .rounded(view.at(6.))
                .cursor_pointer()
                .text_color(view.muted())
                .hover(move |style| style.text_color(hover))
                .child(look.icon(icon, 13., look.theme.muted))
                .child(text)
        };
        div()
            .min_w_0()
            .flex_shrink(1.)
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(view.at(6.))
            .text_size(px(view.scale().name - 0.5))
            .child(
                look.icon_button("orchestrator-back", "icons/arrow-left.svg")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.detail = None;
                        cx.notify();
                    })),
            )
            .child(
                step(
                    "orchestrator-crumb-repo",
                    "icons/github.svg",
                    repo.to_owned(),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.detail = None;
                    this.select_tab(Tab::Issues, cx);
                })),
            )
            .child(separator())
            .child(
                step("orchestrator-crumb-list", list_icon, list.to_owned()).on_click(cx.listener(
                    move |this, _, _, cx| {
                        this.detail = None;
                        this.select_tab(list_tab, cx);
                    },
                )),
            )
            .child(separator())
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(view.at(5.))
                    .text_size(px(view.scale().name))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(view.title())
                    .child(source_mark(look, item.key.source.provider))
                    .child(div().min_w_0().truncate().child(item.identifier.clone())),
            )
    }

    fn render_description(&self, item: &Item) -> AnyElement {
        let look = &self.look;
        let body = item.description.as_deref().unwrap_or("");
        let content = if body.trim().is_empty() {
            look.muted("No description.").into_any_element()
        } else {
            let lines = self.description.lines(body);
            release_notes::render("orchestrator-description", &lines, &look.theme, &look.mono)
                .into_any_element()
        };
        div()
            .id("orchestrator-description")
            .flex_1()
            .min_w_0()
            .overflow_y_scroll()
            .p(look.view().at(16.))
            .child(content)
            .into_any_element()
    }

    fn render_overview(
        &self,
        item: &Item,
        runs: &Runs<'_>,
        item_runs: &[RunRow],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let look = &self.look;
        let view = look.view();
        let row = |key: &'static str, value: AnyElement| {
            div()
                .flex()
                .items_center()
                .gap(view.at(8.))
                .text_size(px(view.scale().secondary))
                .child(
                    div()
                        .flex_none()
                        .w(view.at(KEY_WIDTH))
                        .text_color(view.muted())
                        .child(key),
                )
                .child(div().flex_1().min_w_0().flex().child(value))
        };
        let text = |value: String| div().min_w_0().truncate().child(value).into_any_element();
        let now = chrono::Utc::now();
        let source = match item.key.source.provider {
            crate::orchestrator::Provider::Github => {
                format!("GitHub \u{00b7} {}", item.key.source.repository)
            }
            crate::orchestrator::Provider::Beads => "Beads".to_owned(),
            crate::orchestrator::Provider::Gitlab => {
                format!("GitLab \u{00b7} {}", item.key.source.repository)
            }
        };
        view.side_panel("orchestrator-detail-side", Edge::Right)
            .id("orchestrator-overview")
            .w(view.at(SIDE_WIDTH))
            .overflow_y_scroll()
            .p(view.at(16.))
            .gap(view.at(10.))
            .child(look.label("OVERVIEW"))
            .child(row("Source", text(source)))
            .child(row(
                "Status",
                look.badge(item.state.replace('_', " "), look.state_tone(&item.state))
                    .into_any_element(),
            ))
            .when_some(item.priority, |el, priority| {
                el.child(row(
                    "Priority",
                    look.badge(format!("P{priority}"), look.priority_tone(priority))
                        .into_any_element(),
                ))
            })
            .when_some(item.parent_id.clone(), |el, parent| {
                el.child(row("Parent", look.mono(parent).into_any_element()))
            })
            .when(!item.blocked_by.is_empty(), |el| {
                el.child(row(
                    "Blocked by",
                    look.mono(item.blocked_by.join(", ")).into_any_element(),
                ))
            })
            .when_some(item.author.clone(), |el, author| {
                el.child(row("Author", text(author)))
            })
            .child(row(
                "Updated",
                text(format!("{} ago", age(item.updated_at, now))),
            ))
            .when(!item.labels.is_empty(), |el| {
                el.child(row(
                    "Labels",
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .children(item.labels.iter().map(|label| look.chip(label.clone())))
                        .into_any_element(),
                ))
            })
            .when_some(item_runs.first().copied(), |el, latest| {
                el.child(div().h(view.at(6.)))
                    .child(look.label("LATEST RUN"))
                    .child(self.run_card_small(runs, latest, cx))
            })
            .into_any_element()
    }

    fn render_runs(
        &self,
        runs: &Runs<'_>,
        item_runs: &[RunRow],
        typing: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let look = &self.look;
        let view = look.view();
        let mut column = div()
            .id("orchestrator-runs")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p(view.at(16.))
            .flex()
            .flex_col()
            .gap(view.at(12.));
        if item_runs.is_empty() {
            column = column.child(look.muted("No agent has worked on this yet."));
        }
        for (position, row) in item_runs.iter().enumerate() {
            if position == 1 {
                column = column.child(look.label("EARLIER RUNS"));
            }
            column = column.child(self.run_card(runs, *row, position == 0, typing, cx));
        }
        column
            .child(
                div()
                    .text_size(px(view.scale().secondary))
                    .text_color(view.muted())
                    .child("Status comes from each host's Herdr snapshot. Runs started by agent-launcher are shown here; it alone updates them."),
            )
            .into_any_element()
    }

    fn run_card(
        &self,
        runs: &Runs<'_>,
        row: RunRow,
        current: bool,
        typing: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let look = &self.look;
        let view = look.view();
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
        let line = live
            .and_then(|agent| agent.title.clone())
            .or_else(|| run.message.clone());
        let index = row.run;
        view.settings()
            .card(format!("orchestrator-card-{index}"))
            .px(view.at(14.))
            .py(view.at(12.))
            .gap(view.at(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(view.at(8.))
                    .child(view.dialog().tile(
                        format!("orchestrator-card-tile-{index}"),
                        agent_icon(&run.agent),
                        view.title(),
                    ))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(view.title())
                            .child(run.agent.clone()),
                    )
                    .child(look.chip(host))
                    .child(look.badge(
                        run.owner.as_str(),
                        if run.owner == Owner::HerdrGpui {
                            Tone::Accent
                        } else {
                            Tone::Neutral
                        },
                    ))
                    .child(div().flex_1())
                    .child(look.badge(row.status.label(), look.status_tone(row.status)))
                    .child(look.muted(age(run.started(), chrono::Utc::now()))),
            )
            .when(!branch.is_empty(), |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(view.at(6.))
                        .child(look.icon("icons/git-branch.svg", 11., look.theme.muted))
                        .child(look.mono(branch).text_color(view.text())),
                )
            })
            .when_some(line, |el, line| {
                el.child(
                    view.dialog()
                        .well(format!("orchestrator-card-line-{index}"), Some(&look.mono))
                        .text_size(look.small())
                        .text_color(view.text())
                        .truncate()
                        .child(line),
                )
            })
            .child(self.run_controls(run, index, current, live.is_some(), typing, cx))
    }

    fn render_details(&self, item: &Item, cx: &mut Context<Self>) -> AnyElement {
        let look = &self.look;
        let view = look.view();
        let mut rows: Vec<(&'static str, String)> = vec![
            ("Key", item.key.canonical()),
            ("Identifier", item.identifier.clone()),
            ("State", item.state.clone()),
        ];
        if let Some(url) = &item.url {
            rows.push(("URL", url.clone()));
        }
        if let Some(pr) = &item.pull_request {
            rows.push(("Base", format!("{} @ {}", pr.base_ref, pr.base_sha)));
            rows.push(("Head", format!("{} @ {}", pr.head_ref, pr.head_sha)));
            if let Some(head) = &pr.head_repository {
                rows.push(("Head repository", head.clone()));
            }
        }
        if let Some(activity) = item.activity {
            let count =
                |value: Option<u64>| value.map_or_else(|| "\u{2014}".to_owned(), |v| v.to_string());
            rows.push(("Comments", count(activity.comments)));
            rows.push(("Review comments", count(activity.review_comments)));
            rows.push(("Commits", count(activity.commits)));
        }
        let time = |value: Option<chrono::DateTime<chrono::Utc>>| {
            value.map_or_else(|| "\u{2014}".to_owned(), |value| value.to_rfc3339())
        };
        let deletable = item.key.source.provider == crate::orchestrator::Provider::Beads
            && self.snapshot.access == Some(Access::ReadWrite);
        let (source, id) = (item.key.source.clone(), item.key.native_id.clone());
        rows.push(("Created", time(item.created_at)));
        rows.push(("Updated", time(item.updated_at)));
        div()
            .id("orchestrator-details")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p(view.at(16.))
            .flex()
            .flex_col()
            .gap(view.at(6.))
            .children(rows.into_iter().map(|(key, value)| {
                div()
                    .flex()
                    .gap(view.at(12.))
                    .child(
                        div()
                            .flex_none()
                            .w(view.at(DETAILS_KEY_WIDTH))
                            .text_size(px(view.scale().secondary))
                            .text_color(view.muted())
                            .child(key),
                    )
                    .child(look.mono(value).min_w_0())
            }))
            .when(deletable, |el| {
                el.child(div().h(view.at(12.))).child(
                    div().flex().child(
                        look.button("orchestrator-delete-bead", "Delete bead\u{2026}", false)
                            .text_color(view.settings().danger())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.ask(
                                    Confirm::DeleteBead {
                                        source: source.clone(),
                                        id: id.clone(),
                                    },
                                    cx,
                                );
                            })),
                    ),
                )
            })
            .into_any_element()
    }

    /// A run card's buttons: open its pane while Herdr shows it; for the
    /// newest run herdr-gpui owns, a message field, Send, and Stop; Remove
    /// for any run herdr-gpui owns that has a worktree or failed.
    fn run_controls(
        &self,
        run: &crate::orchestrator::Run,
        index: usize,
        current: bool,
        live: bool,
        typing: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let look = &self.look;
        let view = look.view();
        let own = run.owner == Owner::HerdrGpui;
        let writable = self.snapshot.access == Some(Access::ReadWrite);
        let branch = run
            .workspace
            .as_ref()
            .map(|workspace| workspace.branch.clone())
            .unwrap_or_default();
        let (stop_run, remove_run, agent) = (run.id.clone(), run.id.clone(), run.agent.clone());
        // A failed run with no worktree is still forgotten through Remove.
        let removable =
            run.workspace.is_some() || run.state == crate::orchestrator::RunState::Failed;
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(view.at(6.))
            .when(live, |el| {
                el.child(
                    look.button(("orchestrator-card-open", index), "Open pane", true)
                        .on_click(cx.listener(move |this, _, _, cx| this.open_run(index, cx))),
                )
            })
            .when(own && writable && current && live, |el| {
                el.child(
                    view.field("orchestrator-card-message".into(), typing, None)
                        .flex_1()
                        .min_w(view.at(MESSAGE_MIN))
                        .child(div().flex_1().min_w_0().child(self.message.clone())),
                )
                .child(
                    look.button(("orchestrator-card-send", index), "Send", false)
                        .on_click(cx.listener(|this, _, _, cx| this.send_message(cx))),
                )
                .child(
                    look.button(("orchestrator-card-stop", index), "Stop", false)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.ask(
                                Confirm::Stop {
                                    run: stop_run.clone(),
                                    agent: agent.clone(),
                                },
                                cx,
                            );
                        })),
                )
            })
            .when(own && writable && removable, |el| {
                el.child(
                    look.icon_button(("orchestrator-card-remove", index), "icons/trash.svg")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.ask(
                                Confirm::Remove {
                                    run: remove_run.clone(),
                                    branch: branch.clone(),
                                },
                                cx,
                            );
                        })),
                )
            })
    }
}
