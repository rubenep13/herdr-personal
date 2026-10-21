//! The look Settings shares since the redesign's third phase: its measures,
//! which grow with the `[ui]` face like the menus', its colours from the
//! theme, and the pieces its pages and the pickers it opens build from (a
//! card, a button, a chip, a field), so that no page draws one of its own.
//! Behaviour stays with each page; these pieces only paint.

use super::TypeScale;
use crate::config::{FontConfig, Theme, mix};
use gpui::{
    Div, ElementId, FontWeight, Pixels, Rgba, SharedString, Stateful, div, prelude::*, px, rgb,
};

/// The `[ui]` size the measures below are given at; they grow with the face.
const BASE_SIZE: f32 = 12.;
/// A row of the section list.
const NAV_ROW: f32 = 32.;
/// A control's box: segments, a dropdown, a number, a field, a button.
const CONTROL: f32 = 30.;
/// One segment inside its box.
const SEGMENT: f32 = 26.;
/// A setting's row at its least.
const ROW_MIN: f32 = 48.;
const SMALL_BUTTON: f32 = 26.;
/// The switch's track, as the app draws it elsewhere.
const SWITCH: f32 = 20.;
const CONTROL_RADIUS: f32 = 8.;
const CARD_RADIUS: f32 = 10.;
/// How far the rule between a card's rows keeps from its sides.
const RULE_INSET: f32 = 14.;
const BUTTON_PADDING: f32 = 12.;
const SMALL_BUTTON_PADDING: f32 = 10.;
const BUTTON_GAP: f32 = 7.;
const CHIP_HEIGHT: f32 = 17.;
const CHIP_TEXT: f32 = 11.;
const CHIP_RADIUS: f32 = 5.;
const CHIP_PADDING: f32 = 6.;
const FIELD_PADDING: f32 = 10.;
const FIELD_GAP: f32 = 8.;
/// A card's fill: this much of the hairline over the window.
const CARD_PERCENT: u32 = 30;
/// A hovered row of the section list, as the tree's rows.
const NAV_HOVER_PERCENT: u32 = 60;
/// A hovered row or theme card on the page: more of the hairline than a card.
const HOVER_PERCENT: u32 = 60;
const SHARED_PERCENT: u32 = 14;
const LOCAL_PERCENT: u32 = 70;
/// A status chip's wash of its own colour.
const TONE_PERCENT: u32 = 13;
/// A hovered button leans this far toward the text.
const BUTTON_HOVER_PERCENT: u32 = 10;
const PRIMARY_HOVER_PERCENT: u32 = 15;

/// A button's weight: the page's plain action, the one it leads with, or
/// one that destroys something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ButtonKind {
    Normal,
    Primary,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ButtonSize {
    Regular,
    Small,
}

/// What a chip says by its colour: nothing, all is well, a warning, or where
/// a setting is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Neutral,
    Good,
    Warn,
    Shared,
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SettingsLook {
    page: Rgba,
    nav: Rgba,
    nav_hover: Rgba,
    /// A hovered row or card on the page.
    hover: Rgba,
    card: Rgba,
    /// A control's box and the footer, as the palette's footer.
    well: Rgba,
    line: Rgba,
    button_hover: Rgba,
    title: Rgba,
    text: Rgba,
    muted: Rgba,
    accent: Rgba,
    accent_hover: Rgba,
    on_accent: Rgba,
    shared_fill: Rgba,
    local_fill: Rgba,
    danger: Rgba,
    danger_hover: Rgba,
    on_danger: Rgba,
    good: Rgba,
    good_fill: Rgba,
    warn: Rgba,
    warn_fill: Rgba,
    /// The switch's colours off and on, as [`crate::toggles::colors`] gives
    /// them for the theme.
    switch: [(u32, u32); 2],
    scale: TypeScale,
    /// How far the face is from the size the measures are given at.
    zoom: f32,
}

