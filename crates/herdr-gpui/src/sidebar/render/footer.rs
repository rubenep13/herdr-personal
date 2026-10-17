//! The spaces list's foot in every layout but the Tree's: "new" starts a
//! workspace and "menu" opens the app's menu, as in the terminal client.

use crate::{
    HerdrWindow,
    controls::Command,
    sidebar::{label_text, layout::SidebarLook, line_height},
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(super) fn list_footer(&self, look: SidebarLook, cx: &mut Context<Self>) -> Div {
        let (font, theme) = (&self.config.sidebar, &self.theme);
        let (muted, foreground) = (theme.muted, theme.foreground);
        div()
            .flex_none()
            .h(px(line_height(font) + 2. * look.density.footer_padding()))
            .px(px(look.content_x()))
            .flex()
            .items_center()
            // Menu hugs the sidebar's edge, as in the terminal client.
            .justify_between()
            .text_color(rgb(muted))
            .gap(px(20.))
            .child(
                div()
                    .id("new-workspace")
                    .debug_selector(|| "new-workspace".into())
                    .cursor_pointer()
                    .hover(move |s| s.text_color(rgb(foreground)))
                    .child("new")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.command(Command::Workspace, window, cx)
                    })),
            )
            .child(
                div()
                    .id("sidebar-menu")
                    .debug_selector(|| "sidebar-menu".into())
                    .cursor_pointer()
                    .hover(move |s| s.text_color(rgb(foreground)))
                    .child(label_text("menu"))
                    .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                        this.menu.anchor = event.position();
                        this.open_menu(window, cx);
                    })),
            )
    }
}
