//! The inbox: the repository and its sources, the Issues / Pull requests /
//! Runs tabs, search and filters, and the list beside its preview. Lists are
//! virtualized, so thousands of beads cost only the rows on screen.

use super::{
    Event, OrchestratorView, PREVIEW_MAX_SHARE, PREVIEW_MIN, PreviewDrag,
    look::{Look, age},
    rows::{Runs, Sort, Tab},
};
use crate::{
    design::Tone,
    orchestrator::{Access, Provider, SyncState},
};
use gpui::{prelude::*, *};

impl OrchestratorView {
    pub(super) fn render_inbox(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let preview = self
            .preview_open
            .then(|| self.render_preview_pane(window, cx));
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(self.render_header(cx))
            .child(self.render_tabs(cx))
            .child(self.render_search_row(window, cx))
            .children(self.render_banners(cx))
            .child(
                div()
                    .id("orchestrator-body")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .on_drag_move(cx.listener(
                        |this, event: &DragMoveEvent<PreviewDrag>, window, cx| {
                            let right = event.bounds.right();
                            let width = f32::from(right - event.event.position.x);
                            let max = f32::from(window.viewport_size().width) * PREVIEW_MAX_SHARE;
                            // Dragged nearly shut, the preview closes.
                            if width < PREVIEW_MIN / 2. {
                                this.preview_open = false;
                            } else {
                                this.preview_width = width.clamp(PREVIEW_MIN, max.max(PREVIEW_MIN));
                            }
                            cx.notify();
                        },
                    ))
                    .child(self.render_list(cx))
                    .children(preview),
            )
            .when(self.sort_open, |el| el.child(self.render_sort_menu(cx)))
            .into_any_element()
    }

