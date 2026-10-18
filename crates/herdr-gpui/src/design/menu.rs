//! The look menus and the palette share since the redesign's second phase:
//! its measures, its colours from the theme, and the pieces every menu builds
//! its rows from, so that no menu draws a row of its own and they cannot
//! drift apart again. Behaviour (selection, hover, clicks, keys) stays with
//! each menu; these pieces only paint.

use super::{SELECTION_RADIUS, TypeScale};
use crate::config::{FontConfig, Theme, corners};
use gpui::{
    BoxShadow, Div, ElementId, FontFeatures, FontWeight, IntoElement, Pixels, Rgba, SharedString,
    Stateful, Svg, div, hsla, point, prelude::*, px, rgb, svg,
};

/// The `[ui]` size the measures below are given at; they grow with the face.
const BASE_SIZE: f32 = 12.;
/// A menu's row, and a list's two-line row.
const MENU_ROW: f32 = 32.;
const LIST_ROW: f32 = 44.;
const ROW_PADDING: f32 = 10.;
/// Between a row's leading icon and its label.
const ROW_GAP: f32 = 10.;
const ICON: f32 = 16.;
const SUBMENU_ICON: f32 = 14.;
const SEPARATOR_INSET: f32 = 10.;
const SEPARATOR_GAP: f32 = 5.;
const HEADER_ICON: f32 = 16.;
const HEADER_SUB_ICON: f32 = 11.;
/// The header's second line starts under its title, past the icon.
const HEADER_SUB_INSET: f32 = 24.;
/// A picker's query row and its search icon, and its row of tabs.
const QUERY_HEIGHT: f32 = 56.;
const SEARCH_ICON: f32 = 18.;
const TABS_HEIGHT: f32 = 40.;
/// How far a separator leans from the hairline toward the quiet text colour.
const RULE_PERCENT: u32 = 40;
/// How far a key chip's fill leans from the selection toward the text.
const KEY_PERCENT: u32 = 12;
/// A key chip's height and least width.
const KEYCAP: f32 = 17.;
/// How much of the danger red fills a selected destructive row.
const DANGER_WASH: f32 = 0.14;
/// The panel's shadow: lighter on a light theme, where black reads heavier.
const SHADOW_DARK: f32 = 0.45;
const SHADOW_LIGHT: f32 = 0.18;

/// What leads a row: nothing (the slot stays, so labels line up), an icon
/// and the selector naming it, or a check mark in the accent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Leading {
    Empty,
    Icon(&'static str, String),
    Check,
}

/// How a row reads: selected by the pointer or the keys, unavailable, or a
/// destructive action.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ItemState {
    pub(crate) selected: bool,
    pub(crate) disabled: bool,
    pub(crate) danger: bool,
}

/// A menu's heading: an icon and a bold title, and optionally a second
/// line with its own small icon. Each text carries its selector.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Header<'a> {
    pub(crate) selector: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) title: (&'static str, &'a str),
    pub(crate) sub: Option<(&'static str, &'static str, &'a str)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MenuLook {
    panel: Rgba,
    line: Rgba,
    /// A separator between groups of rows: the hairline would vanish on a
    /// dark theme between rows, so it leans toward the quiet text colour.
    rule: Rgba,
    /// A key chip's fill: a step lighter than the selection, so a chip still
    /// shows on a selected row.
    key: Rgba,
    selection: Rgba,
    text: Rgba,
    selected_text: Rgba,
    muted: Rgba,
    accent: Rgba,
    danger: Rgba,
    shadow: f32,
    scale: TypeScale,
    /// How far the face is from the size the measures are given at.
    zoom: f32,
}

impl MenuLook {
    pub(crate) fn of(theme: &Theme, font: &FontConfig, light: bool) -> Self {
        Self {
            panel: rgb(theme.background),
            line: rgb(theme.active),
            rule: rgb(crate::config::mix(theme.active, theme.muted, RULE_PERCENT)),
            key: rgb(crate::config::mix(
                theme.active,
                theme.foreground,
                KEY_PERCENT,
            )),
            selection: rgb(theme.active),
            text: rgb(theme.subtext()),
            selected_text: rgb(theme.foreground),
            muted: rgb(theme.muted),
            accent: rgb(theme.primary()),
            danger: crate::menu::danger(theme),
            shadow: if light { SHADOW_LIGHT } else { SHADOW_DARK },
            scale: TypeScale::of(font),
            zoom: font.size / BASE_SIZE,
        }
    }

