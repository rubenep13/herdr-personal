//! The review in the redesign's pieces: its bar, its panels and the search
//! band.
use super::{changes, the, window};
use crate::{
    design::Tone,
    review::{diff::Scope, diff::Status, view::files::status_tone},
};
use gpui::{Bounds, Pixels, px, size};

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

fn bounds(cx: &mut gpui::VisualTestContext, selector: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector}"))
}

fn opened(
    cx: &mut gpui::TestAppContext,
    width: f32,
) -> (
    gpui::Entity<crate::HerdrWindow>,
    &mut gpui::VisualTestContext,
) {
    let (view, cx) = window(cx, None);
    cx.simulate_resize(size(px(width), px(900.)));
    cx.update(|window, cx| view.update(cx, |view, cx| view.seed_review(changes(), window, cx)));
    // The panels follow the review's width, which the first frame measures.
    draw(cx);
    draw(cx);
    (view, cx)
}

#[gpui::test]
fn the_review_bar_keeps_its_controls_in_order(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = opened(cx, 1600.);
    let bar = bounds(cx, "review-bar");
    assert_eq!(bar.size.height, px(40.));
    let mut left = bar.left();
    for selector in [
        "review-toggle-files",
        "review-scope-uncommitted",
        "review-against",
        "review-search-open",
        "review-layout-unified",
        "review-toggle-notes",
        "review-destination",
    ] {
        let control = bounds(cx, selector);
        assert!(control.left() >= left, "{selector} is out of order");
        assert!(control.right() <= bar.right(), "{selector}");
        left = control.left();
    }
    assert_eq!(bounds(cx, "review-search-open").size.height, px(28.));
    assert_eq!(bounds(cx, "review-scope").size.height, px(26.));
}

#[gpui::test]
fn scope_segments_switch_the_scope(cx: &mut gpui::TestAppContext) {
    let (view, cx) = opened(cx, 1600.);
    let branch = bounds(cx, "review-scope-branch");
    cx.simulate_click(branch.center(), gpui::Modifiers::default());
    view.read_with(cx, |view, _| {
        assert_eq!(view.reviews[&the(view)].scope, Scope::Branch);
    });
}

#[gpui::test]
fn files_panel_rows_are_inset(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = opened(cx, 1600.);
    let panel = bounds(cx, "review-files");
    let row = bounds(cx, "review-file-0");
    let inset = row.left() - panel.left();
    assert!(inset >= px(8.) && inset <= px(9.), "{inset:?}");
    assert_eq!(row.size.height, px(32.));
    assert_eq!(bounds(cx, "review-files-header").size.height, px(40.));
    assert_eq!(bounds(cx, "review-filter").size.height, px(28.));
}

#[gpui::test]
fn the_search_band_does_not_move_the_bar(cx: &mut gpui::TestAppContext) {
    let (view, cx) = opened(cx, 1600.);
    let before = bounds(cx, "review-bar");
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            let id = the(view);
            view.open_review_search(id, window, cx);
        })
    });
    draw(cx);
    let after = bounds(cx, "review-bar");
    assert_eq!(before, after);
    let band = bounds(cx, "review-search");
    assert!(band.top() >= after.bottom());
    assert_eq!(band.size.height, px(44.));
    let field = bounds(cx, "review-search-field");
    assert!(band.contains(&field.center()));
}

#[gpui::test]
fn a_narrow_review_keeps_its_buttons(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx, None);
    cx.simulate_resize(size(px(1000.), px(700.)));
    view.update(cx, |view, _| view.config.ui.size = 18.);
    cx.update(|window, cx| view.update(cx, |view, cx| view.seed_review(changes(), window, cx)));
    draw(cx);
    draw(cx);
    let bar = bounds(cx, "review-bar");
    for selector in [
        "review-search-open",
        "review-layout-split",
        "review-toggle-notes",
    ] {
        let control = bounds(cx, selector);
        assert!(
            control.right() <= bar.right(),
            "{selector}: {control:?} in {bar:?}"
        );
    }
}

#[test]
fn status_letters_take_their_tone() {
    for (status, tone) in [
        (Status::Modified, Tone::Warn),
        (Status::Added, Tone::Good),
        (Status::Untracked, Tone::Good),
        (Status::Deleted, Tone::Danger),
        (Status::Renamed, Tone::Accent),
    ] {
        assert_eq!(status_tone(status), tone, "{status:?}");
    }
}
