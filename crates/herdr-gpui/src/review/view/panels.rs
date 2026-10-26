//! Whether a review shows its file list and its notes beside the diff. A
//! review narrower than `NARROW` gives the diff all its room until the user
//! says otherwise with the header's icons; a note being written always shows
//! the notes, so the composer is never hidden.
use super::Review;
use crate::{HerdrWindow, browser::TabId};
use gpui::{prelude::*, *};

/// Below this width, in pixels, a review starts with both panels hidden.
pub(super) const NARROW: f32 = 1000.;

/// A side panel of a review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Panel {
    Files,
    Notes,
}

impl Review {
    /// Whether the review was wide enough at its last layout; unknown
    /// counts as wide.
    fn wide(&self) -> bool {
        let width = self.width.get();
        width <= 0. || width >= NARROW
    }

    pub(super) fn shows(&self, panel: Panel) -> bool {
        match panel {
            Panel::Files => self.files_shown.unwrap_or_else(|| self.wide()),
            Panel::Notes => {
                self.draft.is_some()
                    || self.editing.is_some()
                    || self.notes_shown.unwrap_or_else(|| self.wide())
            }
        }
    }
}

impl HerdrWindow {
    /// Shows or hides one of a review's side panels; the choice then holds
    /// whatever the review's width.
    pub(super) fn toggle_review_panel(&mut self, id: TabId, panel: Panel, cx: &mut Context<Self>) {
        if let Some(review) = self.reviews.get_mut(&id) {
            let shown = !review.shows(panel);
            match panel {
                Panel::Files => review.files_shown = Some(shown),
                Panel::Notes => {
                    // Hiding the notes drops a note being written.
                    if !shown {
                        review.draft = None;
                        review.editing = None;
                    }
                    review.notes_shown = Some(shown);
                }
            }
        }
        cx.notify();
    }

    /// The icon that shows or hides `panel`; the notes one counts the
    /// queued notes, so hidden ones are not forgotten.
    pub(super) fn render_review_panel_toggle(
        &self,
        id: TabId,
        review: &Review,
        panel: Panel,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let shown = review.shows(panel);
        let (name, icon, hint) = match panel {
            Panel::Files => (
                "review-toggle-files",
                "icons/panel-left.svg",
                "Changed files",
            ),
            Panel::Notes => ("review-toggle-notes", "icons/panel-right.svg", "Notes"),
        };
        let count = match panel {
            // Only notes the agent does not have yet.
            Panel::Notes => Some(review.notes.iter().filter(|note| !note.sent).count())
                .filter(|&count| count > 0),
            _ => None,
        };
        let look = self.view_look();
        look.icon_button(name, name.into(), icon, shown, true)
            .when_some(count, |button, count| {
                button
                    .w_auto()
                    .px(look.at(6.))
                    .gap(look.at(4.))
                    .child(look.settings().chip(
                        format!("{name}-count"),
                        count.to_string(),
                        crate::design::Tone::Warn,
                    ))
            })
            .tooltip(look.menu().tooltip(hint))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.toggle_review_panel(id, panel, cx);
            }))
    }
}
