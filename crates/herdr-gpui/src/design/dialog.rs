//! The look dialogs, the forms a menu turns into, and the window's notices
//! share since the redesign's fourth phase. It builds on the menus' panel
//! and the Settings controls, and adds what only a dialog has: a heading
//! with a cross, a body of text, wells, notes and errors, and a footer with
//! its keys and buttons. Behaviour stays with each dialog; these pieces only
//! paint, so no dialog draws a header or a button of its own again.

use super::{ButtonKind, ButtonSize, Header, MenuLook, SettingsLook, TypeScale};
use crate::{
    config::{FontConfig, Theme},
    fonts::StyledFont,
};
use gpui::{
    AnyElement, BoxShadow, Div, FontWeight, IntoElement, Pixels, Rgba, SharedString, Stateful,
    Styled, div, hsla, point, prelude::*, px, rgb, svg,
};

/// The `[ui]` size the measures below are given at; they grow with the face.
const BASE_SIZE: f32 = 12.;
const HEADER_MIN: f32 = 52.;
const HEADER_PADDING_Y: f32 = 8.;
const HEADER_PADDING_LEFT: f32 = 16.;
const HEADER_PADDING_RIGHT: f32 = 10.;
const HEADER_GAP: f32 = 10.;
const HEADER_ICON: f32 = 16.;
const SUB_ICON: f32 = 11.;
const SUB_GAP: f32 = 5.;
const BODY_TOP: f32 = 14.;
const BODY_SIDE: f32 = 16.;
const BODY_GAP: f32 = 12.;
/// A field's label sits this close above it.
const LABEL_GAP: f32 = 6.;
/// The column the captions of a form's fields share.
const CAPTION_WIDTH: f32 = 62.;
const CAPTION_ROW_GAP: f32 = 10.;
const CAPTION_GAP: f32 = 12.;
const RADIUS: f32 = 8.;
const WELL_PADDING_X: f32 = 10.;
const WELL_PADDING_Y: f32 = 7.;
/// A path's or a command's text in the terminal's face.
const WELL_MONO: f32 = 12.;
const NOTE_ICON: f32 = 15.;
const NOTE_GAP: f32 = 8.;
const ERROR_PADDING_X: f32 = 10.;
const ERROR_PADDING_Y: f32 = 8.;
/// How much of the danger red an error's box takes.
const ERROR_WASH: f32 = 0.13;
const CHECKBOX: f32 = 16.;
const CHECKBOX_GAP: f32 = 9.;
const FOOTER_PADDING_Y: f32 = 10.;
const FOOTER_PADDING_LEFT: f32 = 16.;
const FOOTER_PADDING_RIGHT: f32 = 12.;
const FOOTER_GAP: f32 = 8.;
const HINT_GAP: f32 = 12.;
const HINT_INNER_GAP: f32 = 5.;
const FORM_TOP: f32 = 6.;
const FORM_SIDE: f32 = 10.;
const FORM_BOTTOM: f32 = 4.;
const FORM_GAP: f32 = 10.;
const FORM_FOOTER_TOP: f32 = 8.;
const FORM_FOOTER_SIDE: f32 = 4.;
const FORM_FOOTER_BOTTOM: f32 = 2.;
const FORM_FOOTER_GAP: f32 = 6.;
/// A notice's leading tile, its corners and its icon.
const TILE: f32 = 28.;
const TILE_ICON: f32 = 15.;
/// How much of its colour a notice's tile takes.
const TILE_WASH: f32 = 0.14;
/// The flash's shadow: shorter than a panel's, as it sits on the terminal.
const FLASH_SHADOW_Y: f32 = 10.;
const FLASH_SHADOW_BLUR: f32 = 26.;
const SHADOW_DARK: f32 = 0.45;
const SHADOW_LIGHT: f32 = 0.18;

/// What a note says by its icon and colour: something to know, something
/// that may hurt, or something already done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoteTone {
    Info,
    Warning,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DialogLook {
    menu: MenuLook,
    settings: SettingsLook,
    panel: Rgba,
    line: Rgba,
    well: Rgba,
    title: Rgba,
    text: Rgba,
    muted: Rgba,
    danger: Rgba,
    good: Rgba,
    /// The checkbox's set fill and ink, and its clear ring.
    checkbox: ((u32, u32), u32),
    shadow: f32,
    scale: TypeScale,
    zoom: f32,
}

