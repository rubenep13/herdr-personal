//! The tools at the end of the sidebar's header: the Tree's search, behind
//! a magnifier until it is used, and on Linux the application menu, which
//! has no menu bar of its own there.

use crate::{
    HerdrWindow,
    design::{ICON_BUTTON, ICON_BUTTON_RADIUS},
    sidebar::SidebarMode,
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    /// Whether the sidebar's header has room for its tools: only the
    /// expanded sidebar does; a rail keeps its toggle.
    pub(crate) fn sidebar_tools_shown(&self) -> bool {
        self.sidebar_mode() == SidebarMode::Expanded
    }

    /// The search button or its open field, then the menu button.
    pub(super) fn sidebar_tools(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let shown = self.sidebar_tools_shown();
        let search =
            shown && self.config.layout.mode.nests_agents() && self.config.show_sidebar_search;
        let field = search
            .then(|| self.tree_header_search(window, cx))
            .flatten();
        let open = field.is_some();
        div()
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .items_center()
            .justify_end()
            .gap(px(4.))
            .children(field)
            .when(search && !open, |tools| {
                tools.child(self.sidebar_search_button(cx))
            })
            .when(cfg!(target_os = "linux") && shown, |tools| {
                tools.child(self.application_menu_button(cx))
            })
    }

    fn sidebar_search_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let active = self.theme.active;
        div()
            .id("sidebar-search-button")
            .debug_selector(|| "sidebar-search-button".into())
            .size(px(ICON_BUTTON))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(ICON_BUTTON_RADIUS))
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(active)))
            // The header moves the window; its buttons must not.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                svg()
                    .path("icons/search.svg")
                    .size(px(16.))
                    .text_color(rgb(self.theme.muted)),
            )
            .on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.focus_tree_search(window, cx);
            }))
    }
}
