//! `?`: every shortcut of the graph, as the design's table lists them.

use super::GitGraphView;
use crate::design::Header;
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
        let view = self.look.view();
        let menu = view.menu();
        let rows = SHORTCUTS.iter().enumerate().map(|(index, (keys, does))| {
            let selector = format!("graph-help-row-{index}");
            div()
                .debug_selector({
                    let selector = selector.clone();
                    move || selector
                })
                .h(px(menu.row_height()))
                .px(view.at(10.))
                .flex()
                .items_center()
                .gap(view.at(12.))
                .child(
                    div()
                        .flex_none()
                        .w(view.at(KEYS_WIDTH))
                        .flex()
                        .items_center()
                        .gap(view.at(4.))
                        .children(key_parts(keys).enumerate().map(|(key, part)| {
                            match part {
                                Part::Key(text) => menu
                                    .keycap(format!("{selector}-key-{key}"), text.to_owned())
                                    .into_any_element(),
                                Part::Between(text) => div()
                                    .text_color(view.muted())
                                    .child(text)
                                    .into_any_element(),
                            }
                        })),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(view.text())
                        .child(*does),
                )
        });
        Some(
            deferred(
                view.menu_panel("graph-help")
                    .occlude()
                    .absolute()
                    .top(view.at(TOP))
                    .right(view.at(12.))
                    .w(view.at(WIDTH))
                    .p_0()
                    .child(menu.header(Header {
                        selector: "graph-help-header",
                        icon: "icons/keyboard.svg",
                        title: ("graph-help-title", "Keyboard shortcuts"),
                        sub: Some((
                            "icons/info.svg",
                            "graph-help-sub",
                            "With the graph focused and no modifier held.",
                        )),
                        danger: false,
                    }))
                    .child(div().p(view.at(6.)).flex().flex_col().children(rows))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_help(cx))),
            )
            .into_any_element(),
        )
    }
}

/// The panel's place under the bar, its width and its keys' column, at the
/// graph's `[ui]`-sized face of 12.
const TOP: f32 = 56.;
const WIDTH: f32 = 440.;
const KEYS_WIDTH: f32 = 150.;

/// A piece of a shortcut's keys: one key, or the words between keys.
enum Part<'a> {
    Key(&'a str),
    Between(&'a str),
}

/// `j / ↓, k / ↑` as the keys `j`, `↓`, `k` and `↑`, with the `/` and `,`
/// between them as words.
fn key_parts(keys: &str) -> impl Iterator<Item = Part<'_>> {
    keys.split(", ").enumerate().flat_map(|(group, keys)| {
        (group > 0).then_some(Part::Between(",")).into_iter().chain(
            keys.split(" / ")
                .enumerate()
                .flat_map(|(alternative, key)| {
                    (alternative > 0)
                        .then_some(Part::Between("/"))
                        .into_iter()
                        .chain(std::iter::once(Part::Key(key)))
                }),
        )
    })
}
