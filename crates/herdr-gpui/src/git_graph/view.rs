//! A Git graph tab's view: it asks its worker for the history and draws what
//! comes back. It knows nothing of tabs; the window feeds it from its tick.

use super::{
    checkout::GraphCheckout,
    error::Error,
    history::History,
    worker::{Reply, Request, Worker},
};
use crate::{
    config::{FontConfig, Theme},
    fonts::StyledFont,
    teleport::Host,
};
use gpui::{prelude::*, *};
use std::sync::Arc;

/// hgg's row height.
const ROW_HEIGHT: f32 = 30.;

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
    scroll: UniformListScrollHandle,
    /// `None` when the host cannot run scripts; `error` says why.
    worker: Option<Worker>,
    /// The generation and kind of the last request; older replies are stale.
    latest: (u64, Request),
    waiting: bool,
    history: Option<Arc<History>>,
    error: Option<Arc<Error>>,
}

/// The headline for a failure the tab shows instead of rows.
pub(crate) fn error_title(error: &Error) -> &'static str {
    match error {
        Error::NotARepository => "Not a Git repository",
        Error::GitMissing => "Git is not installed on this host",
        _ => "Could not read the repository",
    }
}

impl GitGraphView {
    /// A view of `checkout` read on `host`, or showing why it cannot be.
    pub(crate) fn new(
        host: Result<Host, Error>,
        checkout: GraphCheckout,
        look: GraphLook,
        cx: &mut Context<Self>,
    ) -> Self {
        let (worker, error, latest) = match host {
            Ok(host) => {
                let mut worker = Worker::start(host, checkout.checkout.clone());
                let generation = worker.send(Request::Load);
                (Some(worker), None, generation)
            }
            Err(error) => (None, Some(Arc::new(error)), 0),
        };
        Self {
            waiting: worker.is_some(),
            checkout,
            focus_handle: cx.focus_handle(),
            look,
            scroll: UniformListScrollHandle::new(),
            worker,
            latest: (latest, Request::Load),
            history: None,
            error,
        }
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
        if let Some(worker) = self.worker.as_mut() {
            self.latest = (worker.send(request), request);
            self.waiting = true;
        }
    }

    /// Reads the repository again.
    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) {
        self.send(Request::Load);
        cx.notify();
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
        self.look = look;
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn generation(&self) -> u64 {
        self.latest.0
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

    fn render_row(&self, history: &History, row: usize) -> AnyElement {
        let theme = &self.look.theme;
        let Some(commit) = history.commits.get(row) else {
            return div().into_any_element();
        };
        div()
            .id(("graph-row", row))
            .debug_selector(move || format!("graph-row-{row}"))
            .h(px(ROW_HEIGHT))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .text_color(rgb(theme.muted))
                    .text_font(&self.look.mono)
                    .child(commit.oid.chars().take(7).collect::<String>()),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(rgb(theme.foreground))
                    .child(commit.subject.clone()),
            )
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
                error_title(error),
                Some(format!("{} \u{2014} {error}", self.checkout.checkout)),
            ),
            (None, None) => self.render_message("Loading\u{2026}", None),
            (Some(history), _) if history.commits.is_empty() => {
                self.render_message("No commits yet", None)
            }
            (Some(history), _) => uniform_list(
                "graph-rows",
                history.commits.len(),
                cx.processor(|this, range: std::ops::Range<usize>, _, _| {
                    let Some(history) = this.history.clone() else {
                        return Vec::new();
                    };
                    range.map(|row| this.render_row(&history, row)).collect()
                }),
            )
            .track_scroll(&self.scroll)
            .flex_1()
            .min_h_0()
            .into_any_element(),
        };
        div()
            .id("git-graph")
            .key_context("GitGraph")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(theme.background))
            .text_color(rgb(theme.foreground))
            .text_font(&self.look.ui)
            .text_size(px(self.look.ui.size))
            .child(body)
    }
}
