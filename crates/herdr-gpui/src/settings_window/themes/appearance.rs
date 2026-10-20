//! The theme search box, result count and card grid on the Theme page.
use super::{Choice, Scope, SettingsWindow, grid, row_height};
use crate::{
    config::{OMARCHY_THEME, Theme, ThemeName},
    herdr_settings,
    settings_window::ui,
};
use gpui::{
    Context, Div, FontFeatures, FontWeight, ScrollStrategy, Window, div, prelude::*, px, svg,
    uniform_list,
};

/// Where a card's theme comes from, as its second line names it.
pub(super) fn source_label(choice: &Choice) -> &'static str {
    let name = choice.name.as_str();
    match choice.scope {
        Scope::Herdr => "Herdr",
        Scope::App if Theme::BUILTIN_NAMES.contains(&name) => "Built-in",
        Scope::App if name == OMARCHY_THEME => "Omarchy",
        Scope::App if std::path::Path::new(name).is_absolute() || name.starts_with("~/") => "File",
        Scope::App => "Ghostty",
    }
}

impl SettingsWindow {
    /// The search field over the grid, with how many themes it shows.
    pub(in crate::settings_window) fn render_theme_search(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let look = self.look();
        let browser = &self.themes;
        let count = usize::from(browser.ghostty_enabled) * browser.names.len()
            + usize::from(browser.herdr_enabled) * herdr_settings::THEME_NAMES.len();
        let focused = browser.search.read(cx).focus.is_focused(window);
        look.field("settings-theme-search".into(), focused)
            .mt(look.at(2.))
            .on_key_down(cx.listener(Self::theme_browser_key))
            .child(
                svg()
                    .path("icons/search.svg")
                    .size(look.at(15.))
                    .flex_none()
                    .text_color(look.muted()),
            )
            .child(div().flex_1().min_w_0().child(browser.search.clone()))
            .child(
                div()
                    .debug_selector(|| "settings-theme-count".into())
                    .flex_none()
                    .whitespace_nowrap()
                    .text_size(px(look.scale().detail))
                    .text_color(look.muted())
                    .font_features(FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)])))
                    .child(format!(
                        "{} of {} themes{}",
                        browser.filtered.len(),
                        count,
                        if browser.discovering {
                            " / discovering installed themes..."
                        } else {
                            ""
                        }
                    )),
            )
    }

    pub(in crate::settings_window) fn render_theme_results(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        #[cfg(all(feature = "integration-test", target_os = "macos"))]
        grid::clear_native_bounds(cx);
        // The grid fills the page's column: the window past the section list
        // and the body's margins, at most the column's width.
        let look = &self.look();
        let width = (f32::from(window.viewport_size().width)
            - self.navigation_width.width(self.viewport_width)
            - 2. * f32::from(look.at(36.)))
        .min(f32::from(look.at(700.)));
        let columns = grid::columns(width);
        if self.themes.grid.columns != columns {
            self.themes.grid.columns = columns;
            self.themes.scroll.scroll_to_item(
                self.themes.selected.unwrap_or(0) / columns,
                ScrollStrategy::Center,
            );
        }
        let browser = &self.themes;
        let button = |id: &'static str, label: String| ui::button(look, id, label, true);
        div()
            .flex()
            .flex_col()
            .gap(look.at(6.))
            .child(
                div()
                    .id("settings-theme-list")
                    .debug_selector(|| "settings-theme-list".into())
                    // The child list scrolls first; keep wheel events inside the
                    // grid even at its edges or when the results are empty.
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .flex_1()
                    .min_h(px(240.))
                    .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                        if !*hovered {
                            this.hover_theme(None, cx);
                        }
                    }))
                    .overflow_hidden()
                    .when(browser.filtered.is_empty(), |el| {
                        el.child(
                            div()
                                .debug_selector(|| "settings-theme-empty".into())
                                .py(look.at(12.))
                                .text_color(look.muted())
                                .child("No matching themes. Try a shorter search."),
                        )
                    })
                    .when(!browser.filtered.is_empty(), |el| {
                        el.child(
                            uniform_list(
                                "settings-theme-results",
                                browser.filtered.len().div_ceil(columns),
                                cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                                    this.schedule_grid_palettes(range.clone(), cx);
                                    let look = this.look();
                                    range
                                        .map(|row| div().h(px(row_height(&look))).w_full().flex().gap(look.at(10.)).children((0..this.themes.grid.columns).map(|column| {
                                            let index = row * this.themes.grid.columns + column;
                                            if index >= this.themes.filtered.len() {
                                                return div().flex_1().min_w_0().into_any_element();
                                            }
                                             let choice = this.themes.filtered[index].clone();
                                             let name = &choice.name;
                                            let draft =
                                                this.theme_intent.as_ref().is_some_and(|intent| {
                                                    intent.choice.scope == choice.scope
                                                        && match choice.scope {
                                                            Scope::App => {
                                                                ThemeName::side(&intent.choice.name, this.themes.editing_light) == name
                                                            }
                                                            Scope::Herdr => intent.choice.name == *name,
                                                        }
                                                });
                                             let saved = match choice.scope {
                                                 Scope::App => name == this.edited_theme(),
                                                Scope::Herdr => {
                                                    this.shared.as_ref().is_some_and(|shared| {
                                                         shared.theme_name == *name
                                                    })
                                                }
                                            };
                                            let source = source_label(&choice);
                                            let chosen = this.themes.selected == Some(index);
                                            // The chosen card's 2 px ring takes a pixel of its
                                            // padding, so neither it nor its swatch moves.
                                            let ring = if chosen { 2. } else { 1. };
                                            let hover = look.hover();
                                            div()
                                                .id(index)
                                                .relative()
                                                .map(|card| {
                                                    #[cfg(all(feature = "integration-test", target_os = "macos"))]
                                                    let card = card.child(grid::native_probe(index));
                                                    card
                                                })
                                                .debug_selector(move || {
                                                    format!("settings-theme-row-{index}")
                                                })
                                                .h(look.at(96.))
                                                .flex_1()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .pt(look.at(6.) - px(ring))
                                                .px(look.at(6.) - px(ring))
                                                .pb(look.at(8.) - px(ring))
                                                .flex()
                                                .flex_col()
                                                .rounded(look.at(10.))
                                                .bg(look.card_fill())
                                                .border(px(ring))
                                                .border_color(if chosen { look.accent() } else { look.line() })
                                                .when(choice.scope.editable(), |el| el.cursor_pointer())
                                                .hover(move |el| el.bg(hover))
                                                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                                                    // Leaving is handled by the grid, so moving between
                                                    // cards never flashes the saved theme.
                                                    if *hovered {
                                                        this.hover_theme(Some(index), cx);
                                                    }
                                                }))
                                                 .child(this.grid_thumbnail(&choice, index))
                                                .child(
                                                    div()
                                                        .debug_selector(move || {
                                                            format!("settings-theme-name-{index}")
                                                        })
                                                        .mt(look.at(5.))
                                                        .min_w_0()
                                                        .truncate()
                                                        .text_size(px(look.scale().name))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(look.title())
                                                        .child(name.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .debug_selector(move || {
                                                            format!("settings-theme-source-{index}")
                                                        })
                                                        .text_size(px(look.scale().detail))
                                                        .text_color(look.muted())
                                                        .truncate()
                                                        .child(format!("{source}{}", if draft { " / Draft" } else if saved { " / Saved" } else { "" })),
                                                )
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        let focus = this
                                                            .themes
                                                            .search
                                                            .read(cx)
                                                            .focus
                                                            .clone();
                                                        window.focus(&focus, cx);
                                                        this.select_settings_theme(index, cx);
                                                    },
                                                )).into_any_element()
                                        })))
                                        .collect()
                                }),
                            )
                            .track_scroll(&browser.scroll)
                            .h_full()
                            .w_full(),
                        )
                    }),
            )
            .when_some(browser.catalog_error.as_ref(), |el, error| {
                el.child(
                    div()
                        .debug_selector(|| "settings-theme-catalog-error".into())
                        .child(format!(
                            "Catalog discovery failed: {error}. Built-ins remain available."
                        ))
                        .child(
                            button("theme-retry-catalog", "Retry discovery".into())
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.discover_settings_themes(cx)),
                                ),
                        ),
                )
            })
            .when_some(browser.preview_error.as_ref(), |el, error| {
                el.child(
                    div()
                        .debug_selector(|| "settings-theme-preview-error".into())
                        .child(format!(
                            "Preview unavailable: {error}. Keeping the last valid preview."
                        ))
                        .child(
                            button("theme-retry-preview", "Retry preview".into()).on_click(
                                cx.listener(|this, _, _, cx| this.request_theme_preview(cx)),
                            ),
                        ),
                )
            })
    }
}
