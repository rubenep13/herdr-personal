//! The notes beside the diff: the composer for the line being noted or the
//! note being edited, the notes, and Send, Copy, or Clear sent.
use super::{PANEL_SHARE, Review};
use crate::{
    HerdrWindow,
    browser::TabId,
    design::{ButtonKind, Edge, Tone},
    fonts::StyledFont,
    review::notes::Note,
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(super) fn render_review_notes(
        &self,
        id: TabId,
        review: &Review,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let look = self.view_look();
        let settings = look.settings();
        let button = |id: &'static str, label: SharedString, primary: bool| {
            look.button(
                id,
                id.into(),
                None,
                label,
                if primary {
                    ButtonKind::Primary
                } else {
                    ButtonKind::Normal
                },
                true,
            )
        };
        let editing = review.editing;
        let drafting = review
            .draft
            .zip(review.loaded())
            .and_then(|(row, loaded)| loaded.diff.anchor(row))
            .and_then(|anchor| Note::new(anchor, "x"))
            .map(|note| note.place())
            .or_else(|| Some(review.notes.get(editing?)?.place()));
        let place = |text: String| {
            div()
                .min_w_0()
                .truncate()
                .text_font(&self.config.terminal)
                .text_size(look.at(PLACE_TEXT))
                .text_color(look.muted())
                .child(text)
        };
        let composer = drafting.map(|drafted| {
            div()
                .flex()
                .flex_col()
                .gap(look.at(6.))
                .px(look.at(PADDING))
                .pb(look.at(PADDING))
                .border_b_1()
                .border_color(look.line())
                .child(place(drafted))
                .child(
                    div()
                        .id("review-input")
                        .debug_selector(|| "review-input".into())
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            let composing = this
                                .reviews
                                .get(&id)
                                .is_some_and(|review| review.input.read(cx).is_composing());
                            if composing {
                                return;
                            }
                            match event.keystroke.key.as_str() {
                                "enter" => this.add_review_note(id, window, cx),
                                "escape" => this.cancel_review_note(id, window, cx),
                                _ => return,
                            }
                            cx.stop_propagation();
                        }))
                        .child(review.input.clone()),
                )
                .child(
                    div().flex().justify_end().child(
                        button(
                            "review-add",
                            if editing.is_some() {
                                "Save note"
                            } else {
                                "Add note"
                            }
                            .into(),
                            true,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| this.add_review_note(id, window, cx),
                        )),
                    ),
                )
        });
        let rows = review.notes.iter().enumerate().map(|(index, note)| {
            settings
                .card(format!("review-note-{index}"))
                .id(("review-note", index))
                .when(note.sent, |card| card.opacity(0.6))
                .when(editing == Some(index), |card| card.bg(look.selection()))
                .flex_row()
                .items_start()
                .gap(look.at(8.))
                .px(look.at(12.))
                .py(look.at(10.))
                .child(settings.chip(
                    format!("review-note-{index}-number"),
                    (index + 1).to_string(),
                    Tone::Warn,
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(look.at(4.))
                        .child(place(if note.sent {
                            format!("{} \u{b7} sent", note.place())
                        } else {
                            note.place()
                        }))
                        // Clicking the text edits it.
                        .child(
                            div()
                                .id(("review-edit", index))
                                .debug_selector(move || format!("review-edit-{index}"))
                                .cursor_text()
                                .text_color(look.title())
                                .child(note.comment.clone())
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.edit_review_note(id, index, window, cx)
                                })),
                        ),
                )
                .child(
                    look.icon_button(
                        ("review-remove", index),
                        format!("review-remove-{index}"),
                        "icons/close.svg",
                        false,
                        true,
                    )
                    .size(look.at(REMOVE))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.remove_review_note(id, index, cx)),
                    ),
                )
        });
        let has_notes = !review.notes.is_empty();
        let has_agent = review.agent.is_some();
        let unsent = review.notes.iter().filter(|note| !note.sent).count();
        let send_label = crate::agent_notes::send_label(review.notes.len(), unsent);
        let any_sent = unsent < review.notes.len();
        let panel =
            look.side_panel("review-notes", Edge::Right)
                .id("review-notes")
                .h_full()
                .child(look.side_header(
                    "review-notes-header",
                    "Notes",
                    has_notes.then_some(review.notes.len()),
                ))
                .children(composer)
                .child(
                    div()
                        .id("review-note-list")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .px(look.at(PADDING))
                        .py(look.at(8.))
                        .flex()
                        .flex_col()
                        .gap(look.at(8.))
                        .children(rows)
                        .when(!has_notes && review.draft.is_none(), |list| {
                            list.child(
                                div().text_color(look.muted()).child(
                                    "Click a line or a file name to note what should change.",
                                ),
                            )
                        }),
                )
                // The action that sends is the footer's last, as a dialog's is.
                .when(has_notes, |panel| {
                    panel.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.at(6.))
                            .p(look.at(PADDING))
                            .border_t_1()
                            .border_color(look.line())
                            .child(button("review-copy", "Copy".into(), !has_agent).on_click(
                                cx.listener(move |this, _, _, cx| this.copy_review(id, cx)),
                            ))
                            .when(any_sent, |row| {
                                row.child(
                                    button("review-clear-sent", "Clear sent".into(), false)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.clear_sent_review_notes(id, cx)
                                        })),
                                )
                            })
                            .child(div().flex_1())
                            .when(has_agent, |row| {
                                row.child(button("review-send", send_label.into(), true).on_click(
                                    cx.listener(move |this, _, _, cx| this.send_review(id, cx)),
                                ))
                            }),
                    )
                });
        self.resizable_panel(
            panel,
            "review-notes-resize",
            crate::panel_resize::PanelDrag::ReviewNotes,
            Some(PANEL_SHARE),
            cx,
        )
    }
}

/// The panel's padding, a note's place and its remove button, at `[ui]
/// size = 12`.
const PADDING: f32 = 10.;
const PLACE_TEXT: f32 = 11.5;
const REMOVE: f32 = 22.;
