//! The VS Code tab's request for the license the app accepts for VS Code's
//! server when it starts one. Until the user accepts it, here or in
//! Settings > Code, the app starts nothing.
use super::code_view::{READING_WIDTH, VSCODE};
#[cfg(not(test))]
use crate::config::{CodeEdit, Config};
use crate::{
    HerdrWindow,
    code_server::launcher::{LICENSE_TERMS, LICENSE_URL, PRIVACY_URL},
    design::ButtonKind,
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    /// The terms, links to read them, and the button that accepts them.
    pub(super) fn render_code_consent(&self, cx: &mut Context<Self>) -> AnyElement {
        let look = self.view_look();
        let link = |id: &'static str, label: &'static str, url: &'static str| {
            div()
                .id(id)
                .debug_selector(move || id.into())
                .text_color(look.accent())
                .cursor_pointer()
                .hover(|style| style.underline())
                .on_click(move |_, _, cx| cx.open_url(url))
                .child(label)
        };
        let accept = look
            .dialog()
            .button("code-accept", "Accept and start", ButtonKind::Primary, true)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                cx.stop_propagation();
                this.accept_code_license(cx);
            }));
        look.empty("code-consent", Some(VSCODE), "Start VS Code")
            .flex_1()
            .child(
                div()
                    .max_w(look.at(READING_WIDTH))
                    .text_center()
                    .text_size(px(look.scale().secondary))
                    .text_color(look.muted())
                    .child(LICENSE_TERMS),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_center()
                    .gap(look.at(16.))
                    .text_size(px(look.scale().secondary))
                    .child(link("code-license-link", "License Terms", LICENSE_URL))
                    .child(link("code-privacy-link", "Privacy Statement", PRIVACY_URL)),
            )
            .child(accept)
            .into_any_element()
    }

    /// Remembers that the user accepted the license, and starts VS Code at
    /// once rather than after the config reloads.
    pub(crate) fn accept_code_license(&mut self, cx: &mut Context<Self>) {
        self.config.code.license_accepted = true;
        // Tests never write the user's config.
        #[cfg(not(test))]
        self.write_preference(|| Config::save_code(CodeEdit::AcceptLicense), cx);
        cx.notify();
    }
}