impl SettingsLook {
    pub(crate) fn of(theme: &Theme, font: &FontConfig) -> Self {
        let good = theme.ink(theme.palette[2]);
        let danger = crate::menu::danger_value(theme);
        // The theme has no orange: warnings take the ANSI yellow.
        let warn = theme.ink(theme.palette[3]);
        Self {
            page: rgb(theme.background),
            nav: rgb(theme.sidebar_background()),
            nav_hover: rgb(mix(
                theme.sidebar_background(),
                theme.active,
                NAV_HOVER_PERCENT,
            )),
            hover: rgb(mix(theme.background, theme.active, HOVER_PERCENT)),
            card: rgb(mix(theme.background, theme.active, CARD_PERCENT)),
            well: rgb(theme.surface),
            line: rgb(theme.active),
            button_hover: rgb(mix(theme.active, theme.foreground, BUTTON_HOVER_PERCENT)),
            title: rgb(theme.foreground),
            text: rgb(theme.subtext()),
            muted: rgb(theme.muted),
            accent: rgb(theme.primary()),
            accent_hover: rgb(mix(
                theme.primary(),
                theme.foreground,
                PRIMARY_HOVER_PERCENT,
            )),
            on_accent: rgb(crate::toggles::ink_on(theme, theme.primary())),
            shared_fill: rgb(mix(theme.background, theme.primary(), SHARED_PERCENT)),
            local_fill: rgb(mix(theme.background, theme.active, LOCAL_PERCENT)),
            danger: rgb(danger),
            danger_hover: rgb(mix(danger, theme.foreground, PRIMARY_HOVER_PERCENT)),
            on_danger: rgb(crate::toggles::ink_on(theme, danger)),
            good: rgb(good),
            good_fill: rgb(mix(theme.background, good, TONE_PERCENT)),
            warn: rgb(warn),
            warn_fill: rgb(mix(theme.background, warn, TONE_PERCENT)),
            switch: [
                crate::toggles::colors(theme, false),
                crate::toggles::colors(theme, true),
            ],
            scale: TypeScale::of(font),
            zoom: font.size / BASE_SIZE,
        }
    }

    /// `measure`, given at `[ui] size = 12`, at this face's size.
    pub(crate) fn at(&self, measure: f32) -> Pixels {
        px(measure * self.zoom)
    }

    pub(crate) fn scale(&self) -> TypeScale {
        self.scale
    }

    pub(crate) fn nav_row_height(&self) -> f32 {
        NAV_ROW * self.zoom
    }

    pub(crate) fn control_height(&self) -> f32 {
        CONTROL * self.zoom
    }

    pub(crate) fn segment_height(&self) -> f32 {
        SEGMENT * self.zoom
    }

    pub(crate) fn row_min_height(&self) -> f32 {
        ROW_MIN * self.zoom
    }

    pub(crate) fn small_button_height(&self) -> f32 {
        SMALL_BUTTON * self.zoom
    }

    pub(crate) fn page(&self) -> Rgba {
        self.page
    }

    pub(crate) fn nav(&self) -> Rgba {
        self.nav
    }

    pub(crate) fn nav_hover(&self) -> Rgba {
        self.nav_hover
    }

    pub(crate) fn hover(&self) -> Rgba {
        self.hover
    }

    /// A card's fill. The card itself is [`Self::card`].
    pub(crate) fn card_fill(&self) -> Rgba {
        self.card
    }

    pub(crate) fn well(&self) -> Rgba {
        self.well
    }

    /// The hairline, as the redesign's other phases draw it.
    pub(crate) fn line(&self) -> Rgba {
        self.line
    }

    /// A step past the hairline toward the text: a hovered button's fill, a
    /// hovered control's ring.
    pub(crate) fn raised(&self) -> Rgba {
        self.button_hover
    }

    /// A selected row's fill.
    pub(crate) fn selection(&self) -> Rgba {
        self.line
    }

    pub(crate) fn title(&self) -> Rgba {
        self.title
    }

    pub(crate) fn text(&self) -> Rgba {
        self.text
    }

    pub(crate) fn muted(&self) -> Rgba {
        self.muted
    }

    pub(crate) fn accent(&self) -> Rgba {
        self.accent
    }

    /// Read by tests: the chip draws from the field.
    #[cfg(test)]
    pub(crate) fn shared_fill(&self) -> Rgba {
        self.shared_fill
    }

    /// Read by tests: the chip draws from the field.
    #[cfg(test)]
    pub(crate) fn local_fill(&self) -> Rgba {
        self.local_fill
    }

    /// A field whose text does not parse.
    pub(crate) fn danger(&self) -> Rgba {
        self.danger
    }

    /// Read by tests: the chip draws from the field.
    #[cfg(test)]
    pub(crate) fn good(&self) -> Rgba {
        self.good
    }

    pub(crate) fn warn(&self) -> Rgba {
        self.warn
    }