    /// `measure`, given at `[ui] size = 12`, at this face's size.
    pub(crate) fn at(&self, measure: f32) -> Pixels {
        px(measure * self.zoom)
    }

    pub(crate) fn row_height(&self) -> f32 {
        MENU_ROW * self.zoom
    }

    pub(crate) fn list_row_height(&self) -> f32 {
        LIST_ROW * self.zoom
    }

    pub(crate) fn scale(&self) -> TypeScale {
        self.scale
    }

    pub(crate) fn accent(&self) -> Rgba {
        self.accent
    }

    pub(crate) fn muted(&self) -> Rgba {
        self.muted
    }

    pub(crate) fn line(&self) -> Rgba {
        self.line
    }

    pub(crate) fn rule(&self) -> Rgba {
        self.rule
    }

    /// A row's fill: none at rest, the selection colour when selected, and a
    /// wash of red when a selected row is destructive.
    pub(crate) fn fill(&self, state: ItemState) -> Option<Rgba> {
        if !state.selected || state.disabled {
            return None;
        }
        Some(if state.danger {
            Rgba {
                a: DANGER_WASH,
                ..self.danger
            }
        } else {
            self.selection
        })
    }

    /// A row's text colour.
    pub(crate) fn ink(&self, state: ItemState) -> Rgba {
        if state.disabled {
            self.muted
        } else if state.danger {
            self.danger
        } else if state.selected {
            self.selected_text
        } else {
            self.text
        }
    }

    #[cfg(test)]
    pub(super) fn shadow_opacity(&self) -> f32 {
        self.shadow
    }

