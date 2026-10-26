//! An option strip too wide for its card wraps its segments to a second row
//! inside its box rather than leaving the card.
use super::*;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

/// The Tabs & panes page in a window `width` × `height` at `[ui]` `ui`,
/// scrolled to the Position row.
fn tabs_page(cx: &mut TestAppContext, width: f32, height: f32, ui: f32) -> &mut VisualTestContext {
    let (view, cx) = cx.add_window_view(skill_fixture);
    view.update(cx, |view, cx| {
        view.config.ui.size = ui;
        view.section = Section::Tabs;
        cx.notify();
    });
    cx.simulate_resize(size(px(width), px(height)));
    draw(cx);
    view.update(cx, |view, cx| view.scroll_to_row("tabs-placement", cx));
    draw(cx);
    cx
}

fn segments(cx: &mut VisualTestContext) -> Vec<Bounds<Pixels>> {
    (0..3)
        .map(|index| {
            let selector = format!("tabs-placement-control-{index}");
            cx.debug_bounds(Box::leak(selector.into_boxed_str()))
                .unwrap_or_else(|| panic!("segment {index}"))
        })
        .collect()
}

#[gpui::test]
fn segments_stay_inside_their_card(cx: &mut TestAppContext) {
    let cx = tabs_page(cx, 680., 560., 18.);
    let card = cx.debug_bounds("settings-card-tab-bar").unwrap();
    let strip = cx.debug_bounds("tabs-placement-control-segments").unwrap();
    assert!(strip.right() <= card.right(), "{strip:?} in {card:?}");
    for segment in segments(cx) {
        assert!(
            segment.left() >= strip.left() && segment.right() <= strip.right(),
            "{segment:?} in {strip:?}"
        );
        assert!(
            segment.bottom() <= strip.bottom(),
            "{segment:?} in {strip:?}"
        );
    }
}

#[gpui::test]
fn segments_keep_one_row_when_they_fit(cx: &mut TestAppContext) {
    let cx = tabs_page(cx, 1200., 800., 12.);
    let segments = segments(cx);
    for segment in &segments[1..] {
        assert_eq!(segment.top(), segments[0].top(), "{segments:?}");
    }
}
