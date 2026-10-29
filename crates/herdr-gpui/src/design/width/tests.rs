use super::*;
use core::prelude::v1::test;

#[test]
fn band_counts_the_cuts_a_width_reaches() {
    assert_eq!(band(0., &[400., 800.]), 0);
    assert_eq!(band(400., &[400., 800.]), 1);
    assert_eq!(band(799.9, &[400., 800.]), 1);
    assert_eq!(band(800., &[400., 800.]), 2);
    assert_eq!(band(5000., &[]), 0);
}

#[test]
fn a_width_is_zero_until_drawn() {
    assert_eq!(Width::default().get(), 0.);
}

#[test]
fn a_new_width_crossing_no_cut_asks_for_no_frame() {
    let width = Width::default();
    // The first frame always counts: before it the view guessed.
    assert!(width.record(500., &[400., 800.]));
    assert!(!width.record(700., &[400., 800.]));
    assert_eq!(width.get(), 700.);
    assert!(width.record(800., &[400., 800.]));
    assert!(width.record(399., &[400., 800.]));
    assert!(!width.record(399., &[]));
}

#[test]
fn following_asks_for_a_frame_on_any_move() {
    let width = Width::default();
    assert!(width.record_any(500.));
    assert!(!width.record_any(500.2));
    assert!(width.record_any(503.));
    assert_eq!(width.get(), 503.);
}

struct Probed(Width);

impl Render for Probed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.0.probe([500.]))
    }
}

/// A probe that crosses a cut while the window draws asks for the next
/// frame, since a refresh during a draw is dropped.
#[gpui::test]
fn crossing_a_cut_asks_for_the_next_frame(cx: &mut TestAppContext) {
    let (_view, cx) = cx.add_window_view(|_, _| Probed(Width::default()));
    let draw = |cx: &mut VisualTestContext| {
        cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    };
    let next = |cx: &mut VisualTestContext| cx.update(|window, cx| window.simulate_next_frame(cx));
    cx.simulate_resize(size(px(400.), px(300.)));
    draw(cx);
    // The first layout always counts.
    assert!(next(cx) > 0);
    draw(cx);
    cx.simulate_resize(size(px(450.), px(300.)));
    draw(cx);
    assert_eq!(next(cx), 0, "no cut crossed");
    cx.simulate_resize(size(px(600.), px(300.)));
    draw(cx);
    assert!(next(cx) > 0, "500 crossed");
}
