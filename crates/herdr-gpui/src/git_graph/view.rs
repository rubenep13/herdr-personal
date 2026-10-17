//! A Git graph tab's view: it asks its worker for the history and draws what
//! comes back. It knows nothing of tabs; the window feeds it from its tick.

mod chips;
mod header;
mod keys;
mod lanes;
mod rows;

#[cfg(test)]
pub(crate) use chips::chip_icons;

use super::{
    checkout::GraphCheckout,
    error::Error,
    geometry::{ROW_H, graph_width},
    history::History,
    prefs::{GraphPrefs, GraphPrefsStore},
    worker::{Reply, Request, Worker},
};
use crate::{
    config::{FontConfig, Theme},
    fonts::StyledFont,
    teleport::Host,
};
use gpui::{prelude::*, *};
use header::ColumnDrag;
use std::{collections::HashMap, sync::Arc};

/// Rows from the end at which the next page is asked for.
const MORE_WITHIN: usize = 50;

/// Whether the host the repository is on can be asked anything now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Connection {
    Online,
    Offline,
}

/// What the view draws with; the window replaces it when settings change.
#[derive(Clone, Debug)]
pub(crate) struct GraphLook {
    pub(crate) theme: Theme,
    pub(crate) ui: FontConfig,
    pub(crate) mono: FontConfig,
}

pub(crate) struct GitGraphView {
    checkout: GraphCheckout,
    pub(crate) focus_handle: FocusHandle,
    look: GraphLook,
    /// The columns and widths every graph tab shares, kept in step with the
    /// app's store.
    prefs: GraphPrefs,
    _prefs: Subscription,
    scroll: UniformListScrollHandle,
    /// The selected commit, by oid, so it survives a reload.
    selected: Option<String>,
    /// Whether the gear's menu is open.
    column_menu: bool,
    /// `None` when the host cannot run scripts; `error` says why.
    worker: Option<Worker>,
    /// The host the worker reads on, for tests.
    #[cfg(test)]
    host: Option<Host>,
    /// The SSH host's name, for messages; `None` for this machine.
    remote: Option<String>,
    connection: Connection,
    /// The generation and kind of the last request; older replies are stale.
    latest: (u64, Request),
    waiting: bool,
    history: Option<Arc<History>>,
    error: Option<Arc<Error>>,
    /// Shaped label widths, by label; cleared when the font changes.
    chip_widths: HashMap<SharedString, f32>,
    /// The label the pointer is over, shown whole above the others.
    hovered_chip: Option<(usize, usize)>,
}

/// The headline for a failure, on this machine or the SSH host `remote`.
/// SSH's own failures name the host rather than reading as Git's.
pub(crate) fn error_title(error: &Error, remote: Option<&str>) -> String {
    use herdr_client::Error as Script;
    let unreachable = |source: &Script| match source {
        Script::ScriptConnect(_) => true,
        // SSH exits 255 when it cannot connect; Git never does.
        Script::ScriptExit { status, .. } => status.code() == Some(255),
        _ => false,
    };
    match (error, remote) {
        (Error::NotARepository, _) => "Not a Git repository".into(),
        (Error::GitMissing, Some(host)) => format!("Git is not installed on {host}"),
        (Error::GitMissing, None) => "Git is not installed on this machine".into(),
        (Error::Script { source, .. }, Some(host)) if unreachable(source) => {
            format!("Cannot reach {host}")
        }
        (
            Error::Script {
                source: Script::ScriptTimeout,
                ..
            },
            host,
        ) => format!("{} took too long to answer", host.unwrap_or("Git")),
        (
            Error::Script {
                source: Script::ScriptOutputLimit,
                ..
            },
            _,
        ) => "The repository's history is too large to read at once".into(),
        (
            Error::Script {
                source: Script::ScriptUnsupported,
                ..
            },
            Some(_),
        ) => "Remote Git graphs need macOS or Linux".into(),
        (
            Error::Script {
                source: Script::ScriptUnsupported,
                ..
            },
            None,
        ) => "The Git graph needs macOS or Linux".into(),
        (Error::Host(_), _) => "The Git graph needs a local or SSH host".into(),
        _ => "Could not read the repository".into(),
    }
}

