//! The console's window: its bar, the level menu, the log's lines, the
//! selected line's detail and the status line, in the redesigned views'
//! pieces. Reading and filtering live in the parent.

use super::{
    Appearance, Close, FocusLevel, FocusSearch, LEVELS, LogWindow, level_tone, palette_color,
    severity_color,
};
use crate::{
    config::Theme,
    design::{ButtonKind, ViewLook},
    diagnostics::Record,
    fonts::StyledFont,
};
use gpui::{prelude::*, *};
use std::sync::Arc;

/// The search field's width when the bar has room for it; it gives way
/// first when the window narrows.
const SEARCH_WIDTH: f32 = 260.;
/// The column the levels' chips share, so every message starts at one edge.
const LEVEL_WIDTH: f32 = 52.;
/// A line's padding above and below, which with the terminal's line height
/// makes the redesign's 24 px rows.
const LINE_PADDING_Y: f32 = 2.;
const LINE_RADIUS: f32 = 6.;
const LINE_INSET: f32 = 8.;
const LEVEL_MENU_WIDTH: f32 = 180.;
/// Below this window width, at `[ui] size = 12`, the bar's buttons keep only
/// their icons, named by their tooltips, and the level drops its prefix: the
/// bar cannot wrap as the old header did.
const COMPACT_BELOW: f32 = 760.;

/// Whether the console's theme is light, which the menus' shadow follows.
fn light(theme: &Theme) -> bool {
    crate::contrast::luminance(theme.background) > 0.5
}

