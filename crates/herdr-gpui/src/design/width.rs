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
        let width = self.clone();
        canvas(
            move |bounds, window, _| {
                if width.record(f32::from(bounds.size.width), &cuts) {
                    window.refresh();
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
}

/// How many of `cuts` a width reaches.
fn band(width: f32, cuts: &[f32]) -> usize {
    cuts.iter().filter(|cut| **cut <= width).count()
}

#[cfg(test)]
mod tests;