    fn render_header(&self, cx: &mut Context<Self>) -> Div {
        let look = &self.look;
        let view = look.view();
        let info = self.snapshot.repo.as_ref();
        let name = info
            .and_then(|info| info.remote.as_ref())
            .map(|remote| remote.source.repository.clone())
            .or_else(|| {
                info.map(|info| {
                    info.main_root
                        .rsplit('/')
                        .next()
                        .unwrap_or(&info.main_root)
                        .to_owned()
                })
            })
            .unwrap_or_else(|| "Reading the repository\u{2026}".into());
        let host = match &self.request.target {
            herdr_client::ConnectTarget::Ssh { target, .. } => target.clone(),
            _ => "local".into(),
        };
        let sources = self
            .snapshot
            .sources
            .iter()
            .enumerate()
            .map(|(index, status)| {
                let (text, tone, sign_in) = match &status.state {
                    SyncState::Syncing => ("syncing\u{2026}".to_owned(), Tone::Accent, false),
                    SyncState::Synced { count, at } => (
                        format!("{count} \u{00b7} {}", synced(*at)),
                        Tone::Good,
                        false,
                    ),
                    SyncState::Capped { count } => (format!("first {count}"), Tone::Warn, false),
                    SyncState::SignIn => ("sign in".to_owned(), Tone::Warn, true),
                    SyncState::Unsupported => {
                        ("not supported yet".to_owned(), Tone::Neutral, false)
                    }
                    SyncState::Failed(_) => ("failed".to_owned(), Tone::Danger, false),
                };
                let provider = provider_name(status.key.provider);
                view.settings()
                    .chip(
                        format!("orchestrator-source-{index}"),
                        format!("{provider} \u{00b7} {text}"),
                        tone,
                    )
                    .id(("orchestrator-source", index))
                    .when(sign_in, |el| {
                        el.cursor_pointer()
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Event::SignIn)))
                    })
            });
        let dispatchable = self.tab != Tab::Runs
            && self.selected.is_some()
            && self.snapshot.access == Some(Access::ReadWrite);
        view.bar("orchestrator-bar")
            .child(view.bar_title(Some("icons/inbox.svg"), name, Some(host.into())))
            .children(sources)
            .child(div().flex_1())
            .child(
                look.icon_button("orchestrator-refresh", "icons/refresh.svg")
                    .on_click(cx.listener(|this, _, _, _| this.refresh())),
            )
            .when(!self.detached, |el| {
                el.child(
                    look.icon_button("orchestrator-window", "icons/window-maximize.svg")
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(Event::OpenWindow))),
                )
            })
            .child(
                view.icon_button(
                    "orchestrator-preview",
                    "orchestrator-preview".to_owned(),
                    "icons/panel-right.svg",
                    self.preview_open,
                    true,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.preview_open = !this.preview_open;
                    cx.notify();
                })),
            )
            .when(dispatchable, |el| {
                el.child(
                    look.button("orchestrator-dispatch", "Dispatch agent", true)
                        .on_click(cx.listener(|this, _, window, cx| {
                            if let Some(item) = this.selected.clone() {
                                this.open_dispatch(item, window, cx);
                            }
                        })),
                )
            })
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> Div {
        let look = &self.look;
        let view = look.view();
        let menu = view.menu();
        let runs = Runs::new(&self.snapshot.runs, &self.snapshot.sessions, &self.live);
        let waiting = super::rows::attention(&runs);
        let counts = [
            self.issue_rows.len(),
            self.pull_request_rows.len(),
            self.snapshot.runs.len(),
        ];
        menu.tabs("orchestrator-tabs").px(view.at(12.)).children(
            Tab::ALL.into_iter().zip(counts).map(|(tab, count)| {
                let selector = format!("orchestrator-tab-{}", tab.label());
                menu.tab(
                    SharedString::from(selector.clone()),
                    selector,
                    "orchestrator-tab-underline",
                    tab == self.tab,
                )
                .gap(view.at(6.))
                .child(tab.label())
                .child(
                    div()
                        .text_size(view.at(COUNT_TEXT))
                        .text_color(view.muted())
                        .child(count.to_string()),
                )
                // A dot in the waiting colour while an agent waits.
                .when(tab == Tab::Runs && waiting > 0, |el| {
                    el.child(
                        view.status_dot(look.status_hue(super::rows::Status::NeedsInput), false)
                            .size(view.at(6.)),
                    )
                })
                .on_click(cx.listener(move |this, _, _, cx| this.select_tab(tab, cx)))
            }),
        )
    }

    fn render_search_row(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let look = &self.look;
        let view = look.view();
        let filters = self.filters;
        let mut row = view
            .filters("orchestrator-filters")
            .child(self.render_search_field(window, cx));
        let toggle = |id: &'static str, label: &'static str, on: bool, flip: fn(&mut Self)| {
            look.toggle(id, label, on)
                .on_click(cx.listener(move |this, _, _, cx| {
                    flip(this);
                    this.refresh_rows();
                    cx.notify();
                }))
        };
        match self.tab {
            Tab::Issues => {
                row = row
                    .child(toggle(
                        "orchestrator-beads",
                        "Beads",
                        filters.provider == Some(Provider::Beads),
                        |this| {
                            this.filters.provider = match this.filters.provider {
                                Some(Provider::Beads) => None,
                                _ => Some(Provider::Beads),
                            };
                        },
                    ))
                    .child(toggle(
                        "orchestrator-has-run",
                        "Has agent",
                        filters.has_run,
                        |this| {
                            this.filters.has_run = !this.filters.has_run;
                        },
                    ))
                    .child(toggle(
                        "orchestrator-blocked",
                        "Blocked",
                        filters.blocked,
                        |this| {
                            this.filters.blocked = !this.filters.blocked;
                        },
                    ));
            }
            Tab::PullRequests => {
                row = row
                    .child(toggle(
                        "orchestrator-open",
                        "Open",
                        filters.open_only,
                        |this| {
                            this.filters.open_only = !this.filters.open_only;
                        },
                    ))
                    .when(self.login.is_some(), |row| {
                        row.child(toggle("orchestrator-mine", "Mine", self.mine, |this| {
                            this.mine = !this.mine;
                        }))
                    })
                    .child(toggle(
                        "orchestrator-pr-has-run",
                        "Has agent",
                        filters.has_run,
                        |this| {
                            this.filters.has_run = !this.filters.has_run;
                        },
                    ));
            }
            Tab::Runs => {
                row = row.child(toggle(
                    "orchestrator-active",
                    "Active",
                    self.active_runs,
                    |this| {
                        this.active_runs = !this.active_runs;
                    },
                ));
            }
        }
        row.child(div().flex_1())
            .when(self.tab != Tab::Runs, |row| {
                row.child(
                    view.dropdown(
                        "orchestrator-sort",
                        "orchestrator-sort".to_owned(),
                        Some("icons/sort.svg"),
                        None,
                        self.sort.label(),
                    )
                    // In a narrow window the order's name gives way too.
                    .flex_shrink(1.)
                    .min_w_0()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.sort_open = !this.sort_open;
                        cx.notify();
                    })),
                )
            })
    }

    /// The Settings field, the magnifier inside: its ring takes the accent
    /// while it has the keyboard, and a clear button shows once there is
    /// text.
    fn render_search_field(&self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let look = &self.look;
        let view = look.view();
        let focused = self.search.read(cx).focus.is_focused(window);
        view.field(
            "orchestrator-search".to_owned(),
            focused,
            Some("icons/search.svg"),
        )
        .id("orchestrator-search")
        .flex_1()
        .flex_shrink(1.)
        .max_w(view.at(SEARCH_WIDTH))
        .child(div().flex_1().min_w_0().child(self.search.clone()))
        .when(!self.query.is_empty(), |field| {
            field.child(
                look.icon_button("orchestrator-search-clear", "icons/close.svg")
                    .size(view.at(CLEAR))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.search.update(cx, |search, cx| search.clear(cx));
                        window.focus(&this.search.read(cx).focus.clone(), cx);
                    })),
            )
        })
    }

    fn render_sort_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.look.view();
        deferred(
            view.menu_panel("orchestrator-sort-menu")
                .absolute()
                .top(view.at(SORT_TOP))
                .right(view.at(12.))
                .w(view.at(SORT_WIDTH))
                .children(Sort::ALL.into_iter().map(|sort| {
                    let selector = format!("orchestrator-sort-{}", sort.label());
                    view.row(
                        SharedString::from(selector.clone()),
                        selector.clone(),
                        false,
                        false,
                    )
                    .child(div().flex_1().min_w_0().truncate().child(sort.label()))
                    .when(sort == self.sort, |row| {
                        row.child(
                            svg()
                                .debug_selector(move || format!("{selector}-check"))
                                .path("icons/check.svg")
                                .size(view.at(16.))
                                .flex_none()
                                .text_color(view.pill_on().0),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.sort = sort;
                        this.sort_open = false;
                        this.refresh_rows();
                        cx.notify();
                    }))
                })),
        )
    }

    fn render_banners(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let look = &self.look;
        let mut banners = Vec::new();
        if let Some(Access::ReadOnly { found }) = self.snapshot.access {
            banners.push((
                false,
                format!(
                    "Shared data is schema {found}; this herdr-gpui understands up to {}. Read-only until updated.",
                    crate::orchestrator::SCHEMA_VERSION
                ),
            ));
        }
        if let Some(error) = &self.snapshot.error {
            banners.push((false, error.to_string()));
        }
        for status in &self.snapshot.sources {
            let text = match &status.state {
                SyncState::Failed(error) => {
                    format!("{}: {error}", provider_name(status.key.provider))
                }
                SyncState::Capped { count } => format!(
                    "{} has more than {count} items; the newest are shown and not cached.",
                    provider_name(status.key.provider)
                ),
                _ => continue,
            };
            banners.push((matches!(status.state, SyncState::Capped { .. }), text));
        }
        banners
            .into_iter()
            .enumerate()
            .map(|(index, (warning, text))| {
                banner(look, warning, format!("orchestrator-banner-{index}"), text)
                    .id(("orchestrator-banner", index))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, _| this.refresh()))
                    .into_any_element()
            })
            .collect()
    }
}