impl DialogLook {
    pub(crate) fn of(theme: &Theme, font: &FontConfig, light: bool) -> Self {
        Self {
            menu: MenuLook::of(theme, font, light),
            settings: SettingsLook::of(theme, font),
            panel: rgb(theme.background),
            line: rgb(theme.active),
            well: rgb(theme.surface),
            title: rgb(theme.foreground),
            text: rgb(theme.subtext()),
            muted: rgb(theme.muted),
            danger: crate::menu::danger(theme),
            good: rgb(theme.ink(theme.palette[2])),
            checkbox: (crate::toggles::colors(theme, true), theme.muted),
            shadow: if light { SHADOW_LIGHT } else { SHADOW_DARK },
            scale: TypeScale::of(font),
            zoom: font.size / BASE_SIZE,
        }
    }

    pub(crate) fn menu(&self) -> MenuLook {
        self.menu
    }

    pub(crate) fn settings(&self) -> SettingsLook {
        self.settings
    }

    /// `measure`, given at `[ui] size = 12`, at this face's size.
    pub(crate) fn at(&self, measure: f32) -> Pixels {
        px(measure * self.zoom)
    }

    /// A well's fill. The well itself is [`Self::well`].
    pub(crate) fn well_fill(&self) -> Rgba {
        self.well
    }

    pub(crate) fn line(&self) -> Rgba {
        self.line
    }

    pub(crate) fn error_ink(&self) -> Rgba {
        self.danger
    }

    pub(crate) fn error_fill(&self) -> Rgba {
        Rgba {
            a: ERROR_WASH,
            ..self.danger
        }
    }

    pub(crate) fn note_ink(&self, tone: NoteTone) -> Rgba {
        match tone {
            NoteTone::Info => self.muted,
            NoteTone::Warning => self.danger,
            NoteTone::Done => self.good,
        }
    }

    /// A centred dialog's heading: its icon (red over a destructive action),
    /// its title, what it is about, and `close` at the right. A hairline
    /// parts it from the body.
    pub(crate) fn header(&self, header: Header<'_>, close: Stateful<Div>) -> Div {
        let Header {
            selector,
            icon,
            title: (title_selector, title),
            sub,
            danger,
        } = header;
        div()
            .debug_selector(move || selector.into())
            .flex_none()
            .min_h(self.at(HEADER_MIN))
            .py(self.at(HEADER_PADDING_Y))
            .pl(self.at(HEADER_PADDING_LEFT))
            .pr(self.at(HEADER_PADDING_RIGHT))
            .flex()
            .items_center()
            .gap(self.at(HEADER_GAP))
            .border_b_1()
            .border_color(self.line)
            .child(
                svg()
                    .path(icon)
                    .size(self.at(HEADER_ICON))
                    .flex_none()
                    .text_color(self.menu.header_icon(danger))
                    .debug_selector(move || format!("{selector}-icon")),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .debug_selector(move || title_selector.into())
                            .min_w_0()
                            .truncate()
                            .text_size(px(self.scale.name + 1.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(self.title)
                            .child(crate::sidebar::label_text(title)),
                    )
                    .when_some(sub, |titles, (icon, sub_selector, text)| {
                        titles.child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(self.at(SUB_GAP))
                                .text_size(px(self.scale.secondary))
                                .text_color(self.muted)
                                .child(
                                    svg()
                                        .path(icon)
                                        .size(self.at(SUB_ICON))
                                        .flex_none()
                                        .text_color(self.muted),
                                )
                                .child(
                                    div()
                                        .debug_selector(move || sub_selector.into())
                                        .min_w_0()
                                        .truncate()
                                        .child(crate::sidebar::label_text(text)),
                                ),
                        )
                    }),
            )
            .child(close)
    }

    /// A centred dialog's body: the column its texts, fields and notes sit in.
    pub(crate) fn body(&self, selector: &'static str) -> Div {
        div()
            .debug_selector(move || selector.into())
            .min_w_0()
            .flex()
            .flex_col()
            .gap(self.at(BODY_GAP))
            .pt(self.at(BODY_TOP))
            .px(self.at(BODY_SIDE))
            .pb(self.at(BODY_SIDE))
    }

    /// A paragraph of the body.
    pub(crate) fn lead(&self, selector: &'static str, text: impl Into<SharedString>) -> Div {
        div()
            .debug_selector(move || selector.into())
            .min_w_0()
            .text_size(px(self.scale.name))
            .text_color(self.text)
            .child(text.into())
    }

    /// A field's label: the column it heads, so the caller adds the field
    /// as its child and the two keep together.
    pub(crate) fn label(&self, selector: &'static str, text: &'static str) -> Div {
        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(self.at(LABEL_GAP))
            .child(
                div()
                    .debug_selector(move || selector.into())
                    .text_size(px(self.scale.secondary))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(self.text)
                    .child(text),
            )
    }

