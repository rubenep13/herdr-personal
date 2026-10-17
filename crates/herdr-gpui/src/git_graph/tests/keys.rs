//! Moving through the graph from the keyboard, and the columns and widths
//! every graph tab shares.

use super::*;
use crate::git_graph::{
    GraphCheckout,
    geometry::{AUTHOR_W, DATE_W, REFS_DEFAULT},
    history::History,
    model::{Ref, RefKind},
    prefs::GraphPrefsStore,
    view::GitGraphView,
    worker::Request,
};
use crate::teleport::Host;
use gpui::{
    Modifiers, MouseButton, MouseDownEvent, TestAppContext, VisualTestContext, point, px, size,
};
use herdr_client::ConnectTarget;

fn focus(view: &gpui::Entity<GitGraphView>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let handle = view.read(cx).focus_handle.clone();
        window.focus(&handle, cx);
    });
}

fn selected(view: &gpui::Entity<GitGraphView>, cx: &mut VisualTestContext) -> Option<usize> {
    view.read_with(cx, |view, _| view.selected_row())
}

#[gpui::test]
fn j_k_and_arrows_move_the_selection(cx: &mut TestAppContext) {
    let (view, cx) = shown(cx, History::new(&line(40), &[], None, None, false), 300.);
    focus(&view, cx);
    assert_eq!(selected(&view, cx), None);
    let mut seen = Vec::new();
    for key in ["j", "j", "down", "k", "up", "k"] {
        cx.simulate_keystrokes(key);
        seen.push(selected(&view, cx).unwrap());
    }
    assert_eq!(seen, [0, 1, 2, 1, 0, 0], "and never above the top");
    for _ in 0..50 {
        cx.simulate_keystrokes("j");
    }
    assert_eq!(selected(&view, cx), Some(39), "nor below the bottom");
    draw(cx);
    assert!(drawn(cx, "graph-row-39"), "the selection is kept in view");
    view.read_with(cx, |view, _| {
        assert_eq!(view.band(39), 0.42);
        assert_eq!(view.band(38), 0.16);
    });
}

#[gpui::test]
fn h_selects_head_and_c_copies_its_hash(cx: &mut TestAppContext) {
    let refs = [Ref {
        name: "main".into(),
        target: "c5".into(),
        kind: RefKind::Current,
    }];
    let history = History::new(&line(10), &refs, Some("c5".into()), None, false);
    let (view, cx) = shown(cx, history, 600.);
    focus(&view, cx);
    cx.simulate_keystrokes("h");
    assert_eq!(selected(&view, cx), Some(5));
    cx.simulate_keystrokes("c");
    let copied = cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(copied.as_deref(), Some("c5"));
}

#[gpui::test]
fn r_reloads(cx: &mut TestAppContext) {
    let checkout = GraphCheckout::new("/nonexistent/graph-test".into()).unwrap();
    let local = Host::new(&ConnectTarget::Local).unwrap();
    let (view, cx) = cx.add_window_view(|_, cx| GitGraphView::new(Ok(local), checkout, look(), cx));
    focus(&view, cx);
    let first = view.read_with(cx, |view, _| view.generation());
    cx.simulate_keystrokes("r");
    assert_eq!(
        view.read_with(cx, |view, _| view.last_request()),
        (first + 1, Request::Load)
    );
}

#[gpui::test]
fn graph_keys_need_graph_focus(cx: &mut TestAppContext) {
    let (view, cx) = shown(cx, History::new(&line(10), &[], None, None, false), 600.);
    cx.update(|window, cx| window.blur(cx));
    cx.simulate_keystrokes("j j");
    assert_eq!(selected(&view, cx), None);
    // Modifiers belong to the app's own shortcuts.
    focus(&view, cx);
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(selected(&view, cx), None);
}

fn click(cx: &mut VisualTestContext, selector: &str) {
    let at = bounds(cx, selector).center();
    cx.simulate_click(at, Modifiers::none());
    draw(cx);
}

#[gpui::test]
fn gear_adds_columns_and_remembers_them(cx: &mut TestAppContext) {
    let (_view, window) = shown(cx, History::new(&line(5), &[], None, None, false), 600.);
    assert!(!drawn(window, "graph-col-menu"));
    click(window, "graph-col-gear");
    click(window, "graph-col-menu-author");
    click(window, "graph-col-menu-date");
    assert_eq!(bounds(window, "graph-col-author").size.width, px(AUTHOR_W));
    assert_eq!(bounds(window, "graph-col-date").size.width, px(DATE_W));
    assert!(!drawn(window, "graph-col-sha"));
    let prefs = window.update(|_, cx| GraphPrefsStore::get(cx));
    assert!(prefs.author && prefs.date && !prefs.sha);
    // A graph opened later starts with them.
    let (_other, later) = shown(cx, History::new(&line(5), &[], None, None, false), 600.);
    assert_eq!(bounds(later, "graph-col-author").size.width, px(AUTHOR_W));
}

fn drag(cx: &mut VisualTestContext, selector: &str, to: f32) {
    let edge = bounds(cx, selector).center();
    cx.simulate_mouse_down(edge, MouseButton::Left, Modifiers::default());
    // The first move past the threshold only starts the drag.
    cx.simulate_mouse_move(
        point(edge.x + px(20.), edge.y),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(
        point(px(to), edge.y),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(to), edge.y),
        MouseButton::Left,
        Modifiers::default(),
    );
    draw(cx);
}

fn double_click(cx: &mut VisualTestContext, selector: &str) {
    let position = bounds(cx, selector).center();
    cx.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 2,
        first_mouse: false,
    });
    draw(cx);
}

#[gpui::test]
fn refs_column_resizes_within_bounds(cx: &mut TestAppContext) {
    let (_view, cx) = shown(cx, History::new(&line(5), &[], None, None, false), 600.);
    cx.simulate_resize(size(px(1600.), px(600.)));
    draw(cx);
    let refs = |cx: &mut VisualTestContext| bounds(cx, "graph-col-refs").size.width;
    drag(cx, "graph-col-refs-resize", 1500.);
    assert_eq!(refs(cx), px(640.));
    drag(cx, "graph-col-refs-resize", 5.);
    assert_eq!(refs(cx), px(90.));
    double_click(cx, "graph-col-refs-resize");
    assert_eq!(refs(cx), px(REFS_DEFAULT));

    let auto = bounds(cx, "graph-col-graph").size.width;
    let graph = bounds(cx, "graph-col-graph");
    drag(
        cx,
        "graph-col-graph-resize",
        f32::from(graph.right()) + 100.,
    );
    let wider = bounds(cx, "graph-col-graph").size.width;
    assert!(wider > auto + px(50.), "{wider:?} after {auto:?}");
    assert!(
        cx.update(|_, cx| GraphPrefsStore::get(cx).graph_width)
            .is_some()
    );
    double_click(cx, "graph-col-graph-resize");
    assert_eq!(bounds(cx, "graph-col-graph").size.width, auto);
    assert_eq!(
        cx.update(|_, cx| GraphPrefsStore::get(cx).graph_width),
        None
    );
}
