//! The palette paints prepared entries; input and daemon operations live elsewhere.

use super::{Action, AgentFilter, Filter, Palette, hints, preview};
use crate::{HerdrWindow, design::ItemState};
use gpui::{prelude::*, *};

/// The narrowest window, less its margins, that fits Go To's list and
/// preview side by side; a narrower one shows the list alone.
const PREVIEW_MIN_WIDTH: f32 = 840.;

/// The palette's measures at `[ui] size = 12`, in the redesigned look.
const FOOTER_HEIGHT: f32 = 36.;
/// How far the list's rows keep from the panel's edges.
const LIST_INSET: f32 = 6.;
/// How far each level of Go To's nesting steps in.
const NEST: f32 = 18.;
const STATUS_DOT: f32 = 7.;

/// How much larger than the interface text the palette's query is drawn.
const SEARCH_SCALE: f32 = 1.4;

/// The interface font, enlarged for the palette's query field.
pub(super) fn search_font(ui: &crate::config::FontConfig) -> crate::config::FontConfig {
    crate::config::FontConfig {
        size: ui.size * SEARCH_SCALE,
        ..ui.clone()
    }
}

impl HerdrWindow {
    /// The query is the palette's focus, as in Raycast: a large frameless
    /// field behind a search icon spans the top, with the filters, as the
    /// subheader's underlined tabs, and the count quieter beneath it.
    fn palette_header(&self, palette: &Palette, cx: &mut Context<Self>) -> Div {
        let look = self.menu_look();
        let scale = look.scale();
        div()
            .flex_none()
            .child(look.query("palette-search-icon", palette.search.clone()))
            .child(
                look.tabs("palette-filters")
                    .children(Filter::ALL.into_iter().map(|filter| {
                        look.tab(
                            ("palette-filter", filter as usize),
                            format!("palette-filter-{}", filter.label().to_lowercase()),
                            "palette-filter-underline",
                            filter == palette.filter,
                        )
                        .child(filter.label())
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.set_palette_filter(filter, cx);
                                if let Some(palette) = &this.menu.palette {
                                    let focus = palette.search.read(cx).focus.clone();
                                    window.focus(&focus, cx);
                                }
                            },
                        ))
                    }))
                    .child(
                        div()
                            .ml_auto()
                            .flex_none()
                            .self_center()
                            .pr(look.at(6.))
                            .text_size(px(scale.detail))
                            .text_color(look.muted())
                            .child(format!(
                                "{} of {}",
                                palette.filtered.len(),
                                palette.entries.len()
                            )),
                    ),
            )
            .when(palette.filter == Filter::Navigation, |header| {
                header.child(self.palette_agent_filters(palette, cx))
            })
    }

    /// Go To's status chips, each dotted in its status's colour; Alt with the
    /// chip's navigator letter picks one from the keyboard.
    fn palette_agent_filters(&self, palette: &Palette, cx: &mut Context<Self>) -> Div {
        let theme = &self.theme;
        let look = self.menu_look();
        let indicators = self.palette_indicators();
        let wash = rgb(crate::config::mix(theme.background, theme.active, 50));
        div()
            .flex()
            .flex_wrap()
            .gap(look.at(6.))
            .px(look.at(16.))
            .pt(look.at(10.))
            .pb(look.at(4.))
            .text_size(px(look.scale().secondary))
            .children(AgentFilter::ALL.into_iter().map(|filter| {
                let on = filter == palette.agent_filter;
                let name = filter.label().to_lowercase().replace(' ', "-");
                let dot = format!("palette-agent-filter-dot-{name}");
                div()
                    .id(("palette-agent-filter", filter as usize))
                    .debug_selector(move || format!("palette-agent-filter-{name}"))
                    .flex()
                    .items_center()
                    .gap(look.at(6.))
                    .px(look.at(9.))
                    .py(look.at(3.))
                    .rounded(look.at(7.))
                    .border_1()
                    .border_color(rgb(theme.active))
                    .cursor_pointer()
                    .when(on, |chip| {
                        chip.bg(rgb(theme.active)).text_color(rgb(theme.foreground))
                    })
                    .when(!on, |chip| {
                        chip.text_color(rgb(theme.subtext()))
                            .hover(move |chip| chip.bg(wash))
                    })
                    .when_some(filter.status(), |chip, status| {
                        chip.child(
                            div()
                                .debug_selector(move || dot)
                                .size(look.at(STATUS_DOT))
                                .flex_none()
                                .rounded_full()
                                .bg(rgb(crate::sidebar::badge_color(status, indicators, theme))),
                        )
                    })
                    .child(filter.label())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.set_palette_agent_filter(filter, cx);
                        if let Some(palette) = &this.menu.palette {
                            let focus = palette.search.read(cx).focus.clone();
                            window.focus(&focus, cx);
                        }
                    }))
            }))
    }

    /// The status colours and marks the sidebar uses, for Go To's agents.
    fn palette_indicators(&self) -> crate::sidebar::Indicators {
        crate::sidebar::Indicators::new(
            self.settings.shared.as_ref(),
            self.config.status_icons,
            self.theme_light,
            &self.theme,
        )
    }

    /// The mark on a row's tile and its colour: an agent's own, in its
    /// brand's colour, else what the row goes to or does.
    fn palette_mark(&self, entry: &super::Entry) -> (&'static str, u32) {
        if let Some(icon) = entry.agent {
            return (icon.path(), crate::sidebar::logo_color(icon, &self.theme));
        }
        let mark = match &entry.action {
            Action::Go { target, .. } => match target {
                crate::NavigationTarget::Workspace(_) => "icons/folder.svg",
                crate::NavigationTarget::Tab(_) => "icons/tab.svg",
                crate::NavigationTarget::Pane(_) => "icons/terminal.svg",
            },
            Action::Native(_) | Action::Configured(..) => "icons/command.svg",
            Action::Project(_) | Action::NewProject(_) => "icons/folder.svg",
            Action::Note { .. } => "icons/note.svg",
        };
        (mark, self.theme.muted)
    }

    fn palette_row(&self, palette: &Palette, index: usize, cx: &mut Context<Self>) -> Div {
        let hit = &palette.filtered[index];
        let entry = &palette.entries[hit.index];
        // Ranked results need not keep entry order, so only an empty query nests.
        let depth = if palette.query.trim().is_empty() {
            hit.depth
        } else {
            0
        };
        let action = entry.action.clone();
        let theme = &self.theme;
        let look = self.menu_look();
        let scale = look.scale();
        let indicators = self.palette_indicators();
        let state = ItemState {
            selected: index == palette.selected,
            ..ItemState::default()
        };
        // Matched characters were found while ranking; painting only styles them.
        let matched = HighlightStyle {
            color: Some(look.accent().into()),
            font_weight: Some(FontWeight::BOLD),
            ..Default::default()
        };
        let styled = |text: &SharedString, ranges: &[std::ops::Range<usize>]| {
            StyledText::new(text.clone())
                .with_highlights(ranges.iter().map(|range| (range.clone(), matched)))
        };
        let (mark, mark_color) = self.palette_mark(entry);
        // A GUI action's detail is its shortcut, drawn as keys at the row's end.
        let shortcut = matches!(entry.action, Action::Native(_)) && !entry.detail.is_empty();
        let row = look
            .list_row(index, format!("palette-row-{index}"), state)
            .flex_1()
            .min_w_0()
            .child(crate::sidebar::tile(
                format!("palette-tile-{index}"),
                mark,
                mark_color,
                state.selected,
                theme,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .truncate()
                            .font_weight(FontWeight::MEDIUM)
                            .child(styled(&entry.label, &hit.highlights.label)),
                    )
                    .when(!shortcut, |lines| {
                        lines.child(
                            div()
                                .truncate()
                                .text_size(px(scale.detail))
                                .text_color(look.muted())
                                .child(styled(&entry.detail, &hit.highlights.detail)),
                        )
                    }),
            )
            .when(shortcut, |row| {
                row.child(look.keys(&format!("palette-shortcut-{index}"), &entry.detail))
            })
            .map(|row| match entry.status {
                // An agent ends as the sidebar's agent row does: its state's
                // badge, and the state's words in a pill of its colour.
                Some(status) => row
                    .child(crate::sidebar::badge(
                        format!("palette-state-{index}"),
                        status,
                        indicators,
                        theme,
                    ))
                    .when(!entry.badge.is_empty(), |row| {
                        row.child(crate::sidebar::pill(
                            format!("palette-pill-{index}"),
                            &entry.badge,
                            crate::sidebar::badge_color(status, indicators, theme),
                            scale.detail - 1.,
                        ))
                    }),
                None => row.when(!entry.badge.is_empty(), |row| {
                    row.child(
                        div()
                            .debug_selector(move || format!("palette-badge-{index}"))
                            .flex_none()
                            .px(look.at(7.))
                            .py(look.at(2.))
                            .rounded(look.at(5.))
                            .bg(rgb(theme.surface))
                            .text_size(px(scale.detail - 0.5))
                            .text_color(look.muted())
                            .child(entry.badge.clone()),
                    )
                }),
            })
            // As in the menus, the row under the pointer is the selected one.
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered
                    && let Some(palette) = &mut this.menu.palette
                    && palette.selected != index
                {
                    palette.restore = None;
                    palette.selected = index;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.activate_palette(action.clone(), window, cx)
            }));
        // A nested row steps in and hangs on the tree's guide line.
        div().w_full().flex().px(look.at(LIST_INSET)).child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .relative()
                .when(depth > 0, |nest| {
                    nest.ml(look.at(NEST * depth as f32)).child(
                        div()
                            .debug_selector(move || format!("palette-guide-{index}"))
                            .absolute()
                            .left(-look.at(NEST / 2.))
                            .top(look.at(4.))
                            .bottom(look.at(4.))
                            .w(px(1.))
                            .bg(look.line()),
                    )
                })
                .child(row),
        )
    }

    /// The footer's hints, each key drawn as a key.
    fn palette_footer(&self, palette: &Palette) -> Div {
        let theme = &self.theme;
        let look = self.menu_look();
        let footer = div()
            .debug_selector(|| "palette-status".into())
            .flex_none()
            .h(look.at(FOOTER_HEIGHT))
            .px(look.at(16.))
            .flex()
            .items_center()
            .gap(look.at(14.))
            .overflow_hidden()
            .border_t_1()
            .border_color(look.line())
            .bg(rgb(theme.surface))
            .text_size(px(look.scale().detail))
            .text_color(look.muted());
        if palette.busy() {
            return footer.child("Opening local project... Esc to dismiss.");
        }
        let hints = if palette.filter == Filter::Navigation {
            hints::GO_TO_HINTS
        } else {
            hints::HINTS
        };
        let mut footer = footer;
        let mut first = 0;
        for hint in hints {
            let keys =
                hint.keys.iter().enumerate().map(|(offset, key)| {
                    look.keycap(format!("palette-key-{}", first + offset), *key)
                });
            footer = footer.child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(look.at(4.))
                    .children(keys)
                    .child(div().pl(look.at(2.)).child(hint.text)),
            );
            first += hint.keys.len();
        }
        footer
    }

    /// Whether Go To shows its preview: it is toggled on, and the window has
    /// room for it beside the list.
    pub(crate) fn palette_previews(&self, viewport: Size<Pixels>) -> bool {
        self.menu.palette_preview
            && self
                .menu
                .palette
                .as_ref()
                .is_some_and(|palette| palette.filter == Filter::Navigation)
            && f32::from(viewport.width) - 32. >= PREVIEW_MIN_WIDTH
    }

    /// The preview of the selected Go To row, from what its host's
    /// connection already holds.
    pub(super) fn palette_preview(&self, palette: &Palette) -> Option<preview::Preview> {
        let Action::Go {
            endpoint,
            boot,
            target,
        } = &palette.selected_entry()?.action
        else {
            return None;
        };
        let index = self.endpoints.iter().position(|e| e.id == *endpoint)?;
        let live = if index == self.selected_endpoint {
            &self.live
        } else {
            &self.endpoints[index].live
        };
        preview::preview(
            live.snapshot.as_deref(),
            live.surface.as_ref(),
            &self.endpoints[index].last_seen,
            boot,
            target.as_deref(),
        )
    }

    fn render_palette_preview(&self, palette: &Palette) -> Div {
        let theme = &self.theme;
        let pane = div()
            .debug_selector(|| "palette-preview".into())
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .border_l_1()
            .border_color(rgb(theme.active));
        match self.palette_preview(palette) {
            Some(preview::Preview::Screen {
                surface,
                rect,
                seen_at,
            }) => {
                let terminal = &self.config.terminal;
                let look = preview::Look {
                    font: terminal.font(),
                    size: terminal.size,
                    line_ratio: terminal.line_height() / terminal.size,
                    cell_ratio: self.cell_width / terminal.size,
                    theme: theme.clone(),
                };
                pane.relative()
                    .bg(rgb(theme.background))
                    .child(preview::paint_screen(
                        surface,
                        rect,
                        palette.preview_painter.clone(),
                        look,
                    ))
                    .when_some(seen_at, |pane, at| {
                        let age = crate::checkpoint::age(
                            i64::try_from(at).unwrap_or(i64::MAX),
                            i64::try_from(super::frecency::now()).unwrap_or(i64::MAX),
                        );
                        pane.child(
                            div()
                                .debug_selector(|| "palette-preview-age".into())
                                .absolute()
                                .top(px(8.))
                                .right(px(8.))
                                .px(px(8.))
                                .py(px(2.))
                                .rounded(px(4.))
                                .bg(rgb(theme.surface))
                                .border_1()
                                .border_color(rgb(theme.active))
                                .text_color(rgb(theme.muted))
                                .child(format!("Last seen {age}")),
                        )
                    })
            }
            Some(preview::Preview::Facts { title, facts }) => pane
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .children(facts.into_iter().map(|(name, value)| {
                    div()
                        .flex()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex_none()
                                .w(px(self.config.ui.size * 6.))
                                .text_color(rgb(theme.muted))
                                .child(name),
                        )
                        .child(div().flex_1().min_w_0().truncate().child(value))
                }))
                .child(
                    div()
                        .pt(px(10.))
                        .text_color(rgb(theme.muted))
                        .child("Not on screen. Press Enter to go there and watch it live."),
                ),
            None => pane,
        }
    }

    pub(crate) fn render_palette(&self, viewport: Size<Pixels>, cx: &mut Context<Self>) -> Div {
        let Some(palette) = &self.menu.palette else {
            return div();
        };
        let theme = &self.theme;
        let previews = self.palette_previews(viewport);
        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .child(self.palette_header(palette, cx))
            .when(palette.loading_projects, |panel| {
                panel.child(
                    div()
                        .flex_none()
                        .px(px(16.))
                        .py(px(8.))
                        .text_color(rgb(theme.muted))
                        .child("Loading local projects..."),
                )
            })
            .when(!palette.projects.errors.is_empty(), |panel| {
                panel.child(
                    div()
                        .id("palette-project-errors")
                        .max_h(px(72.))
                        .overflow_y_scroll()
                        .flex_none()
                        .px(px(16.))
                        .py(px(8.))
                        .text_color(rgb(theme.muted))
                        .child(
                            palette
                                .projects
                                .errors
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join("\n"),
                        ),
                )
            })
            .when_some(palette.error.clone(), |panel, error| {
                panel.child(
                    div()
                        .id("palette-error")
                        .max_h(px(90.))
                        .overflow_y_scroll()
                        .flex_none()
                        .p(px(12.))
                        .text_color(rgb(theme.foreground))
                        .bg(rgb(theme.active))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .when(palette.filtered.is_empty(), |body| {
                        body.child(
                            div()
                                .flex_1()
                                .p(px(16.))
                                .text_color(rgb(theme.muted))
                                .child("No matching results. Try a shorter search."),
                        )
                    })
                    .when(!palette.filtered.is_empty(), |body| {
                        body.child(
                            uniform_list(
                                "palette-results",
                                palette.filtered.len(),
                                cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                                    let Some(palette) = &this.menu.palette else {
                                        return Vec::new();
                                    };
                                    range
                                        .map(|index| this.palette_row(palette, index, cx))
                                        .collect()
                                }),
                            )
                            .track_scroll(&palette.scroll)
                            .flex_1()
                            .min_w_0(),
                        )
                    })
                    .when(previews, |body| {
                        body.child(self.render_palette_preview(palette))
                    }),
            )
            .child(self.palette_footer(palette))
    }
}
