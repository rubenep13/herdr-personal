//! Asks before opening a terminal hyperlink whose scheme is not one that
//! opens directly. Terminal output is untrusted, and such an address starts
//! whatever application registered its scheme, so the user sees the whole
//! address first and nothing opens unless they say so.

use super::Page;
use crate::{
    HerdrWindow,
    design::{ButtonKind, Header, NoteTone},
    terminal::ExternalUrl,
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    /// Shows the question for `url`, unless another page holds the window.
    pub(crate) fn ask_to_open_link(
        &mut self,
        url: ExternalUrl,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.menu.page.is_some() || !self.open_menu(window, cx) {
            return;
        }
        self.menu.open_link = Some(url);
        self.menu.page = Some(Page::OpenLink);
    }

    /// Opens the address the question is about, and closes the question.
    pub(in crate::menu) fn confirm_open_link(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let url = self.menu.open_link.take();
        self.dismiss_menu(window, cx);
        if let Some(url) = url {
            cx.open_url(url.as_str());
        }
    }

    pub(in crate::menu) fn render_open_link(&self, cx: &mut Context<Self>) -> Div {
        let Some(url) = &self.menu.open_link else {
            return div();
        };
        let look = self.dialog_look();
        let dismiss = |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            this.dismiss_menu(window, cx)
        };
        div()
            .debug_selector(|| "open-link".into())
            .flex()
            .flex_col()
            .child(
                look.header(
                    Header {
                        selector: "open-link-header",
                        icon: "icons/link.svg",
                        title: (
                            "open-link-title",
                            "Are you sure you want to open this link?",
                        ),
                        sub: None,
                        danger: false,
                    },
                    look.menu()
                        .close_button("open-link-dismiss")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            dismiss(this, window, cx);
                        })),
                ),
            )
            .child(
                look.body("open-link-body")
                    .child(
                        look.well("open-link-url".into(), Some(&self.config.terminal))
                            .child(url.to_string()),
                    )
                    .child(look.note(
                        "open-link-note".into(),
                        NoteTone::Info,
                        "A program in the terminal printed it. It opens in another application.",
                    )),
            )
            .child(
                look.footer("open-link-footer")
                    .hint("Esc", "cancel")
                    .hint("↵", "yes")
                    .button(
                        look.button("open-link-no", "No", ButtonKind::Normal, true)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                dismiss(this, window, cx);
                            })),
                    )
                    .button(
                        look.button("open-link-yes", "Yes", ButtonKind::Primary, true)
                            .on_click(cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.confirm_open_link(window, cx);
                            })),
                    ),
            )
    }
}
