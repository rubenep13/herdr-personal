//! The filter row in a narrow inbox: the order keeps its icon, then the
//! toggles fold into one Filters button.
use super::look::{bounds, inbox, issues};
use super::narrow::{settle, ui_size};
use crate::orchestrator::view::{
    filters::{FilterFit, filter_fit},
    rows::Tab,
};
use gpui::{Modifiers, TestAppContext, px};

#[test]
fn filter_fit_folds_in_order() {
    let toggles = [60., 90., 70.];
    assert_eq!(
        filter_fit(800., 180., &toggles, 170., 28., 6.),
        FilterFit::Full
    );
    assert_eq!(
        filter_fit(560., 180., &toggles, 170., 28., 6.),
        FilterFit::IconSort
    );
    assert_eq!(
        filter_fit(380., 180., &toggles, 170., 28., 6.),
        FilterFit::Folded
    );
}

#[gpui::test]
fn the_filter_field_keeps_180_px(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    view.update(cx, |view, cx| ui_size(view, 18., cx));
    settle(cx, 680., 560.);
    let search = bounds(cx, "orchestrator-search");
    let field_min = view.read_with(cx, |view, _| view.look.view().at(180.));
    assert!(search.size.width >= field_min, "{search:?}");
    let row = bounds(cx, "orchestrator-filters");
    let button = bounds(cx, "orchestrator-filters-button");
    assert!(button.right() <= row.right(), "{button:?} in {row:?}");
    assert!(
        button.left() >= search.right(),
        "{button:?} after {search:?}"
    );
    assert!(cx.debug_bounds("orchestrator-beads").is_none());
}

#[gpui::test]
fn a_wide_row_keeps_every_filter(cx: &mut TestAppContext) {
    let (_dir, _view, cx) = inbox(cx, issues(3));
    settle(cx, 1400., 900.);
    for selector in [
        "orchestrator-beads",
        "orchestrator-has-run",
        "orchestrator-blocked",
        "orchestrator-sort",
    ] {
        assert!(cx.debug_bounds(selector).is_some(), "{selector}");
    }
    assert!(cx.debug_bounds("orchestrator-filters-button").is_none());
}

#[gpui::test]
fn folded_filters_count_the_ones_on(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    view.update(cx, |view, cx| ui_size(view, 18., cx));
    settle(cx, 680., 560.);
    assert!(cx.debug_bounds("orchestrator-filters-count").is_none());
    view.update(cx, |view, cx| {
        view.filters.has_run = true;
        cx.notify();
    });
    settle(cx, 680., 560.);
    assert!(cx.debug_bounds("orchestrator-filters-count").is_some());
}

#[gpui::test]
fn the_filters_menu_flips_a_filter(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    view.update(cx, |view, cx| ui_size(view, 18., cx));
    settle(cx, 680., 560.);
    let button = bounds(cx, "orchestrator-filters-button");
    cx.simulate_click(button.center(), Modifiers::default());
    settle(cx, 680., 560.);
    bounds(cx, "orchestrator-filters-menu");
    // Beads, Has agent, then Blocked.
    let blocked = bounds(cx, "orchestrator-filters-menu-2");
    cx.simulate_click(blocked.center(), Modifiers::default());
    settle(cx, 680., 560.);
    view.read_with(cx, |view, _| {
        assert!(view.filters.blocked);
        // None of the fixture's issues is blocked.
        assert!(view.issue_rows.is_empty());
    });
    assert!(
        cx.debug_bounds("orchestrator-filters-menu-2-check")
            .is_some()
    );
}

#[gpui::test]
fn the_filter_bar_keeps_its_height_across_tabs(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    let mut heights = Vec::new();
    for (width, size) in [(1400., 12.), (680., 18.)] {
        view.update(cx, |view, cx| ui_size(view, size, cx));
        for tab in [Tab::Issues, Tab::PullRequests, Tab::Runs] {
            view.update(cx, |view, cx| {
                view.tab = tab;
                view.refresh_rows();
                cx.notify();
            });
            settle(cx, width, 560.);
            heights.push((size, bounds(cx, "orchestrator-filters").size.height));
        }
    }
    for (size, height) in &heights {
        let first = heights.iter().find(|(other, _)| other == size).unwrap().1;
        assert!((*height - first).abs() < px(0.5), "{heights:?}");
    }
}