impl GitGraphView {
    /// A view of `checkout` read on `host`, or showing why it cannot be.
    /// It reads once the host is online.
    pub(crate) fn new(
        host: Result<Host, Error>,
        checkout: GraphCheckout,
        look: GraphLook,
        connection: Connection,
        cx: &mut Context<Self>,
    ) -> Self {
        #[cfg(test)]
        let kept = host.as_ref().ok().cloned();
        let (worker, error) = match host {
            Ok(host) => (Some(Worker::start(host, checkout.checkout.clone())), None),
            Err(error) => (None, Some(Arc::new(error))),
        };
        let prefs = cx.observe_global::<GraphPrefsStore>(|this, cx| {
            this.prefs = GraphPrefsStore::get(cx);
            cx.notify();
        });
        let mut view = Self {
            waiting: false,
            checkout,
            focus_handle: cx.focus_handle(),
            look,
            prefs: GraphPrefsStore::get(cx),
            _prefs: prefs,
            scroll: UniformListScrollHandle::new(),
            selected: None,
            column_menu: false,
            worker,
            #[cfg(test)]
            host: kept,
            remote: None,
            connection,
            latest: (0, Request::Load),
            history: None,
            error,
            chip_widths: HashMap::new(),
            hovered_chip: None,
        };
        view.send(Request::Load);
        view
    }

    /// Names the SSH host the repository is on, for messages.
    pub(crate) fn remote(mut self, remote: Option<String>) -> Self {
        self.remote = remote;
        self
    }

    /// Follows the host's connection: nothing is asked of an offline host,
    /// and the repository is read again when it comes back.
    pub(crate) fn set_connection(&mut self, connection: Connection, cx: &mut Context<Self>) {
        if connection == self.connection {
            return;
        }
        self.connection = connection;
        if connection == Connection::Online {
            self.send(Request::Load);
        }
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn connection(&self) -> Connection {
        self.connection
    }

    /// What the top bar's indicator says.
    pub(crate) fn indicator(&self) -> &'static str {
        match self.connection {
            Connection::Online => "live",
            Connection::Offline => "offline",
        }
    }

