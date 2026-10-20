//! Host colour row: a curated swatch strip, a "none" swatch, a hex field and
//! a reset glyph. Pure rendering; the page owns the colour and the saves.
use crate::{
    config::{FontConfig, parse_hex},
    design::SettingsLook,
    fonts::StyledFont,
    search_input::SearchInput,
};
use gpui::{prelude::*, *};
use std::rc::Rc;

/// Curated host tints, muted enough to sit behind sidebar text.
pub(in crate::settings_window) const SWATCHES: [u32; 11] = [
    0x3a4a6b, 0x2f5d5a, 0x4a5b2f, 0x6b5a2f, 0x6b3a3a, 0x5a3a6b, 0x2f4a6b, 0x4a4a4a, 0x7a4f2a,
    0x2a6b4f, 0x6b2a5a,
];

const SWATCH_SIZE: f32 = 18.;
const SWATCH_RADIUS: f32 = 5.;
/// The chosen swatch's accent ring, and the gap it keeps from the swatch.
const RING: f32 = 2.;
const RING_GAP: f32 = 2.;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::settings_window) enum HostColorChange {
    Set(u32),
    Clear,
}

/// `#rgb` or `#rrggbb` with or without the `#`, any case, padded by spaces.
pub(in crate::settings_window) fn parse_hex_input(text: &str) -> Option<u32> {
    let text = text.trim().to_lowercase();
    if text.starts_with('#') {
        parse_hex(&text)
    } else {
        parse_hex(&format!("#{text}"))
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::settings_window) fn swatch_row(
    look: &SettingsLook,
    host: &str,
    index: usize,
    current: Option<u32>,
    mono: &FontConfig,
    hint: &'static str,
    hex_input: Option<(Entity<SearchInput>, bool)>,
    on_change: impl Fn(HostColorChange, &mut Window, &mut App) + 'static,
    on_edit_hex: impl Fn(&mut Window, &mut App) + 'static,
) -> Div {
    let on_change = Rc::new(on_change);
    // Every swatch sits in a slot as wide as the chosen one's ring, so
    // choosing one moves nothing.
    let slot = |selected: bool, swatch: Stateful<Div>| {
        div()
            .flex_none()
            .p(look.at(RING_GAP))
            .rounded(look.at(SWATCH_RADIUS + RING + RING_GAP))
            .border(look.at(RING))
            .border_color(if selected {
                look.accent()
            } else {
                transparent_black().into()
            })
            .child(swatch)
    };
    let none = {
        let on_change = on_change.clone();
        slot(
            current.is_none(),
            div()
                .id(SharedString::from(format!("host-colour-{index}-none")))
                .debug_selector(move || format!("host-colour-{index}-none"))
                .size(look.at(SWATCH_SIZE))
                .rounded(look.at(SWATCH_RADIUS))
                .border_1()
                .border_dashed()
                .border_color(look.muted())
                .cursor_pointer()
                .on_click(move |_, window, cx| on_change(HostColorChange::Clear, window, cx)),
        )
    };
    let swatches = SWATCHES.iter().enumerate().map(|(k, &color)| {
        let on_change = on_change.clone();
        slot(
            current == Some(color),
            div()
                .id(SharedString::from(format!(
                    "host-colour-{index}-swatch-{k}"
                )))
                .debug_selector(move || format!("host-colour-{index}-swatch-{k}"))
                .size(look.at(SWATCH_SIZE))
                .rounded(look.at(SWATCH_RADIUS))
                .bg(rgb(color))
                .border_1()
                .border_color(look.line())
                .cursor_pointer()
                .on_click(move |_, window, cx| on_change(HostColorChange::Set(color), window, cx)),
        )
    });
    let dot = div()
        .size(px(10.))
        .flex_none()
        .rounded_full()
        .map(|dot| match current {
            Some(color) => dot.bg(rgb(color)),
            None => dot.border_1().border_dashed().border_color(look.muted()),
        });
    let hex = div()
        .id(SharedString::from(format!("host-colour-{index}-hex")))
        .debug_selector(move || format!("host-colour-{index}-hex"))
        .w(look.at(92.))
        .flex_none()
        .text_size(px(look.scale().secondary))
        .map(|hex| match hex_input {
            // A typed colour that does not parse keeps the field, outlined.
            Some((input, invalid)) => hex
                .rounded(look.at(6.))
                .border_1()
                .border_color(if invalid { look.danger() } else { look.line() })
                .child(input),
            None => {
                let (label, color) = match current {
                    Some(color) => (format!("#{color:06x}"), look.title()),
                    None => ("none".to_owned(), look.muted()),
                };
                hex.text_font(mono)
                    .text_color(color)
                    .cursor_pointer()
                    .on_click(move |_, window, cx| on_edit_hex(window, cx))
                    .child(label)
            }
        });
    let reset = div().w(look.at(20.)).flex_none().map(|slot| {
        if current.is_none() {
            return slot;
        }
        let on_change = on_change.clone();
        slot.child(
            div()
                .id(SharedString::from(format!("host-colour-{index}-reset")))
                .debug_selector(move || format!("host-colour-{index}-reset"))
                .text_color(look.muted())
                .cursor_pointer()
                .on_click(move |_, window, cx| on_change(HostColorChange::Clear, window, cx))
                .child("⟲"),
        )
    });
    div()
        .debug_selector(move || format!("host-colour-{index}"))
        .flex()
        .items_center()
        .gap(look.at(10.))
        .min_h(px(look.row_min_height()))
        .px(look.at(14.))
        .py(look.at(9.))
        .text_size(px(look.scale().name))
        .text_color(look.title())
        .child(
            div()
                .debug_selector(move || format!("host-colour-{index}-label"))
                .w(look.at(120.))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.at(8.))
                .child(dot)
                .child(div().min_w_0().truncate().child(host.to_owned())),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(look.at(4.))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(look.at(2.))
                        .child(none)
                        .children(swatches),
                )
                .when(!hint.is_empty(), |col| {
                    col.child(
                        div()
                            .text_size(px(look.scale().detail))
                            .text_color(look.muted())
                            .child(hint),
                    )
                }),
        )
        .child(hex)
        .child(reset)
}

#[cfg(test)]
mod tests;
