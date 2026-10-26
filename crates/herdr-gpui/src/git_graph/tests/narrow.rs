//! The graph in a narrow tab: the detail goes under the graph, the saved
//! width waits for room, and the column gear keeps its own place.
use super::detail::focus;
use super::look::sized;
use super::*;
use crate::git_graph::{history::History, prefs::GraphPrefsStore};
use gpui::{
    Bounds, Modifiers, MouseButton, Pixels, TestAppContext, VisualTestContext, point, px, size,
};

/// The graph with the second commit's detail open, in a window `width`
/// wide, drawn twice: the first frame measures the tab.
fn with_detail(
    cx: &mut TestAppContext,
    width: f32,
) -> (
    gpui::Entity<crate::git_graph::view::GitGraphView>,
    &mut VisualTestContext,
) {
    let (view, cx) = shown(cx, History::new(&line(5), &[], None, None, false), 560.);
    focus(&view, cx);
    cx.simulate_keystrokes("j enter");
    resize(cx, width);
    (view, cx)
}

fn resize(cx: &mut VisualTestContext, width: f32) {
    cx.simulate_resize(size(px(width), px(560.)));
    draw(cx);
    draw(cx);
}

fn crosses(a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
    a.left() < b.right() && b.left() < a.right() && a.top() < b.bottom() && b.top() < a.bottom()
}

#[gpui::test]
fn a_narrow_graph_puts_the_detail_under_it(cx: &mut TestAppContext) {
    let (_view, cx) = with_detail(cx, 680.);
    let detail = bounds(cx, "graph-detail");
    let columns = bounds(cx, "graph-cols");
    let row = bounds(cx, "graph-row-0");
    assert!(detail.top() >= row.bottom(), "{detail:?} under {row:?}");
    assert_eq!(detail.left(), columns.left());
    assert!((detail.size.width - columns.size.width).abs() <= px(1.));
    let area = detail.bottom() - columns.top();
    assert!(
        (detail.size.height - area / 2.).abs() <= px(1.),
        "{detail:?} of {area:?}"
    );
    assert!(!drawn(cx, "graph-detail-resize"), "nothing to drag under");
}

#[gpui::test]
fn a_wide_graph_keeps_the_detail_beside_it(cx: &mut TestAppContext) {
    let (_view, cx) = with_detail(cx, 1400.);
    let detail = bounds(cx, "graph-detail");
    let columns = bounds(cx, "graph-cols");
    assert!(
        detail.left() >= columns.right(),
        "{detail:?} beside {columns:?}"
    );
    let saved = cx.update(|_, cx| GraphPrefsStore::get(cx).panel_width);
    assert!(
        (detail.size.width - px(saved)).abs() <= px(1.),
        "{detail:?}"
    );
    assert!(columns.size.width >= px(400.));
}

#[gpui::test]
fn the_saved_panel_width_survives_the_stack(cx: &mut TestAppContext) {
    let (_view, cx) = with_detail(cx, 1400.);
    let saved = cx.update(|_, cx| GraphPrefsStore::get(cx).panel_width);
    resize(cx, 680.);
    resize(cx, 1400.);
    assert_eq!(
        cx.update(|_, cx| GraphPrefsStore::get(cx).panel_width),
        saved
    );
    let detail = bounds(cx, "graph-detail");
    assert!(
        (detail.size.width - px(saved)).abs() <= px(1.),
        "{detail:?}"
    );
}

#[gpui::test]
fn the_panel_drag_leaves_the_graph_its_room(cx: &mut TestAppContext) {
    let (_view, cx) = with_detail(cx, 1000.);
    let edge = bounds(cx, "graph-detail-resize").center();
    cx.simulate_mouse_down(edge, MouseButton::Left, Modifiers::default());
    for x in [edge.x - px(20.), px(10.)] {
        cx.simulate_mouse_move(point(x, edge.y), MouseButton::Left, Modifiers::default());
    }
    cx.simulate_mouse_up(
        point(px(10.), edge.y),
        MouseButton::Left,
        Modifiers::default(),
    );
    resize(cx, 1000.);
    let columns = bounds(cx, "graph-cols");
    let detail = bounds(cx, "graph-detail");
    assert!(detail.left() >= columns.right(), "still beside");
    assert!(columns.size.width >= px(400.) - px(1.), "{columns:?}");
}

#[gpui::test]
fn the_column_gear_covers_no_title(cx: &mut TestAppContext) {
    let (_view, cx) = sized(cx, 18.);
    cx.update(|_, cx| {
        GraphPrefsStore::update(cx, |prefs| {
            prefs.author = true;
            prefs.date = true;
            prefs.sha = true;
        })
    });
    resize(cx, 680.);
    let gear = bounds(cx, "graph-col-gear");
    let titles = bounds(cx, "graph-col-titles");
    let columns = bounds(cx, "graph-cols");
    assert!(gear.right() <= columns.right(), "{gear:?} in {columns:?}");
    // The titles end where the gear's own place starts, and cut there.
    assert!(gear.left() >= titles.right(), "{gear:?} after {titles:?}");
    for key in ["refs", "graph", "msg", "author", "date", "sha"] {
        let title = bounds(cx, &format!("graph-col-{key}"));
        assert!(
            !crosses(gear, title) || title.right() > titles.right(),
            "{key}: {title:?} under {gear:?}"
        );
    }
    let refs = bounds(cx, "graph-col-refs");
    assert!(refs.right() <= titles.right(), "{refs:?} in {titles:?}");
}

#[gpui::test]
fn the_time_marker_keeps_off_the_columns(cx: &mut TestAppContext) {
    let now = chrono::Utc::now().timestamp();
    let mut commits = line(2);
    commits[0].authored = now;
    commits[1].authored = now - 3 * 24 * 3600;
    let (_view, cx) = shown(cx, History::new(&commits, &[], None, None, false), 560.);
    cx.update(|_, cx| GraphPrefsStore::update(cx, |prefs| prefs.author = true));
    draw(cx);
    draw(cx);
    let marker = bounds(cx, "graph-when-1");
    let author = bounds(cx, "graph-col-author");
    assert!(
        marker.right() <= author.left(),
        "{marker:?} before {author:?}"
    );
}