    /// Dresses a popup's panel: the window's background, a hairline, the
    /// panel's corners and a soft shadow. Its padding is the caller's.
    pub(crate) fn panel<E: Styled>(&self, panel: E) -> E {
        panel
            .bg(self.panel)
            .border_1()
            .border_color(self.line)
            .rounded(px(corners::PANEL))
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., self.shadow),
                offset: point(px(0.), px(18.)),
                blur_radius: px(44.),
                spread_radius: px(0.),
                inset: false,
            }])
    }

    /// The frame both kinds of row share: rounded, filled while selected,
    /// tinted by its state. The pointer selects a row (each menu moves its
    /// selection on hover), so there is no separate hover style to drift
    /// from it.
    fn frame(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        height: f32,
        state: ItemState,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .debug_selector(move || selector)
            .flex_none()
            .h(px(height))
            .px(self.at(ROW_PADDING))
            .flex()
            .items_center()
            .gap(self.at(ROW_GAP))
            .rounded(px(SELECTION_RADIUS))
            .text_size(px(self.scale.name))
            .text_color(self.ink(state))
            .when_some(self.fill(state), |row, fill| row.bg(fill))
            .when(!state.disabled, |row| row.cursor_pointer())
    }

    /// A menu's one-line row, its leading slot filled. The caller adds the
    /// label, anything at its end, and its handlers.
    pub(crate) fn row(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        leading: Leading,
        state: ItemState,
    ) -> Stateful<Div> {
        self.frame(id, selector, self.row_height(), state)
            .child(self.leading(leading, state))
    }

    /// A list's two-line row, such as the palette's. The caller adds its tile
    /// and its lines.
    pub(crate) fn list_row(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        state: ItemState,
    ) -> Stateful<Div> {
        self.frame(id, selector, self.list_row_height(), state)
    }

    fn leading(&self, leading: Leading, state: ItemState) -> Div {
        let size = self.at(ICON);
        let color = if state.danger || state.disabled {
            self.ink(state)
        } else if state.selected {
            self.selected_text
        } else {
            self.muted
        };
        let slot = div()
            .size(size)
            .flex_none()
            .flex()
            .items_center()
            .justify_center();
        match leading {
            Leading::Empty => slot,
            Leading::Icon(path, selector) => slot.child(
                svg()
                    .path(path)
                    .size(size)
                    .flex_none()
                    .text_color(color)
                    .when(state.disabled, |icon| icon.opacity(0.6))
                    .debug_selector(move || selector),
            ),
            Leading::Check => slot.child(
                svg()
                    .path("icons/check.svg")
                    .size(size)
                    .flex_none()
                    .text_color(self.accent),
            ),
        }
    }

    /// A row's label: it takes the room left and ends in an ellipsis.
    pub(crate) fn label(&self, selector: String, text: impl IntoElement) -> Div {
        div()
            .debug_selector(move || selector)
            .flex_1()
            .min_w_0()
            .truncate()
            .child(text)
    }

    /// Quiet text at a row's end, such as a shortcut or a process.
    pub(crate) fn detail(&self, selector: String, text: impl IntoElement) -> Div {
        div()
            .debug_selector(move || selector)
            .flex_none()
            .text_size(px(self.scale.secondary))
            .text_color(self.muted)
            .font_features(FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)])))
            .child(text)
    }

    /// The chevron that ends a row opening a submenu.
    pub(crate) fn submenu(&self, selector: String) -> Svg {
        svg()
            .path("icons/chevron-right.svg")
            .size(self.at(SUBMENU_ICON))
            .flex_none()
            .text_color(self.muted)
            .debug_selector(move || selector)
    }

    pub(crate) fn separator(&self) -> Div {
        div()
            .flex_none()
            .h(px(1.))
            .mx(self.at(SEPARATOR_INSET))
            .my(self.at(SEPARATOR_GAP))
            .bg(self.rule)
    }

    /// One key of a shortcut, drawn as a key.
    pub(crate) fn keycap(&self, selector: String, key: impl Into<SharedString>) -> Div {
        div()
            .debug_selector(move || selector)
            .flex_none()
            .h(self.at(KEYCAP))
            .min_w(self.at(KEYCAP))
            .px(self.at(4.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(self.at(4.))
            .bg(self.key)
            .text_color(self.text)
            .text_size(px(self.scale.detail - 1.))
            .child(key.into())
    }

    /// A shortcut's keys as chips: the row named `selector`, each key
    /// `{selector}-key-{n}`.
    pub(crate) fn keys(&self, selector: &str, shortcut: &str) -> Div {
        let row = selector.to_owned();
        div()
            .debug_selector(move || row)
            .flex()
            .flex_none()
            .items_center()
            .gap(self.at(3.))
            .children(
                keycaps(shortcut)
                    .enumerate()
                    .map(|(index, key)| self.keycap(format!("{selector}-key-{index}"), key)),
            )
    }

    /// A small heading over a group of rows.
    pub(crate) fn section(&self, selector: String, text: impl Into<SharedString>) -> Div {
        div()
            .debug_selector(move || selector)
            .flex_none()
            .px(self.at(ROW_PADDING))
            .pt(self.at(8.))
            .pb(self.at(4.))
            .text_size(px(self.scale.detail))
            .font_weight(FontWeight::MEDIUM)
            .text_color(self.muted)
            .child(text.into())
    }

    /// A picker's query row: a search icon, then the field.
    pub(crate) fn query(&self, icon_selector: &'static str, field: impl IntoElement) -> Div {
        div()
            .flex_none()
            .h(self.at(QUERY_HEIGHT))
            .px(self.at(18.))
            .flex()
            .items_center()
            .gap(self.at(12.))
            .border_b_1()
            .border_color(self.line)
            .child(
                svg()
                    .path("icons/search.svg")
                    .size(self.at(SEARCH_ICON))
                    .flex_none()
                    .text_color(self.muted)
                    .debug_selector(move || icon_selector.into()),
            )
            .child(div().flex_1().min_w_0().child(field))
    }

    /// The row a picker's tabs sit in, ruled underneath.
    pub(crate) fn tabs(&self, selector: &'static str) -> Div {
        div()
            .debug_selector(move || selector.into())
            .flex_none()
            .h(self.at(TABS_HEIGHT))
            .px(self.at(10.))
            .flex()
            .border_b_1()
            .border_color(self.line)
    }

    /// One of a picker's tabs, as the subheader's: the chosen one in the
    /// text colour over a line of the accent across the row's rule.
    pub(crate) fn tab(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        underline: &'static str,
        on: bool,
    ) -> Stateful<Div> {
        let hover = self.text;
        div()
            .id(id)
            .debug_selector(move || selector)
            .relative()
            .h_full()
            .px(self.at(10.))
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(px(self.scale.name))
            .when(on, |tab| {
                tab.text_color(self.selected_text)
                    .font_weight(FontWeight::MEDIUM)
                    .child(
                        div()
                            .debug_selector(move || underline.into())
                            .absolute()
                            .left(self.at(6.))
                            .right(self.at(6.))
                            .bottom(px(-1.))
                            .h(px(2.))
                            .rounded(px(2.))
                            .bg(self.accent),
                    )
            })
            .when(!on, |tab| {
                tab.text_color(self.muted)
                    .hover(move |tab| tab.text_color(hover))
            })
    }

    pub(crate) fn header(&self, header: Header<'_>) -> Div {
        let Header {
            selector,
            icon,
            title: (title_selector, title),
            sub,
        } = header;
        div()
            .debug_selector(move || selector.into())
            .flex_none()
            .flex()
            .flex_col()
            .gap(self.at(3.))
            .px(self.at(ROW_PADDING))
            .pt(self.at(6.))
            .pb(self.at(10.))
            .mb(self.at(6.))
            .border_b_1()
            .border_color(self.line)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(self.at(8.))
                    .min_w_0()
                    .child(
                        svg()
                            .path(icon)
                            .size(self.at(HEADER_ICON))
                            .flex_none()
                            .text_color(self.muted)
                            .debug_selector(move || format!("{selector}-icon")),
                    )
                    .child(
                        div()
                            .debug_selector(move || title_selector.into())
                            .min_w_0()
                            .truncate()
                            .text_size(px(self.scale.name))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(self.selected_text)
                            .child(crate::sidebar::label_text(title)),
                    ),
            )
            .when_some(sub, |header, (icon, sub_selector, text)| {
                header.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(self.at(5.))
                        .pl(self.at(HEADER_SUB_INSET))
                        .min_w_0()
                        .text_size(px(self.scale.secondary))
                        .text_color(self.muted)
                        .child(
                            svg()
                                .path(icon)
                                .size(self.at(HEADER_SUB_ICON))
                                .flex_none()
                                .text_color(self.muted)
                                .debug_selector(move || format!("{selector}-sub-icon")),
                        )
                        .child(
                            div()
                                .debug_selector(move || sub_selector.into())
                                .min_w_0()
                                .truncate()
                                .child(crate::sidebar::label_text(text)),
                        ),
                )
            })
    }
}