    /// Fields in rows behind captions that share one column, so every field
    /// starts at the same edge.
    pub(crate) fn captions(
        &self,
        selector: &'static str,
        rows: impl IntoIterator<Item = (&'static str, AnyElement)>,
    ) -> Div {
        div()
            .debug_selector(move || selector.into())
            .min_w_0()
            .flex()
            .flex_col()
            .gap(self.at(CAPTION_ROW_GAP))
            .children(rows.into_iter().map(|(caption, field)| {
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(self.at(CAPTION_GAP))
                    .child(
                        div()
                            .flex_none()
                            .w(self.at(CAPTION_WIDTH))
                            .text_size(px(self.scale.name - 0.5))
                            .text_color(self.muted)
                            .child(caption),
                    )
                    .child(div().flex_1().min_w_0().child(field))
            }))
    }

    /// A path, a URL, a command or a summary set apart from the text: in the
    /// terminal's face when `face` is given, else in the interface's. The
    /// caller adds the text, which wraps where it must.
    pub(crate) fn well(&self, selector: String, face: Option<&FontConfig>) -> Div {
        let well = div()
            .debug_selector(move || selector)
            .min_w_0()
            .rounded(self.at(RADIUS))
            .bg(self.well)
            .border_1()
            .border_color(self.line)
            .px(self.at(WELL_PADDING_X))
            .py(self.at(WELL_PADDING_Y))
            .text_color(self.title);
        match face {
            Some(face) => well.text_font(face).text_size(self.at(WELL_MONO)),
            None => well.text_size(px(self.scale.name)),
        }
    }

    /// A line to know about, behind its tone's icon and in its colour.
    pub(crate) fn note(
        &self,
        selector: String,
        tone: NoteTone,
        text: impl Into<SharedString>,
    ) -> Div {
        let icon = match tone {
            NoteTone::Info => "icons/info.svg",
            NoteTone::Warning => "icons/alert.svg",
            NoteTone::Done => "icons/check.svg",
        };
        let ink = self.note_ink(tone);
        div()
            .debug_selector(move || selector)
            .min_w_0()
            .flex()
            .items_start()
            .gap(self.at(NOTE_GAP))
            .text_size(px(self.scale.name - 0.5))
            .text_color(ink)
            .child(self.mark(icon, ink))
            .child(div().flex_1().min_w_0().child(text.into()))
    }

    /// What went wrong, in full, in a box washed with the danger red.
    pub(crate) fn error(&self, selector: String, text: impl Into<SharedString>) -> Div {
        div()
            .debug_selector(move || selector)
            .min_w_0()
            .flex()
            .items_start()
            .gap(self.at(NOTE_GAP))
            .px(self.at(ERROR_PADDING_X))
            .py(self.at(ERROR_PADDING_Y))
            .rounded(self.at(RADIUS))
            .bg(self.error_fill())
            .text_size(px(self.scale.name - 0.5))
            .text_color(self.danger)
            .child(self.mark("icons/alert.svg", self.danger))
            .child(div().flex_1().min_w_0().child(text.into()))
    }

    /// A note's or an error's icon, lowered to sit on its first line.
    fn mark(&self, icon: &'static str, ink: Rgba) -> gpui::Svg {
        svg()
            .path(icon)
            .size(self.at(NOTE_ICON))
            .flex_none()
            .mt(self.at(1.))
            .text_color(ink)
    }

    /// The app's checkbox with its label; the whole line takes the click,
    /// which the caller adds.
    pub(crate) fn checkbox(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
    ) -> Stateful<Div> {
        let (colours, ring) = self.checkbox;
        div()
            .id(id)
            .debug_selector(move || id.into())
            .min_w_0()
            .flex()
            .items_center()
            .gap(self.at(CHECKBOX_GAP))
            .cursor_pointer()
            .text_size(px(self.scale.name))
            .text_color(self.text)
            .child(crate::toggles::checkbox_in(
                colours,
                ring,
                CHECKBOX * self.zoom,
                checked,
            ))
            .child(label)
    }

    /// A centred dialog's footer: its keys at the left, its buttons at the
    /// right, on the palette footer's fill.
    pub(crate) fn footer(&self, selector: &'static str) -> Footer {
        Footer {
            look: *self,
            selector,
            compact: false,
            hints: Vec::new(),
            buttons: Vec::new(),
        }
    }

    /// The body of a form a menu turns into, inside the menu's panel.
    pub(crate) fn form_body(&self, selector: &'static str) -> Div {
        div()
            .debug_selector(move || selector.into())
            .min_w_0()
            .flex()
            .flex_col()
            .gap(self.at(FORM_GAP))
            .pt(self.at(FORM_TOP))
            .px(self.at(FORM_SIDE))
            .pb(self.at(FORM_BOTTOM))
    }

    /// A form's footer: small buttons at the right under a hairline, and no
    /// keys.
    pub(crate) fn form_footer(&self, selector: &'static str) -> Footer {
        Footer {
            compact: true,
            ..self.footer(selector)
        }
    }

    /// A dialog's button, 30 tall; its selector is its `id`.
    pub(crate) fn button(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        kind: ButtonKind,
        enabled: bool,
    ) -> Stateful<Div> {
        self.settings
            .button(id, id.into(), label, kind, ButtonSize::Regular, enabled)
    }

    /// A form's button, 26 tall; its selector is its `id`.
    pub(crate) fn small_button(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        kind: ButtonKind,
        enabled: bool,
    ) -> Stateful<Div> {
        self.settings
            .button(id, id.into(), label, kind, ButtonSize::Small, enabled)
    }

    /// A notice's leading tile: its icon in `colour`, on a wash of it.
    pub(crate) fn tile(&self, selector: String, icon: &'static str, colour: Rgba) -> Div {
        div()
            .debug_selector(move || selector)
            .flex_none()
            .size(self.at(TILE))
            .rounded(self.at(RADIUS))
            .bg(Rgba {
                a: TILE_WASH,
                ..colour
            })
            .flex()
            .items_center()
            .justify_center()
            .child(svg().path(icon).size(self.at(TILE_ICON)).text_color(colour))
    }

    /// Dresses the flash: the panel's fill and hairline, smaller corners,
    /// and a shorter shadow than a panel's.
    pub(crate) fn flash<E: Styled>(&self, element: E) -> E {
        element
            .bg(self.panel)
            .border_1()
            .border_color(self.line)
            .rounded(self.at(RADIUS))
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., self.shadow),
                offset: point(px(0.), self.at(FLASH_SHADOW_Y)),
                blur_radius: self.at(FLASH_SHADOW_BLUR),
                spread_radius: px(0.),
                inset: false,
            }])
    }
}

