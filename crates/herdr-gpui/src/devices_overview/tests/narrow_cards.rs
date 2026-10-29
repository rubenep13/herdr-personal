//! The overview in a narrow tab: totals two by two, the chart under its
//! figure, an axis whose labels never touch, and a search that keeps room.
use super::*;
use gpui::{Bounds, Pixels, px, size};

fn bounds(cx: &mut VisualTestContext, selector: &str) -> Bounds<Pixels> {
    cx.debug_bounds(Box::leak(selector.to_owned().into_boxed_str()))
        .unwrap_or_else(|| panic!("{selector}"))
}

/// The overview open in a window `width` × `height` at `[ui]` `size`,
/// drawn twice: the first frame measures the tab.
fn shown(
    cx: &mut gpui::TestAppContext,
    width: f32,
    height: f32,
    ui: f32,
) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(width), px(height)));
    view.update(cx, |view, _| view.config.ui.size = ui);
    open(&view, cx);
    draw(cx);
    (view, cx)
}

fn at(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext, measure: f32) -> Pixels {
    view.read_with(cx, |view, _| view.view_look().at(measure))
}

fn two_by_two(cx: &mut VisualTestContext) {
    let cards: Vec<_> = (0..4)
        .map(|index| bounds(cx, &format!("devices-total-{index}")))
        .collect();
    assert!(
        (cards[1].top() - cards[0].top()).abs() < px(1.),
        "{cards:?}"
    );
    assert!(cards[1].left() >= cards[0].right(), "{cards:?}");
    assert!(cards[2].top() >= cards[0].bottom(), "{cards:?}");
    assert!(
        (cards[3].top() - cards[2].top()).abs() < px(1.),
        "{cards:?}"
    );
}

#[gpui::test]
fn the_totals_go_two_by_two_when_narrow(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = shown(cx, 680., 560., 18.);
    two_by_two(cx);
    // Two by two, each card takes half its row.
    let totals = bounds(cx, "devices-totals");
    for index in 0..4 {
        let card = bounds(cx, &format!("devices-total-{index}"));
        assert!(
            card.size.width >= totals.size.width / 2. - px(12.),
            "{card:?} in {totals:?}"
        );
    }
}

#[gpui::test]
fn the_totals_share_a_row_at_950(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = shown(cx, 950., 800., 12.);
    let first = bounds(cx, "devices-total-0");
    for index in 1..4 {
        let card = bounds(cx, &format!("devices-total-{index}"));
        assert!((card.top() - first.top()).abs() < px(1.), "{card:?}");
    }
}

#[gpui::test]
fn the_totals_follow_their_group_width(cx: &mut gpui::TestAppContext) {
    let (view, cx) = shown(cx, 1400., 900., 12.);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.command(Command::SplitEditor, window, cx);
            let slots = view.group_slots();
            let Some(crate::browser::Pick::Page(overview)) = view.group_pick(slots[0].id) else {
                panic!("the overview is shown")
            };
            view.show_browser_tab_in(Some(slots[1].id), overview, window, cx);
        })
    });
    draw(cx);
    draw(cx);
    two_by_two(cx);
}

#[gpui::test]
fn the_chart_drops_under_the_figure_when_narrow(cx: &mut gpui::TestAppContext) {
    let (view, cx) = shown(cx, 680., 560., 12.);
    let card = bounds(cx, "devices-activity");
    let summary = bounds(cx, "devices-summary");
    let bars = bounds(cx, "devices-bars");
    assert!(bars.top() >= summary.bottom(), "{bars:?} under {summary:?}");
    let inner = card.size.width - at(&view, cx, 14.) * 2. - px(2.);
    assert!(
        (bars.size.width - inner).abs() <= px(1.),
        "{bars:?} in {card:?}"
    );
}

#[gpui::test]
fn a_wide_chart_keeps_its_figure_beside(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = shown(cx, 1400., 900., 12.);
    let summary = bounds(cx, "devices-summary");
    let bars = bounds(cx, "devices-bars");
    assert!(
        bars.left() >= summary.right(),
        "{bars:?} beside {summary:?}"
    );
}

#[gpui::test]
fn axis_labels_never_overlap(cx: &mut gpui::TestAppContext) {
    for width in [680., 950., 1400.] {
        for ui in [12., 18.] {
            let (_view, cx) = shown(cx, width, 800., ui);
            let card = bounds(cx, "devices-activity");
            let labels: Vec<_> = (0..3)
                .filter_map(|index| {
                    cx.debug_bounds(["devices-axis-0", "devices-axis-1", "devices-axis-2"][index])
                })
                .collect();
            assert!(cx.debug_bounds("devices-axis-0").is_some());
            assert!(cx.debug_bounds("devices-axis-2").is_some());
            for pair in labels.windows(2) {
                assert!(
                    pair[0].right() <= pair[1].left(),
                    "{width} at [ui] {ui}: {labels:?}"
                );
            }
            for label in &labels {
                assert!(label.right() <= card.right(), "{label:?} in {card:?}");
            }
            if width == 680. && ui == 18. {
                assert!(cx.debug_bounds("devices-axis-1").is_none());
            }
        }
    }
}

#[gpui::test]
fn the_search_keeps_its_width(cx: &mut gpui::TestAppContext) {
    let (view, cx) = shown(cx, 680., 560., 18.);
    let search = bounds(cx, "devices-search");
    assert!(search.size.width >= at(&view, cx, 140.), "{search:?}");
    let bar = bounds(cx, "devices-bar");
    let add = bounds(cx, "devices-add");
    assert!(add.right() <= bar.right(), "{add:?} in {bar:?}");
}
