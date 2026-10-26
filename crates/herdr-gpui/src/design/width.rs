//! The width a view was given at its last layout, for views that change
//! shape with it: they read it while rendering and draw again only when a
//! new width crosses one of their cuts.

use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

/// Shared between a view's state and the probe in its last frame. Zero
/// until the first frame lays the view out.
#[derive(Clone, Debug, Default)]
pub(crate) struct Width(Rc<Cell<f32>>);

impl Width {
    pub(crate) fn get(&self) -> f32 {
        self.0.get()
    }

    /// An element filling its parent that records the parent's width as it
    /// lays out, and asks for another frame when that width moves across
    /// one of `cuts`, so the view follows on the next frame.
    pub(crate) fn probe<const N: usize>(&self, cuts: [f32; N]) -> impl IntoElement {
        self.canvas(move |width, now| width.record(now, &cuts))
    }

    /// Like [`Self::probe`] for a view whose shape follows its width
    /// smoothly: any move asks for another frame.
    pub(crate) fn follow(&self) -> impl IntoElement {
        self.canvas(Self::record_any)
    }

    fn canvas(&self, moved: impl Fn(&Self, f32) -> bool + 'static) -> impl IntoElement {
        let width = self.clone();
        canvas(
            move |bounds, window, _| {
                // A refresh while the window draws is dropped: ask for the
                // next frame and refresh there.
                if moved(&width, f32::from(bounds.size.width)) {
                    window.on_next_frame(|window, _| window.refresh());
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0()
    }

    /// Keeps `now` and says whether the view must draw again: the first
    /// width always counts, since the view guessed before it.
    fn record(&self, now: f32, cuts: &[f32]) -> bool {
        let before = self.0.replace(now);
        before <= 0. && now > 0. || band(before, cuts) != band(now, cuts)
    }

    /// Keeps `now` and says whether it moved by half a pixel or more.
    fn record_any(&self, now: f32) -> bool {
        (self.0.replace(now) - now).abs() >= 0.5
    }
}

/// How many of `cuts` a width reaches.
fn band(width: f32, cuts: &[f32]) -> usize {
    cuts.iter().filter(|cut| **cut <= width).count()
}

#[cfg(test)]
mod tests;
