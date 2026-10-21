//! The review's header: the panels' toggles, which changes show and against
//! what, and how they are drawn: unified or side by side, with or without
//! changes in whitespace alone, with a field to find text in them.
use super::{Layout, Review, panels};
use crate::{HerdrWindow, browser::TabId, review::diff::Scope};
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(super) fn render_review_header(
        &self,
        id: TabId,
        review: &Review,
        cx: &mut Context<Self>,
    ) -> Div {
        let look = self.view_look();
        let destination = match &review.agent {
            Some(agent) => format!("Notes go to {}", agent.label),
            None => "No agent in this workspace; notes can be copied".into(),
        };
        let against = match review.loaded().and_then(|loaded| loaded.base.as_deref()) {
            Some(base) if review.scope == Scope::Branch => {
                format!("{} against {base}", review.checkout.branch)
            }
            _ => review.checkout.branch.clone(),
        };
        let quiet = |selector: &'static str| {
            div()
                .debug_selector(move || selector.into())
                .min_w_0()
                .truncate()
                .text_size(px(look.scale().secondary))
                .text_color(look.muted())
        };
        look.bar("review-bar")
            .child(self.render_review_panel_toggle(id, review, panels::Panel::Files, cx))
            .child(look.bar_title(Some("icons/diff-unified.svg"), "Review changes", None))
            .child(self.render_review_scope(id, review.scope, cx))
            // The branch is the bar's context: it gives way first.
            .child(
                quiet("review-against")
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(look.at(5.))
                    .child(
                        svg()
                            .path("icons/git-branch.svg")
                            .size(look.at(11.))
                            .flex_none()
                            .text_color(look.muted()),
                    )
                    .child(div().min_w_0().truncate().child(against)),
            )
            .when(review.batch_out, |header| {
                header.child(quiet("review-reading").flex_none().child("Reading\u{2026}"))
            })
            .child(
                self.review_icon_button(
                    "review-search-open",
                    "icons/search.svg",
                    "Find in changes",
                    review.search.open,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.open_review_search(id, window, cx);
                })),
            )
            .child(
                self.review_icon_button(
                    "review-whitespace",
                    "icons/whitespace.svg",
                    "Hide whitespace changes",
                    review.ignore_whitespace,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.toggle_review_whitespace(id, cx);
                })),
            )
            .child(self.render_review_layout(id, review.layout, cx))
            .child(look.bar_rule())
            .child(self.render_review_panel_toggle(id, review, panels::Panel::Notes, cx))
            // Where notes go, last and quiet; it gives way before the buttons.
            .child(
                quiet("review-destination")
                    .flex_shrink(1.)
                    .child(destination),
            )
    }

    /// A square icon button of the bar, lit when `on`.
    fn review_icon_button(
        &self,
        name: &'static str,
        icon: &'static str,
        hint: &'static str,
        on: bool,
    ) -> Stateful<Div> {
        let look = self.view_look();
        look.icon_button(name, name.into(), icon, on, true)
            .tooltip(look.menu().tooltip(hint))
    }

    /// Shows the diff unified or side by side; notes and the place stay.
    pub(crate) fn set_review_layout(&mut self, id: TabId, layout: Layout, cx: &mut Context<Self>) {
        if let Some(review) = self.reviews.get_mut(&id) {
            review.set_layout(layout);
        }
        cx.notify();
    }

    /// The two layouts, as a segmented control of icons.
    fn render_review_layout(&self, id: TabId, current: Layout, cx: &mut Context<Self>) -> Div {
        let look = self.view_look();
        let segment =
            |name: &'static str, icon: &'static str, hint: &'static str, layout: Layout| {
                let on = layout == current;
                look.segment(
                    name,
                    name.into(),
                    svg()
                        .path(icon)
                        .size(look.at(14.))
                        .flex_none()
                        .text_color(if on { look.title() } else { look.muted() }),
                    on,
                )
                .px(look.at(5.))
                .tooltip(look.menu().tooltip(hint))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.set_review_layout(id, layout, cx);
                }))
            };
        look.segments("review-layout".into())
            .child(segment(
                "review-layout-unified",
                "icons/diff-unified.svg",
                "Unified",
                Layout::Unified,
            ))
            .child(segment(
                "review-layout-split",
                "icons/diff-split.svg",
                "Side by side",
                Layout::Split,
            ))
    }

    fn render_review_scope(&self, id: TabId, current: Scope, cx: &mut Context<Self>) -> Div {
        let look = self.view_look();
        let segment = |name: &'static str, label: &'static str, scope: Scope| {
            look.segment(name, name.into(), label, scope == current)
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.set_review_scope(id, scope, cx);
                }))
        };
        look.segments("review-scope".into())
            .child(segment(
                "review-scope-uncommitted",
                "Uncommitted",
                Scope::Uncommitted,
            ))
            .child(segment("review-scope-branch", "Branch", Scope::Branch))
    }
}
