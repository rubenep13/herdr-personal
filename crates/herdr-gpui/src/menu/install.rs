//! The dialog that asks for Herdr when no daemon can start: what to do, and
//! a button that opens Herdr's website. Nothing is installed from here.

use crate::{
    HerdrWindow,
    design::{ButtonKind, Header},
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(super) fn render_install(&self, cx: &mut Context<Self>) -> Div {
        let look = self.dialog_look();
        let close = look
            .menu()
            .close_button("install-dismiss")
            .on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.dismiss_menu(window, cx);
            }));
        div()
            .flex()
            .flex_col()
            .child(look.header(
                Header {
                    selector: "install-header",
                    icon: "icons/download.svg",
                    title: ("install-title", "Herdr must be installed"),
                    sub: None,
                    danger: false,
                },
                close,
            ))
            .child(look.body("install-body").child(look.lead(
                "install-text",
                "Install Herdr first, then choose Terminal > Reconnect. The Install button opens the Herdr website; nothing is installed automatically.",
            )))
            .child(
                look.footer("install-footer")
                    .hint("Esc", "cancel")
                    .button(
                        look.button("menu-dismiss", "Dismiss", ButtonKind::Normal, true)
                            .on_click(cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.dismiss_menu(window, cx);
                            })),
                    )
                    .button(
                        look.button("menu-install", "Install", ButtonKind::Primary, true)
                            .on_click(|_, _, cx| {
                                cx.stop_propagation();
                                cx.open_url(crate::about::WEBSITE);
                            }),
                    ),
            )
    }
}
