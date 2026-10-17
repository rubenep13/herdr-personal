//! Searching from the tab: `/` opens a field in the top bar, Enter asks the
//! worker, the first result is selected and the rest walk with n and N.
//! Matching rows are marked, and a list under the bar names the results.

use super::GitGraphView;
use crate::{
    config::mix,
    fonts::StyledFont,
    git_graph::{
        search::{SearchHit, results_label},
        text::relative_time,
        worker::Request,
    },
};
use gpui::{prelude::*, *};
use std::{collections::HashSet, sync::Arc};

/// The search a tab shows: its query and, once read, its results.
#[derive(Default)]
pub(super) struct Search {
    query: String,
    generation: u64,
    hits: Option<Arc<Vec<SearchHit>>>,
    more: bool,
    /// The result selected last, for n and N.
    index: Option<usize>,
    failed: bool,
}

impl GitGraphView {
    /// Opens the field, focused.
    pub(super) fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.get_or_insert_with(Search::default);
        let focus = self.search_input.read(cx).focus.clone();
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Asks for the commits matching `query`.
    pub(crate) fn start_search(&mut self, query: &str, cx: &mut Context<Self>) {
        let query = query.trim().to_owned();
        self.matches.clear();
        if query.is_empty() {
            self.search = Some(Search::default());
            cx.notify();
            return;
        }
        let generation = self.send(Request::Search(query.clone()));
        self.search = Some(Search {
            query,
            generation,
            ..Search::default()
        });
        cx.notify();
    }

    pub(super) fn receive_search(
        &mut self,
        generation: u64,
        query: &str,
        hits: Arc<Vec<SearchHit>>,
        more: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(search) = self.search.as_mut() else {
            return false;
        };
        if generation < search.generation || query != search.query {
            return false;
        }
        self.matches = hits
            .iter()
            .map(|hit| hit.oid.clone())
            .collect::<HashSet<_>>();
        let first = hits.first().map(|hit| hit.oid.clone());
        search.index = first.as_ref().map(|_| 0);
        search.hits = Some(hits);
        search.more = more;
        if let Some(oid) = first {
            self.jump_to(&oid, cx);
        }
        true
    }

    /// Selects the next (or previous) result, after the last one the first.
    pub(super) fn step_result(&mut self, forward: bool, cx: &mut Context<Self>) -> bool {
        let Some(search) = self.search.as_mut() else {
            return false;
        };
        let Some(hits) = search.hits.clone().filter(|hits| !hits.is_empty()) else {
            return false;
        };
        let count = hits.len();
        let index = match (search.index, forward) {
            (None, _) => 0,
            (Some(index), true) => (index + 1) % count,
            (Some(index), false) => (index + count - 1) % count,
        };
        search.index = Some(index);
        self.jump_to(&hits[index].oid, cx);
        true
    }

    /// Closes the field and clears the marks.
    pub(super) fn close_search(&mut self, cx: &mut Context<Self>) -> bool {
        if self.search.take().is_none() {
            return false;
        }
        self.matches.clear();
        self.search_input.update(cx, |input, cx| input.clear(cx));
        cx.notify();
        true
    }

    /// A failed search says so; it never touches the rows.
    pub(super) fn search_failed(&mut self, generation: u64) -> bool {
        let Some(search) = self
            .search
            .as_mut()
            .filter(|search| search.generation == generation)
        else {
            return false;
        };
        search.failed = true;
        search.hits = Some(Arc::default());
        true
    }

    pub(crate) fn is_match(&self, oid: &str) -> bool {
        self.matches.contains(oid)
    }

    #[cfg(test)]
    pub(crate) fn last_search(&self) -> u64 {
        self.search.as_ref().map_or(0, |search| search.generation)
    }

    /// The field and what it found, for the top bar; `None` when closed.
    pub(super) fn render_search_field(&self) -> Option<AnyElement> {
        let search = self.search.as_ref()?;
        let theme = &self.look.theme;
        let count = match (&search.hits, search.query.is_empty()) {
            _ if search.failed => "Search failed".into(),
            (Some(hits), _) => results_label(hits.len(), search.more),
            (None, false) => "Searching\u{2026}".into(),
            (None, true) => String::new(),
        };
        Some(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .debug_selector(|| "graph-search-input".to_owned())
                        .w(px(240.))
                        .h(px(26.))
                        .px_2()
                        .flex()
                        .items_center()
                        .rounded(px(4.))
                        .border_1()
                        .border_color(rgb(theme.active))
                        .bg(rgb(theme.background))
                        .line_height(px(20.))
                        .child(self.search_input.clone()),
                )
                .when(!count.is_empty(), |el| {
                    el.child(
                        div()
                            .debug_selector(|| "graph-search-count".to_owned())
                            .flex_none()
                            .text_size(px(11.5))
                            .text_color(rgb(theme.muted))
                            .child(count),
                    )
                })
                .into_any_element(),
        )
    }

    /// The results under the top bar, each a hash, subject, author and
    /// date; a click selects it in the graph.
    pub(super) fn render_results(&self, top: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        let search = self.search.as_ref()?;
        let hits = search
            .hits
            .as_ref()
            .filter(|hits| !hits.is_empty())?
            .clone();
        let theme = &self.look.theme;
        let now = chrono::Utc::now().timestamp();
        let rows = hits.iter().enumerate().map(|(index, hit)| {
            let chosen = search.index == Some(index);
            div()
                .id(("graph-search-result", index))
                .debug_selector(move || format!("graph-search-result-{index}"))
                .h(px(28.))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .whitespace_nowrap()
                .cursor_pointer()
                .rounded(px(4.))
                .when(chosen, |el| el.bg(rgb(theme.active)))
                .hover(|style| style.bg(rgb(theme.active)))
                .child(
                    div()
                        .flex_none()
                        .w(px(60.))
                        .text_font(&self.look.mono)
                        .text_size(px(11.5))
                        .text_color(rgb(theme.subtext()))
                        .child(hit.oid.chars().take(7).collect::<String>()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .child(hit.subject.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .w(px(120.))
                        .truncate()
                        .text_color(rgb(theme.muted))
                        .child(hit.author.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .w(px(96.))
                        .truncate()
                        .text_color(rgb(theme.muted))
                        .child(relative_time(hit.authored, now)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(search) = this.search.as_mut() {
                        search.index = Some(index);
                    }
                    if let Some(hit) = this
                        .search
                        .as_ref()
                        .and_then(|search| search.hits.as_ref()?.get(index).cloned())
                    {
                        this.jump_to(&hit.oid, cx);
                    }
                }))
        });
        Some(
            deferred(
                div()
                    .id("graph-search-results")
                    .occlude()
                    .absolute()
                    .top(px(top))
                    .right(px(12.))
                    .w(px(560.))
                    .max_h(px(320.))
                    .overflow_y_scroll()
                    .p_1()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(theme.active))
                    .bg(rgb(mix(theme.surface, theme.foreground, 4)))
                    .shadow_lg()
                    .text_size(px(12.5))
                    .line_height(px(28.))
                    .children(rows),
            )
            .into_any_element(),
        )
    }
}