/// A dialog's or a form's footer, built from its keys and its buttons.
pub(crate) struct Footer {
    look: DialogLook,
    selector: &'static str,
    compact: bool,
    hints: Vec<(&'static str, &'static str)>,
    buttons: Vec<AnyElement>,
}

impl Footer {
    /// A key the dialog answers to and what it does: "Esc", "cancel". A
    /// form's footer shows none.
    pub(crate) fn hint(mut self, key: &'static str, word: &'static str) -> Self {
        if !self.compact {
            self.hints.push((key, word));
        }
        self
    }

    pub(crate) fn button(mut self, button: impl IntoElement) -> Self {
        self.buttons.push(button.into_any_element());
        self
    }
}

impl IntoElement for Footer {
    type Element = Div;

    fn into_element(self) -> Div {
        let Self {
            look,
            selector,
            compact,
            hints,
            buttons,
        } = self;
        let buttons = buttons
            .into_iter()
            .map(|button| div().flex_none().child(button));
        if compact {
            return div()
                .debug_selector(move || selector.into())
                .flex_none()
                .flex()
                .items_center()
                .justify_end()
                .gap(look.at(FORM_FOOTER_GAP))
                .mt(look.at(FORM_BOTTOM))
                .pt(look.at(FORM_FOOTER_TOP))
                .px(look.at(FORM_FOOTER_SIDE))
                .pb(look.at(FORM_FOOTER_BOTTOM))
                .border_t_1()
                .border_color(look.line)
                .children(buttons);
        }
        // The keys give way before the buttons: they shrink and clip, the
        // buttons never do.
        let hints = div()
            .debug_selector(move || format!("{selector}-hints"))
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(look.at(HINT_GAP))
            .text_size(px(look.scale.detail))
            .text_color(look.muted)
            .children(hints.into_iter().enumerate().map(|(index, (key, word))| {
                div()
                    .debug_selector(move || format!("{selector}-hint-{index}"))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.at(HINT_INNER_GAP))
                    .child(
                        look.menu
                            .keycap(format!("{selector}-hint-{index}-key"), key),
                    )
                    .child(word)
            }));
        div()
            .debug_selector(move || selector.into())
            .flex_none()
            .flex()
            .items_center()
            .gap(look.at(FOOTER_GAP))
            .py(look.at(FOOTER_PADDING_Y))
            .pl(look.at(FOOTER_PADDING_LEFT))
            .pr(look.at(FOOTER_PADDING_RIGHT))
            .border_t_1()
            .border_color(look.line)
            .bg(look.well)
            .child(hints)
            .children(buttons)
    }
}

#[cfg(test)]
mod tests;
