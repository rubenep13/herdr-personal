//! The sidebar's column: its header, level with the main header beside it,
//! with the toggle after the traffic lights' room and the sidebar's tools at
//! its end, then the sidebar itself.

use super::{LEADING, SIDEBAR_TOGGLE_SIZE, movable};
use crate::HerdrWindow;
use gpui::{prelude::*, *};

impl HerdrWindow {
    /// Keep the toggle anchored in the sidebar whenever it fits. The rail has
    /// room on Linux/Windows, but not beside macOS's traffic-light clearance;
    /// the main header takes it whenever the sidebar cannot.
    pub(super) fn sidebar_header_has_toggle(&self, window: &Window) -> bool {
        self.sidebar_mode()
            .width(self.sidebar_width, f32::from(window.viewport_size().width))
            .is_some_and(|width| width >= LEADING + SIDEBAR_TOGGLE_SIZE)
    }

    /// The sidebar column's first row: the toggle whenever it fits, then the
    /// sidebar's tools at its end.
    pub(crate) fn sidebar_header(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let header = div()
            .debug_selector(|| "sidebar-titlebar".into())
            .flex()
            .flex_none()
            .items_center()
            .w_full()
            .h(px(crate::design::HEADER_HEIGHT))
            .pr(px(10.))
            .overflow_hidden()
            .bg(self.fill().panel(self.theme.sidebar_background()))
            .border_b_1()
            .border_color(rgb(self.theme.active))
            .child(div().flex_none().w(px(LEADING)).h_full())
            .when(self.sidebar_header_has_toggle(window), |header| {
                header.child(self.sidebar_toggle(cx))
            })
            .child(self.sidebar_tools(window, cx));
        movable(header, window)
    }

    /// The sidebar under its header row, at the width the sidebar takes.
    pub(crate) fn sidebar_column(
        &self,
        sidebar: impl IntoElement,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let width = self
            .sidebar_mode()
            .width(self.sidebar_width, f32::from(window.viewport_size().width))
            .unwrap_or(0.);
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(width))
            .min_h_0()
            .child(self.sidebar_header(window, cx))
            .child(div().flex().flex_1().min_h_0().child(sidebar))
    }

    /// The development build's worktree banner.
    pub(crate) fn render_worktree_banner(&self) -> Option<Div> {
        crate::worktree_banner::render(
            env!("HERDR_BUILD_WORKTREE") == "1",
            env!("HERDR_BUILD_BRANCH"),
            env!("HERDR_BUILD_PR"),
        )
    }
}