    /// A switch, the app's shared one at this face's size.
    pub(crate) fn switch(&self, on: bool) -> Div {
        crate::toggles::switch_in(self.switch[usize::from(on)], SWITCH * self.zoom, on)
    }

    /// A section's card: its rows go in a column, split by [`Self::rule`].
    pub(crate) fn card(&self, selector: String) -> Div {
        div()
            .debug_selector(move || selector)
            .flex()
            .flex_col()
            .bg(self.card)
            .border_1()
            .border_color(self.line)
            .rounded(self.at(CARD_RADIUS))
    }

    /// The line between two rows of a card.
    pub(crate) fn rule(&self) -> Div {
        div()
            .flex_none()
            .h(px(1.))
            .mx(self.at(RULE_INSET))
            .bg(self.line)
    }

    /// A button with its label. The caller adds its handler; a disabled one
    /// fades and takes no pointer.
    pub(crate) fn button(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        label: impl Into<SharedString>,
        kind: ButtonKind,
        size: ButtonSize,
        enabled: bool,
    ) -> Stateful<Div> {
        let (height, padding, text) = match size {
            ButtonSize::Regular => (self.control_height(), BUTTON_PADDING, self.scale.name),
            ButtonSize::Small => (
                self.small_button_height(),
                SMALL_BUTTON_PADDING,
                self.scale.name - 0.5,
            ),
        };
        let (fill, hover, ink) = self.button_colours(kind);
        div()
            .id(id)
            .debug_selector(move || selector)
            .flex_none()
            .h(px(height))
            .px(self.at(padding))
            .flex()
            .items_center()
            .justify_center()
            .gap(self.at(BUTTON_GAP))
            .rounded(self.at(CONTROL_RADIUS))
            .bg(fill)
            .text_color(ink)
            .text_size(px(text))
            .whitespace_nowrap()
            .when(kind != ButtonKind::Normal, |button| {
                button.font_weight(FontWeight::MEDIUM)
            })
            .child(label.into())
            .when(enabled, |button| {
                button
                    .cursor_pointer()
                    .hover(move |button| button.bg(hover))
            })
            .when(!enabled, |button| button.opacity(0.5))
    }

    /// A button's fill, its hovered fill and its text: the hairline for a
    /// plain one, the accent for the one a page leads with, the danger red
    /// for one that destroys something, with whatever reads on the coloured
    /// ones (the window's background on the Catppuccin themes).
    pub(super) fn button_colours(&self, kind: ButtonKind) -> (Rgba, Rgba, Rgba) {
        match kind {
            ButtonKind::Normal => (self.line, self.button_hover, self.title),
            ButtonKind::Primary => (self.accent, self.accent_hover, self.on_accent),
            ButtonKind::Danger => (self.danger, self.danger_hover, self.on_danger),
        }
    }

    /// A small label in its tone's colours: a status, or where a setting is
    /// kept.
    pub(crate) fn chip(&self, selector: String, text: impl Into<SharedString>, tone: Tone) -> Div {
        let (ink, fill) = match tone {
            Tone::Neutral => (self.muted, self.well),
            Tone::Good => (self.good, self.good_fill),
            Tone::Warn => (self.warn, self.warn_fill),
            Tone::Shared => (self.accent, self.shared_fill),
            Tone::Local => (self.text, self.local_fill),
        };
        div()
            .debug_selector(move || selector)
            .flex_none()
            .h(self.at(CHIP_HEIGHT))
            .px(self.at(CHIP_PADDING))
            .flex()
            .items_center()
            .rounded(self.at(CHIP_RADIUS))
            .bg(fill)
            .text_color(ink)
            .text_size(self.at(CHIP_TEXT))
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .child(text.into())
    }

    /// A text field's frame: the caller adds its icon, its input and any
    /// count. Focused, its ring takes the accent.
    pub(crate) fn field(&self, selector: String, focused: bool) -> Div {
        div()
            .debug_selector(move || selector)
            .flex_none()
            .h(self.at(CONTROL))
            .px(self.at(FIELD_PADDING))
            .flex()
            .items_center()
            .gap(self.at(FIELD_GAP))
            .rounded(self.at(CONTROL_RADIUS))
            .bg(self.well)
            .border_1()
            .border_color(if focused { self.accent } else { self.line })
            .text_size(px(self.scale.name))
            .text_color(self.title)
    }
}

#[cfg(test)]
mod tests;
