//! Fonts: one family and size per face, then a live specimen of the face in use.
use super::{SettingsWindow, ui};
use crate::{
    config::{FONT_SIZE_RANGE, FontFace},
    font_picker::FontTarget,
    settings_window::{
        controls::{FACES, face_font, family_label},
        inputs::NumberField,
    },
};
use gpui::{prelude::*, *};

/// The number control's id for a face; ids live for the whole run.
pub(in crate::settings_window) const fn size_control(face: FontFace) -> &'static str {
    match face {
        FontFace::Terminal => "fonts-terminal-size-control",
        FontFace::Sidebar => "fonts-sidebar-size-control",
        FontFace::Tabs => "fonts-tabs-size-control",
        FontFace::Ui => "fonts-ui-size-control",
    }
}

fn specimen_title(face: FontFace) -> &'static str {
    match face {
        FontFace::Terminal => "Specimen \u{b7} Terminal",
        FontFace::Sidebar => "Specimen \u{b7} Sidebar",
        FontFace::Tabs => "Specimen \u{b7} Tabs",
        FontFace::Ui => "Specimen \u{b7} Interface",
    }
}

impl SettingsWindow {
    pub(in crate::settings_window) fn render_fonts(&self, cx: &mut Context<Self>) -> Div {
        #[cfg(all(feature = "integration-test", target_os = "macos"))]
        self.reset_native_font_bounds(cx);
        let look = &self.look();
        let mut faces = ui::group(look, "faces", "Faces", None);
        for (face, label) in FACES {
            faces = faces.row(self.face_row(face, label, cx));
        }
        let faces = faces.row(
            ui::row(look, "fonts-all-row", "All faces").control(
                self.all_families_button()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_control_font_picker(FontTarget::All, window, cx)
                    })),
            ),
        );
        div().flex().flex_col().min_w_0().child(faces).child(
            ui::group(
                look,
                "specimen",
                specimen_title(self.active_font_face()),
                None,
            )
            .body(self.render_font_specimen()),
        )
    }

    fn all_families_button(&self) -> Stateful<Div> {
        self.font_button("fonts-all".into(), "Use one family for all\u{2026}")
    }

    fn face_row(
        &self,
        face: FontFace,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let look = &self.look();
        let field = NumberField::FontSize(face);
        let id = field.selector();
        let size = self.control_face_size(face);
        let view = cx.entity().downgrade();
        let row = ui::row(look, format!("fonts-{}", face.name()), label)
            .control(self.family_button(face, cx))
            .control(
                ui::number(
                    look,
                    ui::Number {
                        id,
                        value: size,
                        range: FONT_SIZE_RANGE,
                        unit: "px",
                        slider: false,
                        enabled: true,
                    },
                    self.number_edit_for(field),
                    {
                        let view = view.clone();
                        move |change, _, cx| {
                            let _ =
                                view.update(cx, |this, cx| this.commit_number(field, change, cx));
                        }
                    },
                    move |window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.begin_number_edit(field, size, window, cx)
                        });
                    },
                )
                .map(|number| {
                    #[cfg(all(feature = "integration-test", target_os = "macos"))]
                    let number = number.child(crate::settings_window::controls::native_probe(id));
                    number
                }),
            );
        #[cfg(all(feature = "integration-test", target_os = "macos"))]
        let row = div()
            .child(row)
            .child(crate::settings_window::controls::native_probe(format!(
                "fonts-{}",
                face.name()
            )));
        row
    }

    /// The family as a dropdown-styled button that opens the font picker.
    fn family_button(&self, face: FontFace, cx: &mut Context<Self>) -> Stateful<Div> {
        let look = self.look();
        self.font_button(format!("fonts-{}-family", face.name()), "")
            .w(look.at(220.))
            .justify_between()
            .gap(look.at(8.))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .child(family_label(&face_font(&self.config, face).family).to_owned()),
            )
            .child(
                svg()
                    .path("icons/chevron-down.svg")
                    .size(look.at(14.))
                    .flex_none()
                    .text_color(look.muted()),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_control_font_picker(FontTarget::Face(face), window, cx);
            }))
    }
}