/// Moves a menu's selection with the pointer: onto `index` while it is
/// hovered, and off it again when the pointer leaves, so the row reads as it
/// did before. Returns whether the selection changed.
pub(crate) fn menu_hover(selected: &mut Option<usize>, index: usize, hovered: bool) -> bool {
    let next = if hovered {
        Some(index)
    } else if *selected == Some(index) {
        None
    } else {
        return false;
    };
    let changed = *selected != next;
    *selected = next;
    changed
}

/// The keys of a shortcut as its chips name them: `ctrl-shift-t` is Ctrl,
/// Shift, T, and a chord's keystrokes follow one another.
pub(crate) fn keycaps(shortcut: &str) -> impl Iterator<Item = String> + '_ {
    shortcut
        .split(' ')
        .flat_map(|keystroke| {
            let (modifiers, key) = match keystroke.strip_suffix('-') {
                Some(modifiers) if modifiers.is_empty() || modifiers.ends_with('-') => {
                    (modifiers, "-")
                }
                _ => keystroke.rsplit_once('-').unwrap_or(("", keystroke)),
            };
            modifiers
                .split('-')
                .filter(|modifier| !modifier.is_empty())
                .chain(std::iter::once(key))
        })
        .map(|key| {
            let mut chars = key.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase())
                .into_iter()
                .chain(chars)
                .collect()
        })
        .filter(|key: &String| !key.is_empty())
}

#[cfg(test)]
mod tests;