/// The inbox's measures at the orchestrator's face of 12: a tab's count,
/// the search field's widest and its clear button, and the sort menu's
/// place under the bar, tabs and filters, and its width.
const COUNT_TEXT: f32 = 11.;
const SEARCH_WIDTH: f32 = 380.;
const CLEAR: f32 = 20.;
const SORT_TOP: f32 = 40. + 40. + 46. - 4.;
const SORT_WIDTH: f32 = 200.;

/// "synced now", "synced 4m ago": when a source last finished a sync.
fn synced(at: chrono::DateTime<chrono::Utc>) -> String {
    match age(Some(at), chrono::Utc::now()).as_str() {
        "now" => "synced now".into(),
        ago => format!("synced {ago} ago"),
    }
}

fn provider_name(provider: Provider) -> &'static str {
    match provider {
        Provider::Github => "GitHub",
        Provider::Beads => "Beads",
        Provider::Gitlab => "GitLab",
    }
}

/// A notice over the list, in full: the dialogs' error box, or their
/// warning box when `warning`.
pub(super) fn banner(look: &Look, warning: bool, selector: String, text: String) -> Div {
    let view = look.view();
    let dialog = view.dialog();
    div()
        .flex_none()
        .mx(view.at(12.))
        .mb(view.at(8.))
        .child(if warning {
            dialog.warning(selector, text)
        } else {
            dialog.error(selector, text)
        })
}