fn styled_body(record: &Record, theme: &Theme) -> StyledText {
    use std::fmt::Write;
    let mut text = record.target.clone();
    let target_end = text.len();
    for span in &record.spans {
        let _ = write!(text, " [{span}]");
    }
    let spans_end = text.len();
    if !record.message.is_empty() {
        let _ = write!(text, " {}", record.message);
    }
    let fields_start = text.len();
    for (key, value) in &record.fields {
        let _ = write!(text, " {key}={value}");
    }
    if record.truncated {
        text.push_str(" [truncated]");
    }
    let end = text.len();
    StyledText::new(text).with_highlights(
        [
            (
                0..target_end,
                HighlightStyle {
                    color: Some(palette_color(theme, 6).into()),
                    ..Default::default()
                },
            ),
            (
                target_end..spans_end,
                HighlightStyle {
                    color: Some(rgb(theme.muted).into()),
                    ..Default::default()
                },
            ),
            (
                fields_start..end,
                HighlightStyle {
                    color: Some(palette_color(theme, 4).into()),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .filter(|(range, _)| !range.is_empty()),
    )
}

impl LogWindow {
    fn render_bar(&self, look: &ViewLook, window: &Window, cx: &mut Context<Self>) -> Div {
        let search_focused = self.search.read(cx).focus.is_focused(window);
        let level_focused = self.level_focus.is_focused(window);
        let compact = window.viewport_size().width < look.at(COMPACT_BELOW);
        let button = |id: &'static str, icon: &'static str, label: &'static str| {
            if compact {
                look.icon_button(id, id.into(), icon, false, true)
                    .tooltip(look.menu().tooltip(label))
            } else {
                look.button(id, id.into(), Some(icon), label, ButtonKind::Normal, true)
            }
        };
        look.bar("logs-bar")
            .child(look.bar_title(Some("icons/logs.svg"), "Logs", None))
            .child(div().flex_1())
            .child(
                look.field(
                    "logs-search".into(),
                    search_focused,
                    Some("icons/search.svg"),
                )
                .w(look.at(SEARCH_WIDTH))
                .flex_shrink(1.)
                .child(div().flex_1().min_w_0().child(self.search.clone())),
            )
            .child(
                look.dropdown(
                    "minimum-level",
                    "minimum-level".into(),
                    None,
                    (!compact).then_some("Minimum:"),
                    self.minimum.to_string(),
                )
                .track_focus(&self.level_focus)
                .when(level_focused, |dropdown| {
                    dropdown.border_color(look.accent())
                })
                .on_click(cx.listener(|this, _, window, cx| this.open_levels(window, cx)))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(
                        event.keystroke.key.as_str(),
                        "enter" | "space" | "down" | "up"
                    ) {
                        cx.stop_propagation();
                        window.prevent_default();
                        this.open_levels(window, cx);
                    }
                }))
                .when_some(self.level_menu, |dropdown, selected| {
                    dropdown.child(self.render_level_menu(look, selected, cx))
                }),
            )
            .child(
                button(
                    "follow",
                    if self.following {
                        "icons/pause.svg"
                    } else {
                        "icons/play.svg"
                    },
                    if self.following {
                        "Pause"
                    } else {
                        "Resume tail"
                    },
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.following = !this.following;
                    this.generation = None;
                    cx.notify();
                })),
            )
            .child(
                button("copy", "icons/copy.svg", "Copy")
                    .on_click(cx.listener(|this, _, _, cx| this.share(false, cx))),
            )
            .child(
                button(
                    "export",
                    "icons/download.svg",
                    if self.exporting {
                        "Working..."
                    } else {
                        "Export..."
                    },
                )
                .on_click(cx.listener(|this, _, _, cx| this.share(true, cx))),
            )
    }

    fn render_level_menu(
        &self,
        look: &ViewLook,
        selected: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        deferred(
            anchored().child(
                look.menu_panel("level-menu")
                    .absolute()
                    .top_full()
                    .left_0()
                    .min_w(look.at(LEVEL_MENU_WIDTH))
                    .occlude()
                    .track_focus(&self.menu_focus)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .on_mouse_down_out(
                        cx.listener(|this, _, window, cx| this.close_levels(window, cx)),
                    )
                    .on_key_down(cx.listener(Self::level_key))
                    .children(LEVELS.iter().enumerate().map(|(index, level)| {
                        let level = *level;
                        look.choice(
                            ("level-option", index),
                            format!("level-option-{index}"),
                            level.to_string(),
                            index == selected,
                            self.minimum == level,
                        )
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            if crate::design::menu_hover(&mut this.level_menu, index, *hovered) {
                                cx.notify();
                            }
                        }))
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.minimum = level;
                                this.generation = None;
                                this.close_levels(window, cx);
                            },
                        ))
                    })),
            ),
        )
        .with_priority(1)
    }

    fn render_line(&self, look: &ViewLook, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let record = self.rows[index].clone();
        let theme = &self.appearance.theme;
        let font = &self.appearance.config.terminal;
        let selected = self
            .selected
            .as_ref()
            .is_some_and(|chosen| Arc::ptr_eq(chosen, &record));
        let hover = look.hover();
        div()
            .w_full()
            .px(look.at(LINE_INSET))
            .child(
                div()
                    .id(index)
                    .debug_selector(move || format!("log-row-{index}"))
                    .flex()
                    .w_full()
                    .gap(look.at(LINE_INSET))
                    .py(px(LINE_PADDING_Y))
                    .px(look.at(LINE_INSET))
                    .rounded(look.at(LINE_RADIUS))
                    .text_color(rgb(theme.foreground))
                    .text_font(font)
                    .text_size(px(font.size))
                    .line_height(px(font.line_height()))
                    .child(
                        div()
                            .flex_none()
                            .whitespace_nowrap()
                            .text_color(look.muted())
                            .child(record.timestamp.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .w(look.at(LEVEL_WIDTH))
                            .h(px(font.line_height()))
                            .flex()
                            .items_center()
                            .child(look.settings().chip(
                                format!("log-level-{index}"),
                                record.level.as_str(),
                                level_tone(record.level),
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .whitespace_normal()
                            .debug_selector(move || format!("log-body-{index}"))
                            .child(styled_body(&record, theme)),
                    )
                    .cursor_pointer()
                    .when(selected, |line| line.bg(look.selection()))
                    .when(!selected, |line| line.hover(move |line| line.bg(hover)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = Some(record.clone());
                        cx.notify();
                    })),
            )
            .into_any_element()
    }
}

impl Render for LogWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.following {
            self.scroll.scroll_to(ListOffset {
                item_ix: self.rows.len(),
                offset_in_item: px(0.),
            });
        }
        let header = crate::titlebar::header(&self.appearance.theme, window, |window, _| {
            window.remove_window();
        });
        let Appearance { config, theme } = &self.appearance;
        let look = ViewLook::of(theme, &config.ui, light(theme));
        let root = div()
            .key_context("LogWindow")
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &Close, window, _| window.remove_window()))
            .on_action(cx.listener(|_, action: &crate::RunCommand, window, cx| {
                crate::window::run_window_command(action.command, window, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| {
                this.level_menu = None;
                window.focus(&this.search.read(cx).focus.clone(), cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FocusLevel, window, cx| {
                this.level_menu = None;
                window.focus(&this.level_focus, cx);
                cx.notify();
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(look.background())
            .text_color(rgb(theme.foreground))
            .text_font(&config.ui)
            .text_size(px(config.ui.size))
            .line_height(px(config.ui.line_height()))
            .children(header)
            .child(self.render_bar(&look, window, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .pt(look.at(4.))
                    .on_scroll_wheel(cx.listener(|this, _, _, cx| {
                        this.following = false;
                        cx.notify();
                    }))
                    .when(self.rows.is_empty(), |area| {
                        area.child(look.empty("log-empty", None, "No matching logs."))
                    })
                    .when(!self.rows.is_empty(), |area| {
                        area.child(
                            list(
                                self.scroll.clone(),
                                cx.processor(|this, index: usize, _, cx| {
                                    let Appearance { config, theme } = &this.appearance;
                                    let look = ViewLook::of(theme, &config.ui, light(theme));
                                    this.render_line(&look, index, cx)
                                }),
                            )
                            .size_full(),
                        )
                    }),
            )
            .when_some(self.selected.clone(), |root, record| {
                root.child(
                    div()
                        .debug_selector(|| "log-detail".into())
                        .flex_none()
                        .h((window.viewport_size().height * 0.2).min(px(100.)))
                        .p(look.at(8.))
                        .bg(look.side())
                        .border_t_1()
                        .border_color(look.line())
                        .child(
                            look.dialog()
                                .well("log-detail-well".into(), Some(&config.terminal))
                                .id("log-detail")
                                .size_full()
                                .overflow_y_scroll()
                                .text_color(severity_color(theme, record.level))
                                .text_size(px(config.terminal.size))
                                .line_height(px(config.terminal.line_height()))
                                .child(record.line()),
                        ),
                )
            })
            .child(
                div()
                    .flex_none()
                    .px(look.at(14.))
                    .py(look.at(6.))
                    .border_t_1()
                    .border_color(look.line())
                    .truncate()
                    .text_size(px(look.scale().secondary))
                    .text_color(look.muted())
                    .child(format!(
                        "{} shown | {} dropped | {} | {}",
                        self.rows.len(),
                        self.dropped,
                        if self.following { "LIVE" } else { "PAUSED" },
                        self.status
                    )),
            );
        let border = theme.active;
        crate::titlebar::frame(window, border, root)
    }
}
