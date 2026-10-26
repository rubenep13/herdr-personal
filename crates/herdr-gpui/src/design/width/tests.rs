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
