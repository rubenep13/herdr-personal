//! A pull request's own pages: its conversation (drawn by `conversation`,
//! commented on and merged through `pr_actions`, which sends each write once
//! and never retries it) and its checks, from the status the worker looked
//! up by number.

use super::{OrchestratorView, dispatch::Confirm};
use crate::{
    design::Tone,
    orchestrator::Item,
    pr_actions,
    pull_request::{MergeMethod, Outcome, PullRequest, State},
};
use gpui::{prelude::*, *};

impl OrchestratorView {
    /// The token that reads the followed pull request's repository.
    fn pr_token(&self) -> Option<std::sync::Arc<secrecy::SecretString>> {
        let tokens = self.request.token.as_ref()?;
        Some(match self.pr.target() {
            Some(target) => tokens.for_owner(&target.owner).clone(),
            None => tokens.main().clone(),
        })
    }

    /// The looked-up status of the open pull request, once it arrived.
    pub(super) fn open_pull_request(&self) -> Option<&PullRequest> {
        let number = self
            .item(&self.detail.as_ref()?.key)?
            .pull_request
            .as_ref()?
            .number;
        let lookup = self.snapshot.pull_request.as_ref()?;
        if lookup.number != number {
            return None;
        }
        lookup.result.as_ref().as_ref().ok()?.as_ref()
    }

    /// Follows the open pull request, and reads its conversation when the
    /// Conversation page shows and nothing was read yet.
    pub(super) fn follow_pull_request(&mut self) {
        let target = self
            .open_pull_request()
            .and_then(|pr| pr_actions::Target::for_reading(pr).ok());
        self.pr.track(target);
        let wants =
            matches!(&self.detail, Some(detail) if detail.tab == super::DetailTab::Conversation);
        if wants
            && self.pr.target().is_some()
            && self.pr.comments().is_none()
            && !self.pr.loading()
            && self.pr.comments_error().is_none()
            && self.pr.running().is_none()
            && let Some(token) = self.pr_token()
        {
            let _ = self.pr.load_comments(token);
        }
    }

    pub(super) fn post_comment(&mut self, cx: &mut Context<Self>) {
        let text = self.comment.read(cx).text().trim().to_owned();
        let (Some(token), Some(pr)) = (self.pr_token(), self.open_pull_request()) else {
            return;
        };
        let methods = pr.merge_methods.clone();
        if text.is_empty() {
            return;
        }
        if self
            .pr
            .start(pr_actions::Action::Comment(text), &methods, token)
            .is_ok()
        {
            self.comment.update(cx, |input, cx| input.clear(cx));
        }
        cx.notify();
    }

    /// Merges the open pull request at the head the user saw.
    /// Merges `expected`, the pull request and head commit the user confirmed.
    /// A refresh that moved the head, or another pull request opened since,
    /// refuses the merge instead of naming a commit the user never saw.
    pub(super) fn merge(
        &mut self,
        method: MergeMethod,
        expected: &pr_actions::Target,
        cx: &mut Context<Self>,
    ) {
        if self.pr.target() != Some(expected) {
            self.notice = Some(crate::orchestrator::Notice {
                outcome: Err(std::sync::Arc::new(
                    crate::orchestrator::Error::PullRequestChanged,
                )),
            });
            cx.notify();
            return;
        }
        let (Some(token), Some(pr)) = (self.pr_token(), self.open_pull_request()) else {
            return;
        };
        let methods = pr.merge_methods.clone();
        let _ = self
            .pr
            .start(pr_actions::Action::Merge(method), &methods, token);
        cx.notify();
    }

    /// Reads the conversation again while its page shows; the comments read
    /// before stay until the new ones arrive.
    pub(super) fn reload_conversation(&mut self) {
        let shown =
            matches!(&self.detail, Some(detail) if detail.tab == super::DetailTab::Conversation);
        if shown
            && self.pr.target().is_some()
            && self.pr.running().is_none()
            && let Some(token) = self.pr_token()
        {
            let _ = self.pr.load_comments(token);
        }
    }

