//! Searching from the tab: `/` opens a field in the top bar, Enter asks the
//! worker, the first result is selected and the rest walk with n and N.
//! Matching rows are marked, and a list under the bar names the results.

use super::GitGraphView;
use crate::{
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

/// The open search's field, and the results' panel and columns, at the
/// graph's `[ui]`-sized face of 12.
const FIELD_WIDTH: f32 = 240.;
const RESULTS_INSET: f32 = 12.;
const RESULTS_WIDTH: f32 = 560.;
const RESULTS_HEIGHT: f32 = 320.;
const HASH_WIDTH: f32 = 60.;
const AUTHOR_WIDTH: f32 = 120.;
const DATE_WIDTH: f32 = 96.;

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
    pub(super) fn render_search_field(&self, focused: bool) -> Option<AnyElement> {
        let search = self.search.as_ref()?;
        let view = self.look.view();
        let count = match (&search.hits, search.query.is_empty()) {
            _ if search.failed => "Search failed".into(),
            (Some(hits), _) => results_label(hits.len(), search.more),
            (None, false) => "Searching\u{2026}".into(),
            (None, true) => String::new(),
        };
        Some(
            view.field(
                "graph-search-input".to_owned(),
                focused,
                Some("icons/search.svg"),
            )
            .w(view.at(FIELD_WIDTH))
            .flex_shrink(1.)
            .child(div().flex_1().min_w_0().child(self.search_input.clone()))
            .when(!count.is_empty(), |field| {
                field.child(
                    div()
                        .debug_selector(|| "graph-search-count".to_owned())
                        .flex_none()
                        .text_size(px(view.scale().secondary))
                        .text_color(view.muted())
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
        let view = self.look.view();
        let now = chrono::Utc::now().timestamp();
        let rows = hits.iter().enumerate().map(|(index, hit)| {
            let chosen = search.index == Some(index);
            view.row(
                ("graph-search-result", index),
                format!("graph-search-result-{index}"),
                false,
                chosen,
            )
            .child(
                div()
                    .flex_none()
                    .w(view.at(HASH_WIDTH))
                    .text_font(&self.look.mono)
                    .text_size(px(view.scale().detail))
                    .text_color(view.text())
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
                    .w(view.at(AUTHOR_WIDTH))
                    .truncate()
                    .text_size(px(view.scale().secondary))
                    .text_color(view.muted())
                    .child(hit.author.clone()),
            )
            .child(
                div()
                    .flex_none()
                    .w(view.at(DATE_WIDTH))
                    .truncate()
                    .text_size(px(view.scale().secondary))
                    .text_color(view.muted())
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
                view.menu_panel("graph-search-results")
                    .occlude()
                    .absolute()
                    .top(px(top))
                    .right(view.at(RESULTS_INSET))
                    .w(view.at(RESULTS_WIDTH))
                    .max_h(view.at(RESULTS_HEIGHT))
                    .overflow_y_scroll()
                    .children(rows),
            )
            .into_any_element(),
        )
    }
}
