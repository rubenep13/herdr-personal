//! The one tooltip the app shows, since the redesign's fifth phase: the
//! menus' panel in small, with the window's fill, a hairline and a softer
//! shadow, so a hint looks the same over the status bar, the sidebar or a
//! view. [`super::MenuLook::tooltip`] builds it for `.tooltip(…)`.

#[cfg(test)]
use super::MenuLook;
use gpui::{
    BoxShadow, Context, IntoElement, Pixels, Render, Rgba, SharedString, Window, div, hsla, point,
    prelude::*, px,
};

/// Its corners and padding at `[ui] size = 12`.
const RADIUS: f32 = 7.;
const PADDING_X: f32 = 9.;
const PADDING_Y: f32 = 5.;
/// Its shadow: the panel's, shorter.
const SHADOW_Y: f32 = 8.;
const SHADOW_BLUR: f32 = 22.;

#[derive(Clone, Debug)]
pub(crate) struct Tooltip {
    text: SharedString,
    fill: Rgba,
    line: Rgba,
    ink: Rgba,
    shadow: f32,
    text_size: f32,
    zoom: f32,
}

impl Tooltip {
    pub(super) fn new(
        text: SharedString,
        (fill, line, ink): (Rgba, Rgba, Rgba),
        shadow: f32,
        text_size: f32,
        zoom: f32,
    ) -> Self {
        Self {
            text,
            fill,
            line,
            ink,
            shadow,
            text_size,
            zoom,
        }
    }

    fn at(&self, measure: f32) -> Pixels {
        px(measure * self.zoom)
    }
}

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .debug_selector(|| "tooltip".into())
            .px(self.at(PADDING_X))
            .py(self.at(PADDING_Y))
            .rounded(self.at(RADIUS))
            .bg(self.fill)
            .border_1()
            .border_color(self.line)
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., self.shadow),
                offset: point(px(0.), self.at(SHADOW_Y)),
                blur_radius: self.at(SHADOW_BLUR),
                spread_radius: px(0.),
                inset: false,
            }])
            .text_size(px(self.text_size))
            .text_color(self.ink)
            .child(self.text.clone())
    }
}

#[cfg(test)]
mod tests;