    #[cfg(test)]
    pub(crate) fn host(&self) -> Option<&Host> {
        self.host.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn worker_finished(&self) -> Option<Arc<std::sync::atomic::AtomicBool>> {
        self.worker.as_ref().map(Worker::finished)
    }

    /// Takes the worker's news; whether anything changed.
    pub(crate) fn poll(&mut self, cx: &mut Context<Self>) -> bool {
        let mut changed = false;
        while let Some((generation, reply)) = self.worker.as_mut().and_then(Worker::try_recv) {
            changed |= self.receive(generation, reply, cx);
        }
        changed
    }

    /// Applies a reply unless a later request made it stale.
    pub(crate) fn receive(
        &mut self,
        generation: u64,
        reply: Reply,
        cx: &mut Context<Self>,
    ) -> bool {
        if generation < self.latest.0 {
            return false;
        }
        self.waiting = false;
        match reply {
            Reply::History(history) => {
                self.history = Some(history);
                self.error = None;
            }
            Reply::Failed(error) => {
                // A failed load leaves nothing worth showing; a failed page
                // keeps the rows already read.
                if self.latest.1 == Request::Load {
                    self.history = None;
                }
                self.error = Some(error);
            }
        }
        cx.notify();
        true
    }

    fn send(&mut self, request: Request) {
        if self.connection == Connection::Offline {
            return;
        }
        if let Some(worker) = self.worker.as_mut() {
            self.latest = (worker.send(request), request);
            self.waiting = true;
        }
    }

    /// Asks for the next page once the rows drawn come near the end, one
    /// request at a time.
    fn more_if_near_the_end(&mut self, drawn_to: usize) {
        let Some(history) = &self.history else {
            return;
        };
        if history.has_more && !self.waiting && drawn_to + MORE_WITHIN >= history.commits.len() {
            self.send(Request::More);
        }
    }

    /// Reads the repository again.
    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) {
        self.send(Request::Load);
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn scroll_to_row(&mut self, row: usize) {
        self.scroll.scroll_to_item(row, ScrollStrategy::Top);
    }

    /// The selected commit's row in the rows loaded.
    pub(crate) fn selected_row(&self) -> Option<usize> {
        let oid = self.selected.as_deref()?;
        self.history.as_ref()?.row_of(oid)
    }

    /// Selects `row` and scrolls it into view.
    fn select_row(&mut self, row: usize, cx: &mut Context<Self>) {
        let Some(commit) = self
            .history
            .as_ref()
            .and_then(|history| history.commits.get(row))
        else {
            return;
        };
        self.selected = Some(commit.oid.clone());
        self.column_menu = false;
        self.scroll.scroll_to_item(row, ScrollStrategy::Nearest);
        cx.notify();
    }

    /// How strongly `row`'s band is tinted.
    pub(crate) fn band(&self, row: usize) -> f32 {
        if self.selected_row() == Some(row) {
            lanes::BAND_SELECTED
        } else {
            lanes::BAND
        }
    }

    pub(crate) fn set_look(&mut self, look: GraphLook, cx: &mut Context<Self>) {
        if look.theme == self.look.theme
            && look.ui.family == self.look.ui.family
            && look.ui.size == self.look.ui.size
            && look.mono.family == self.look.mono.family
            && look.mono.size == self.look.mono.size
        {
            return;
        }
        if look.ui.family != self.look.ui.family {
            self.chip_widths.clear();
        }
        self.look = look;
        cx.notify();
    }

    /// The graph column's width: chosen, or fitted to the lanes.
    fn graph_width(&self) -> f32 {
        let lanes = self
            .history
            .as_ref()
            .map_or(1, |history| history.layout.lane_count);
        self.prefs.graph_width.unwrap_or_else(|| graph_width(lanes))
    }

    /// Which of `row`'s labels fit in the label column.
    #[cfg(test)]
    pub(crate) fn chip_fit(
        &mut self,
        row: usize,
        window: &mut Window,
    ) -> Option<super::chips::ChipFit> {
        let history = self.history.clone()?;
        let chips = history.chips.get(&history.commits.get(row)?.oid)?;
        Some(self.fit(chips, window))
    }

    #[cfg(test)]
    pub(crate) fn generation(&self) -> u64 {
        self.latest.0
    }

    #[cfg(test)]
    pub(crate) fn last_request(&self) -> (u64, Request) {
        self.latest
    }

    #[cfg(test)]
    pub(crate) fn history(&self) -> Option<&Arc<History>> {
        self.history.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn error(&self) -> Option<&Arc<Error>> {
        self.error.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn is_loading(&self) -> bool {
        self.waiting && self.history.is_none()
    }

    fn render_message(&self, title: &str, detail: Option<String>) -> AnyElement {
        let theme = &self.look.theme;
        div()
            .id("graph-message")
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .text_color(rgb(theme.foreground))
                    .child(title.to_owned()),
            )
            .when_some(detail, |el, detail| {
                el.child(div().text_color(rgb(theme.muted)).child(detail))
            })
            .into_any_element()
    }
}

impl Focusable for GitGraphView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GitGraphView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.look.theme;
        let body = match (&self.history, &self.error) {
            (None, Some(error)) => self.render_message(
                &error_title(error, self.remote.as_deref()),
                Some(format!("{} \u{2014} {error}", self.checkout.checkout)),
            ),
            (None, None) if self.connection == Connection::Offline => self.render_message(
                "Offline",
                Some(format!(
                    "{} is not connected",
                    self.remote.as_deref().unwrap_or("The host")
                )),
            ),
            (None, None) => self.render_message("Loading\u{2026}", None),
            (Some(history), _) if history.commits.is_empty() => {
                self.render_message("No commits yet", None)
            }
            (Some(history), _) => div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(self.render_columns(cx))
                .child(
                    uniform_list(
                        "graph-rows",
                        history.commits.len(),
                        cx.processor(|this, range: std::ops::Range<usize>, window, cx| {
                            this.more_if_near_the_end(range.end);
                            let Some(history) = this.history.clone() else {
                                return Vec::new();
                            };
                            let now = chrono::Utc::now().timestamp();
                            range
                                .map(|row| this.render_row(&history, row, now, window, cx))
                                .collect()
                        }),
                    )
                    .track_scroll(&self.scroll)
                    .flex_1()
                    .min_h_0(),
                )
                .into_any_element(),
        };
        let bar = self.render_bar(cx);
        div()
            .id("git-graph")
            .key_context("GitGraph")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key))
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<ColumnDrag>, _, cx| {
                    let x = f32::from(event.event.position.x - event.bounds.origin.x);
                    this.drag_column(*event.drag(cx), x, cx);
                }),
            )
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(theme.background))
            .text_color(rgb(theme.foreground))
            .text_font(&self.look.ui)
            .text_size(px(self.look.ui.size))
            .line_height(px(ROW_H))
            .child(bar)
            .child(body)
    }
}
