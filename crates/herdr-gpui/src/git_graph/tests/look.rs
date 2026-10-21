//! The graph's chrome in the redesign's pieces: its bar, column headings and
//! menus, growing with the graph's face.
use super::*;
use crate::git_graph::history::History;
use gpui::{TestAppContext, px, size};

/// A view at the graph's face of `font` points, showing a short line.
fn sized(
    cx: &mut TestAppContext,
    font: f32,
) -> (
    gpui::Entity<crate::git_graph::view::GitGraphView>,
    &mut gpui::VisualTestContext,
) {
    let checkout = crate::git_graph::GraphCheckout::new("/repo".into()).unwrap();
    let mut graph = look();
    graph.ui.size = font;
    let (view, cx) = cx.add_window_view(|_, cx| {
        crate::git_graph::view::GitGraphView::new(
            Err(crate::git_graph::Error::WorkerStopped),
            checkout,
            graph,
            crate::git_graph::view::Connection::Online,
            cx,
        )
    });
    cx.simulate_resize(size(px(1000.), px(600.)));
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let history = History::new(&line(3), &[], None, None, false);
            let reply = crate::git_graph::worker::Reply::History(std::sync::Arc::new(history));
            assert!(view.receive(0, reply, cx));
        })
    });
    draw(cx);
    (view, cx)
}

#[gpui::test]
fn the_graph_bar_is_a_view_bar(cx: &mut TestAppContext) {
    let (_view, cx) = sized(cx, 12.);
    assert_eq!(bounds(cx, "graph-bar").size.height, px(40.));
    let help = bounds(cx, "graph-help-button");
    assert_eq!(help.size.height, px(28.));
    assert_eq!(help.size.width, px(28.));
    assert_eq!(bounds(cx, "graph-reload").size.height, px(28.));
    assert_eq!(bounds(cx, "graph-cols").size.height, px(28.));
}

#[gpui::test]
fn the_graph_grows_with_its_font(cx: &mut TestAppContext) {
    let (_view, cx) = sized(cx, 18.);
    assert_eq!(bounds(cx, "graph-bar").size.height, px(60.));
    assert_eq!(bounds(cx, "graph-cols").size.height, px(42.));
    assert_eq!(bounds(cx, "graph-help-button").size.height, px(42.));
}

#[gpui::test]
fn the_column_menu_is_a_menu(cx: &mut TestAppContext) {
    let (_view, cx) = sized(cx, 12.);
    let gear = bounds(cx, "graph-col-gear");
    cx.simulate_click(gear.center(), gpui::Modifiers::default());
    draw(cx);
    let menu = bounds(cx, "graph-col-menu");
    let section = bounds(cx, "graph-col-menu-section");
    assert!(menu.contains(&section.center()));
    for key in ["author", "date", "sha"] {
        let row = bounds(cx, &format!("graph-col-menu-{key}"));
        assert_eq!(row.size.height, px(32.), "{key}");
        assert!(row.top() >= section.bottom());
    }
}