    pub(super) fn render_checks(&self, item: &Item) -> AnyElement {
        let look = &self.look;
        let view = look.view();
        let mut column = div()
            .id("orchestrator-checks")
            .flex_1()
            .min_w_0()
            .overflow_y_scroll()
            .p(view.at(16.))
            .flex()
            .flex_col()
            .gap(view.at(4.));
        let Some(pr) = self.open_pull_request() else {
            let text = match self.lookup_problem(item) {
                _ if self.request.token.is_none() => {
                    "Sign in to GitHub to read the checks.".to_owned()
                }
                Some(problem) => problem,
                None => "Reading the checks\u{2026}".to_owned(),
            };
            return column.child(look.muted(text)).into_any_element();
        };
        column = column
            .child(look.label(format!("CHECKS \u{00b7} {}", pr.checks_summary)))
            .child(view.list().px_0().children(pr.check_list().enumerate().map(
                |(index, (name, outcome))| {
                    let (icon, tone) = outcome_mark(outcome);
                    let (ink, _) = view.settings().tone(tone);
                    view.row(
                        ("orchestrator-check", index),
                        format!("orchestrator-check-{index}"),
                        false,
                        false,
                    )
                    .cursor_default()
                    .child(
                        svg()
                            .path(icon)
                            .size(view.at(14.))
                            .flex_none()
                            .text_color(ink),
                    )
                    .child(div().flex_1().min_w_0().truncate().child(name.to_owned()))
                    .child(look.muted(outcome.to_string()))
                },
            )))
            .child(div().h(view.at(12.)))
            .child(look.label("REVIEW"))
            .child(
                div()
                    .text_size(px(view.scale().secondary))
                    .child(pr.review()),
            )
            .child(div().h(view.at(12.)))
            .child(look.label("MERGE"))
            .child(
                div()
                    .text_size(px(view.scale().secondary))
                    .text_color(view.text())
                    .child(format!("{} \u{00b7} {}", pr.lifecycle(), pr.merge_status())),
            );
        column.into_any_element()
    }

    /// Why `item`'s pull request lookup came back without one: its error,
    /// or GitHub knowing no such pull request. `None` while it is read.
    pub(super) fn lookup_problem(&self, item: &Item) -> Option<String> {
        let lookup = self.snapshot.pull_request.as_ref()?;
        if Some(lookup.number) != item.pull_request.as_ref().map(|pr| pr.number) {
            return None;
        }
        match lookup.result.as_ref() {
            Err(error) => Some(error.to_string()),
            Ok(Some(_)) => None,
            Ok(None) => Some("GitHub reports no such pull request.".to_owned()),
        }
    }

    /// The merge methods GitHub allows here, as a small menu under the button.
    pub(super) fn render_merge_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.merge_open {
            return None;
        }
        let pr = self.open_pull_request()?;
        let look = &self.look;
        let view = look.view();
        Some(
            deferred(
                view.menu_panel("orchestrator-merge-menu")
                    .absolute()
                    .top(view.at(MERGE_TOP))
                    .right(view.at(12.))
                    .w(view.at(MERGE_WIDTH))
                    .children(pr.merge_methods.iter().copied().map(|method| {
                        let selector = format!("orchestrator-merge-{}", method.graphql());
                        view.row(SharedString::from(selector.clone()), selector, false, false)
                            .child(method.action())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.merge_open = false;
                                // The confirmation names what the menu showed.
                                if let Some(target) = this.pr.target().cloned() {
                                    this.ask(Confirm::Merge { method, target }, cx);
                                }
                            }))
                    }))
                    .when(pr.merge_methods.is_empty(), |el| {
                        el.child(look.muted("This repository allows no merge method."))
                    }),
            )
            .into_any_element(),
        )
    }

    /// Whether the open pull request can be merged from here.
    pub(super) fn mergeable(&self) -> bool {
        self.open_pull_request()
            .is_some_and(|pr| pr.state == State::Open && !pr.is_draft)
            && self.pr.target().is_some()
            && self.pr.running().is_none()
    }
}

/// A check's result as an icon in its tone: passed green, failed red,
/// pending yellow, skipped grey.
fn outcome_mark(outcome: Outcome) -> (&'static str, Tone) {
    match outcome {
        Outcome::Passed => ("icons/check.svg", Tone::Good),
        Outcome::Failed => ("icons/close.svg", Tone::Danger),
        Outcome::Pending => ("icons/clock.svg", Tone::Warn),
        Outcome::Skipped => ("icons/chevron-right.svg", Tone::Neutral),
    }
}

/// The Merge menu sits under the bar, at the orchestrator's face of 12.
const MERGE_TOP: f32 = 40.;
const MERGE_WIDTH: f32 = 220.;
