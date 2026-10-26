//! The history grows with the interface: rows, lanes, nodes and labels at
//! the chrome's face, the same for painting and for clicks, with column
//! widths saved as at the face of 12.
use super::look::sized;
use super::*;
use crate::git_graph::{
    geometry::{CHIP_H, LANE_W, NODE_R, ROW_H},
    history::History,
    prefs::GraphPrefsStore,
    view::GraphLook,
};
use gpui::{Modifiers, MouseButton, TestAppContext, point, px};

fn chrome(size: f32) -> GraphLook {
    let config = crate::config::Config::default();
    let mut chrome = config.ui.clone();
    chrome.size = size;
    GraphLook::new(
        crate::config::Theme::default(),
        config.ui.clone(),
        config.terminal.clone(),
        false,
        &chrome,
    )
}

#[gpui::test]
fn the_default_graph_is_unchanged(_cx: &mut TestAppContext) {
    let look = look();
    assert_eq!(look.scale(), 1.);
    assert_eq!(look.at(ROW_H), 30.);
    assert_eq!(look.at(LANE_W), 22.);
    assert_eq!(look.at(NODE_R), 10.);
    assert_eq!(look.at(CHIP_H), 22.);
}

#[gpui::test]
fn rows_and_lanes_grow_with_the_interface(cx: &mut TestAppContext) {
    let look = chrome(18.);
    assert_eq!(look.at(ROW_H), 45.);
    assert_eq!(look.at(LANE_W), 33.);
    assert_eq!(look.at(CHIP_H), 33.);
    let (_view, cx) = sized(cx, 18.);
    assert_eq!(bounds(cx, "graph-row-0").size.height, px(45.));
    assert_eq!(bounds(cx, "graph-node-0").size.width, px(30.));
    // One lane: the column's least width, 64 at the face of 12.
    assert_eq!(bounds(cx, "graph-graph-0").size.width, px(96.));
}

#[gpui::test]
fn ctrl_plus_grows_the_history(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update(cx, |view, _| {
        view.config.ui.size = 12.;
        view.config.terminal.size = 18.;
        view.configured_terminal_size = 12.;
        let look = view.graph_look();
        assert_eq!(look.scale(), 1.5);
        assert_eq!(look.at(ROW_H), 45.);
        assert_eq!(look.ui.size, 18.);
        assert_eq!(look.mono.size, 18.);
    });
}

#[gpui::test]
fn the_rows_follow_the_interface_size(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update(cx, |view, _| {
        view.config.ui.size = 18.;
        view.config.terminal.size = 12.;
        view.configured_terminal_size = 12.;
        let look = view.graph_look();
        assert_eq!(look.scale(), 1.5);
        assert_eq!(look.ui.size, 18.);
    });
}

#[gpui::test]
fn a_click_lands_on_the_row_drawn_there(cx: &mut TestAppContext) {
    let (view, cx) = sized(cx, 18.);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let history = History::new(&line(6), &[], None, None, false);
            let reply = crate::git_graph::worker::Reply::History(std::sync::Arc::new(history));
            assert!(view.receive(1, reply, cx));
        })
    });
    draw(cx);
    let columns = bounds(cx, "graph-cols");
    // Row 3's middle, from the scaled geometry rather than the layout.
    let y = columns.bottom() + px(3. * 45. + 22.5);
    let x = columns.left() + px(400.);
    cx.simulate_click(point(x, y), Modifiers::default());
    draw(cx);
    view.read_with(cx, |view, _| assert_eq!(view.selected_row(), Some(3)));
}

#[gpui::test]
fn column_widths_are_saved_unscaled(cx: &mut TestAppContext) {
    let (_view, cx) = sized(cx, 18.);
    let edge = bounds(cx, "graph-col-refs-resize").center();
    cx.simulate_mouse_down(edge, MouseButton::Left, Modifiers::default());
    for x in [edge.x + px(10.), px(300.)] {
        cx.simulate_mouse_move(point(x, edge.y), MouseButton::Left, Modifiers::default());
    }
    cx.simulate_mouse_up(
        point(px(300.), edge.y),
        MouseButton::Left,
        Modifiers::default(),
    );
    let saved = cx.update(|_, cx| GraphPrefsStore::get(cx).refs_width);
    assert!((saved - 200.).abs() < 0.5, "{saved}");
    draw(cx);
    assert!((bounds(cx, "graph-col-refs").size.width - px(300.)).abs() < px(0.5));
}

#[gpui::test]
fn a_saved_width_paints_unscaled_at_twelve(cx: &mut TestAppContext) {
    let (_view, cx) = sized(cx, 12.);
    cx.update(|_, cx| GraphPrefsStore::update(cx, |prefs| prefs.refs_width = 200.));
    draw(cx);
    draw(cx);
    assert!((bounds(cx, "graph-col-refs").size.width - px(200.)).abs() < px(0.5));
}
