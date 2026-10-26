//! The notes panel drawn beside an annotated page.

use super::super::Tab;
use crate::{
    HerdrWindow,
    browser::annotate::{Anchor, Note},
    design::{ButtonKind, Edge, Tone},
};
use gpui::{prelude::*, *};
use std::{sync::Arc, time::Instant};

impl HerdrWindow {
    /// The notes panel beside the page.
    pub(crate) fn render_annotations(&mut self, tab: &Tab, cx: &mut Context<Self>) -> AnyElement {
        let id = tab.id;
        let theme = self.theme.clone();
        let look = self.view_look();
        let armed = self.browser.annotations.armed(id);
        let notes = self.tab_notes(id);
        let input = notes.input.clone();
        let regions = notes.regions;
        let editing = notes.editing;
        let pending = notes
            .pending
            .as_ref()
            .map(|draft| {
                (
                    draft.anchor.summary(),
                    draft.image.clone(),
                    draft.capture.is_some(),
                )
            })
            .or_else(|| {
                let note = notes.notes.get(editing?)?;
                Some((note.anchor.summary(), note.image.clone(), false))
            });
        let list: Vec<(usize, Note)> = notes.notes.iter().cloned().enumerate().collect();
        let unsent = list.iter().filter(|(_, note)| !note.sent).count();
        let send_label = crate::agent_notes::send_label(list.len(), unsent);
        let any_sent = unsent < list.len();
        let now = Instant::now();
        self.browser.annotations.observe_notes(id, list.len(), now);
        let origin = tab.origin.is_some();
        let tab_for_send = tab.clone();
        let tab_for_copy = tab.clone();
        let button = |id: &'static str, label: SharedString, kind: ButtonKind| {
            look.button(id, id.into(), None, label, kind, true)
        };
        let primary = |lead: bool| {
            if lead {
                ButtonKind::Primary
            } else {
                ButtonKind::Normal
            }
        };
        let thumbnail = |image: Arc<Image>| {
            img(image)
                .max_w_full()
                .max_h(px(96.))
                .rounded(px(crate::config::corners::CONTROL))
                .border_1()
                .border_color(rgb(theme.active))
        };
        let composer = pending.map(|(summary, image, capturing)| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .border_b_1()
                .border_color(rgb(theme.active))
                .child(div().text_color(rgb(theme.muted)).truncate().child(summary))
                .children(image.map(thumbnail))
                .when(capturing, |draft| {
                    draft.child(
                        div()
                            .text_color(rgb(theme.muted))
                            .child("Taking a screenshot\u{2026}"),
                    )
                })
                .child(
                    div()
                        .id("annotation-input")
                        .debug_selector(|| "annotation-input".into())
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            match event.keystroke.key.as_str() {
                                "enter" => this.add_note(id, window, cx),
                                "escape" => this.cancel_note(id, window, cx),
                                _ => return,
                            }
                            cx.stop_propagation();
                        }))
                        .children(input),
                )
                .child(
                    div().flex().gap_1().child(
                        button(
                            "annotation-add",
                            if editing.is_some() {
                                "Save note"
                            } else {
                                "Add note"
                            }
                            .into(),
                            ButtonKind::Primary,
                        )
                        .on_click(
                            cx.listener(move |this, _, window, cx| this.add_note(id, window, cx)),
                        ),
                    ),
                )
        });
        let growth: Vec<Option<f32>> = (0..list.len())
            .map(|index| self.browser.annotations.note_growth(id, index, now))
            .collect();
        let rows = list.into_iter().map(|(index, note)| {
            let sent = note.sent;
            div()
                .id(("annotation-note", index))
                // A new note opens into the list and fades in.
                .when_some(growth[index], |row, k| {
                    row.max_h(px(320. * k)).overflow_hidden().opacity(k)
                })
                .when(sent && growth[index].is_none(), |row| row.opacity(0.6))
                .when(editing == Some(index), |row| row.bg(rgb(theme.active)))
                .flex()
                .gap_2()
                .p_2()
                .border_b_1()
                .border_color(rgb(theme.active))
                .child(look.settings().chip(
                    format!("annotation-note-{index}-number"),
                    (index + 1).to_string(),
                    Tone::Warn,
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_color(rgb(theme.muted))
                                .truncate()
                                .child(if sent {
                                    format!("{} \u{b7} sent", note.anchor.summary())
                                } else {
                                    note.anchor.summary()
                                }),
                        )
                        // Clicking the text edits it.
                        .child(
                            div()
                                .id(("annotation-edit", index))
                                .debug_selector(move || format!("annotation-edit-{index}"))
                                .cursor_text()
                                // A page's result arrives without a note.
                                .when(note.comment.is_empty(), |hint| {
                                    hint.text_color(rgb(theme.muted))
                                        .child("Add a note\u{2026}")
                                })
                                .child(note.comment)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.edit_note(id, index, window, cx);
                                })),
                        )
                        .children(note.image.clone().map(thumbnail)),
                )
                .child(
                    look.icon_button(
                        ("annotation-remove", index),
                        format!("annotation-remove-{index}"),
                        "icons/close.svg",
                        false,
                        true,
                    )
                    .size(look.at(22.))
                    .on_click(cx.listener(move |this, _, _, cx| this.remove_note(id, index, cx))),
                )
        });
        let has_notes = !self.tab_notes(id).notes.is_empty();
        let panel = look
            .side_panel("annotations", Edge::Right)
            .id("annotations")
            .h_full()
            .child(
                look.side_header("annotations-header", "Notes", None)
                    .child(
                        // Drag draws a region while this is on; Shift-drag
                        // draws one either way.
                        button(
                            "annotation-region",
                            "Region".into(),
                            if regions {
                                ButtonKind::Lit
                            } else {
                                ButtonKind::Normal
                            },
                        )
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_regions(id, cx))),
                    )
                    .child(
                        button("annotation-page", "Note on page".into(), ButtonKind::Normal)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.begin_note(id, Anchor::Page, None, window, cx);
                            })),
                    ),
            )
            .children(composer)
            .child(
                div()
                    .id("annotation-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(rows)
                    .when(!has_notes && armed, |list| {
                        list.child(
                            div()
                                .p_2()
                                .text_color(rgb(theme.muted))
                                .child("Click an element, select text, or Shift-drag a region in the page, then describe the change."),
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
                        .p(look.at(10.))
                        .border_t_1()
                        .border_color(look.line())
                        .child(
                            button("annotation-copy", "Copy".into(), primary(!origin)).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    this.copy_notes(&tab_for_copy, cx)
                                }),
                            ),
                        )
                        .when(any_sent, |row| {
                            row.child(
                                button(
                                    "annotation-clear-sent",
                                    "Clear sent".into(),
                                    ButtonKind::Normal,
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.clear_sent_notes(id, cx)
                                })),
                            )
                        })
                        .child(div().flex_1())
                        .when(origin, |row| {
                            row.child(
                                button("annotation-send", send_label.into(), ButtonKind::Primary)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.send_notes(&tab_for_send, cx)
                                    })),
                            )
                        }),
                )
            });
        self.resizable_panel(
            panel,
            "annotations-resize",
            crate::panel_resize::PanelDrag::PageNotes,
            None,
            cx,
        )
        .into_any_element()
    }
}
