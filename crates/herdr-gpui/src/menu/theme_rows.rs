//! What the Color Scheme picker and a host's terminal theme picker share: a
//! theme's row, with its colour strip, and the footer with the keys they
//! answer to.
use crate::design::{ItemState, Leading, MenuLook};
use gpui::{Div, SharedString, Stateful, div, prelude::*, px, relative, svg};

/// One theme: its strip (or another leading mark), its name, and "Current"
/// with a check on the one in use.
#[allow(clippy::too_many_arguments)]
pub(crate) fn theme_row(
    look: &MenuLook,
    index: usize,
    selector: String,
    label: SharedString,
    label_selector: String,
    leading: Leading,
    selected: bool,
    current: bool,
) -> Stateful<Div> {
    let check = format!("{selector}-check");
    let detail = format!("{selector}-current");
    look.row(
        index,
        selector,
        leading,
        ItemState {
            selected,
            ..ItemState::default()
        },
    )
    .w_full()
    .child(look.label(label_selector, label))
    .when(current, |row| {
        row.child(look.detail(detail, "Current")).child(
            svg()
                .path("icons/check.svg")
                .size(look.at(16.))
                .flex_none()
                .text_color(look.accent())
                .debug_selector(move || check),
        )
    })
}

/// The keys a theme picker answers to.
const KEYS: &[(&[&str], &str)] = &[
    (&["\u{2191}", "\u{2193}"], "preview"),
    (&["\u{21b5}"], "save"),
    (&["Esc"], "close"),
];

/// A picker's footer, as the palette's: a message while one stands, else
/// the keys and then `note`.
pub(crate) fn picker_footer(
    look: &MenuLook,
    selector: &'static str,
    message: Option<SharedString>,
    note: Option<&'static str>,
) -> Div {
    let footer = div()
        .debug_selector(move || selector.into())
        .flex_none()
        .h(look.at(FOOTER_HEIGHT))
        .px(look.at(16.))
        .flex()
        .items_center()
        .gap(look.at(14.))
        .overflow_hidden()
        .whitespace_nowrap()
        .border_t_1()
        .border_color(look.line())
        .bg(look.well())
        .text_size(px(look.scale().detail))
        .text_color(look.muted());
    if let Some(message) = message {
        return footer.child(
            div()
                .debug_selector(move || format!("{selector}-message"))
                .min_w_0()
                .truncate()
                .child(message),
        );
    }
    let mut first = 0;
    let mut footer = footer;
    for (keys, text) in KEYS {
        let caps = keys
            .iter()
            .enumerate()
            .map(|(offset, key)| look.keycap(format!("{selector}-key-{}", first + offset), *key));
        footer = footer.child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(look.at(4.))
                .children(caps)
                .child(div().pl(look.at(2.)).child(*text)),
        );
        first += keys.len();
    }
    footer.when_some(note, |footer, note| {
        footer.child(
            div()
                .debug_selector(move || format!("{selector}-note"))
                .ml_auto()
                .min_w_0()
                .truncate()
                .child(note),
        )
    })
}

/// An error in full above a picker's footer, as the palette shows one: it
/// wraps, and past a few lines it scrolls. The list above gives it room, so
/// no row moves.
pub(crate) fn picker_error(
    look: &MenuLook,
    selector: &'static str,
    error: SharedString,
) -> Stateful<Div> {
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .flex_none()
        .max_h(look.at(ERROR_HEIGHT))
        .overflow_y_scroll()
        .px(look.at(16.))
        .py(look.at(8.))
        .border_t_1()
        .border_color(look.line())
        .text_size(px(look.scale().secondary))
        .line_height(relative(1.5))
        .text_color(look.danger())
        .child(error)
}

/// The footer's height, as the palette's, and how tall an error may grow
/// before it scrolls, at the default `[ui]` size.
const FOOTER_HEIGHT: f32 = 36.;
const ERROR_HEIGHT: f32 = 96.;
