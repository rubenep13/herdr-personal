//! `?`: every shortcut of the graph, as the design's table lists them.

use super::GitGraphView;
use crate::{config::mix, fonts::StyledFont};
use gpui::{prelude::*, *};

/// The graph's keys and what they do; they act with the graph focused and
/// no modifier held.
pub(crate) const SHORTCUTS: &[(&str, &str)] = &[
    ("j / \u{2193}, k / \u{2191}", "Next / previous commit"),
    ("Enter", "Open the detail"),
    ("Esc", "Close the detail or the search"),
    ("/", "Search"),
    ("n / N", "Next / previous result"),
    ("w", "Uncommitted changes"),
    ("h", "HEAD"),
    ("c", "Copy the hash"),
    ("r", "Reload"),
    ("?", "Help with every shortcut"),
];

impl GitGraphView {
    pub(super) fn toggle_help(&mut self, cx: &mut Context<Self>) {
        self.help = !self.help;
        cx.notify();
    }

    pub(super) fn render_help(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.help {
            return None;
        }
        let theme = &self.look.theme;
        let rows = SHORTCUTS.iter().enumerate().map(|(index, (keys, does))| {
            div()
                .debug_selector(move || format!("graph-help-row-{index}"))
                .flex()
                .gap_3()
                .child(
                    div()
                        .flex_none()
                        .w(px(150.))
                        .text_font(&self.look.mono)
                        .text_color(rgb(theme.foreground))
                        .child(*keys),
                )
                .child(div().text_color(rgb(theme.subtext())).child(*does))
        });
        Some(
            deferred(
                div()
                    .id("graph-help")
                    .debug_selector(|| "graph-help".to_owned())
                    .occlude()
                    .absolute()
                    .top(px(56.))
                    .right(px(12.))
                    .w(px(440.))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgb(theme.active))
                    .bg(rgb(mix(theme.surface, theme.foreground, 4)))
                    .shadow_lg()
                    .text_size(px(12.5))
                    .line_height(px(22.))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(theme.foreground))
                            .child("Keyboard shortcuts"),
                    )
                    .child(
                        div()
                            .pb_1()
                            .text_size(px(11.5))
                            .text_color(rgb(theme.muted))
                            .child("With the graph focused and no modifier held."),
                    )
                    .children(rows)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_help(cx))),
            )
            .into_any_element(),
        )
    }
}
