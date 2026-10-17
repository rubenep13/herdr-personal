//! Rows drawn as hgg draws them: 30 px rows, lanes of the theme's colours,
//! author bubbles, merge dots, the WIP ring and stash square, and branch
//! labels that fold into a "+N" badge, with only the visible rows drawn.

use super::*;
use crate::git_graph::{
    GraphCheckout,
    chips::{Chip, ChipKind},
    error::Error,
    geometry::{GRAPH_PAD, LANE_W, ROW_H, Step, lane_colour, lane_x, row_y, segment_steps},
    git::WorktreeCounts,
    history::History,
    layout::{Anchor, Point, Segment},
    model::{Ref, RefKind},
    view::{GitGraphView, GraphLook, chip_icons},
    worker::{Reply, Request},
};
use crate::{config::Theme, teleport::Host};
use gpui::{Bounds, Entity, Pixels, TestAppContext, VisualTestContext, px, size};
use herdr_client::ConnectTarget;
use std::sync::Arc;

fn look() -> GraphLook {
    let config = crate::config::Config::default();
    GraphLook {
        theme: Theme::default(),
        ui: config.ui.clone(),
        mono: config.terminal.clone(),
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

/// A view showing `history` in a window `height` pixels tall, with no
/// worker behind it.
fn shown(
    cx: &mut TestAppContext,
    history: History,
    height: f32,
) -> (Entity<GitGraphView>, &mut VisualTestContext) {
    let checkout = GraphCheckout::new("/repo".into()).unwrap();
    let (view, cx) = cx.add_window_view(|_, cx| {
        GitGraphView::new(Err(Error::WorkerStopped), checkout, look(), cx)
    });
    cx.simulate_resize(size(px(1000.), px(height)));
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            assert!(view.receive(0, Reply::History(Arc::new(history)), cx));
        })
    });
    draw(cx);
    (view, cx)
}

fn bounds(cx: &mut VisualTestContext, selector: &str) -> Bounds<Pixels> {
    cx.debug_bounds(Box::leak(selector.to_owned().into_boxed_str()))
        .unwrap_or_else(|| panic!("{selector} is drawn"))
}

fn drawn(cx: &mut VisualTestContext, selector: &str) -> bool {
    cx.debug_bounds(Box::leak(selector.to_owned().into_boxed_str()))
        .is_some()
}

fn line(count: usize) -> Vec<Commit> {
    (0..count)
        .map(|row| {
            let parent = format!("c{}", row + 1);
            let parents: Vec<&str> = if row + 1 < count {
                vec![&parent]
            } else {
                vec![]
            };
            commit(&format!("c{row}"), &parents)
        })
        .collect()
}

fn merge_history() -> Vec<Commit> {
    vec![
        commit("m", &["a", "b"]),
        commit("a", &["base"]),
        commit("b", &["base"]),
        commit("base", &[]),
    ]
}

fn reference(name: &str, target: &str, kind: RefKind) -> Ref {
    Ref {
        name: name.into(),
        target: target.into(),
        kind,
    }
}

#[gpui::test]
fn rows_are_thirty_pixels_and_columns_follow_the_defaults(cx: &mut TestAppContext) {
    let history = History::new(&merge_history(), &[], None, None, false);
    assert_eq!(history.layout.lane_count, 2);
    let (_view, cx) = shown(cx, history, 600.);
    assert_eq!(bounds(cx, "graph-row-0").size.height, px(ROW_H));
    assert_eq!(bounds(cx, "graph-col-refs").size.width, px(190.));
    let graph = (GRAPH_PAD * 2. + 2. * LANE_W).max(64.);
    assert_eq!(bounds(cx, "graph-col-graph").size.width, px(graph));
    assert!(drawn(cx, "graph-col-msg"));
    for hidden in ["graph-col-author", "graph-col-date", "graph-col-sha"] {
        assert!(!drawn(cx, hidden), "{hidden}");
    }
    // Cells line up with their headers.
    assert_eq!(
        bounds(cx, "graph-graph-0").origin.x,
        bounds(cx, "graph-col-graph").origin.x
    );
}

#[gpui::test]
fn only_visible_rows_are_drawn(cx: &mut TestAppContext) {
    let history = History::new(&line(3000), &[], None, None, true);
    let (_view, cx) = shown(cx, history, 600.);
    let rows = (0..3000)
        .filter(|row| drawn(cx, &format!("graph-row-{row}")))
        .count();
    assert!(rows >= 15, "{rows} rows drawn");
    assert!(rows <= 600 / 30 + 10, "{rows} rows drawn");
}

#[gpui::test]
fn nodes_follow_the_commit_kind(cx: &mut TestAppContext) {
    let mut stash = commit("s", &["m"]);
    stash.kind = CommitKind::Stash;
    let mut commits = vec![stash];
    commits.extend(merge_history());
    let counts = WorktreeCounts {
        staged: 1,
        unstaged: 2,
        ..Default::default()
    };
    let history = History::new(&commits, &[], Some("m".into()), Some(counts), false);
    let (view, cx) = shown(cx, history, 600.);
    let side = |cx: &mut VisualTestContext, row: usize| {
        let node = bounds(cx, &format!("graph-node-{row}"));
        (node.size.width, node.size.height)
    };
    assert_eq!(side(cx, 0), (px(18.), px(18.)), "the WIP ring");
    assert_eq!(side(cx, 1), (px(14.), px(14.)), "the stash square");
    assert_eq!(side(cx, 2), (px(10.), px(10.)), "the merge dot");
    assert_eq!(side(cx, 3), (px(20.), px(20.)), "the author bubble");
    assert!(drawn(cx, "graph-initials-3"));
    assert!(!drawn(cx, "graph-initials-2"));
    // The WIP row says what is uncommitted.
    assert!(drawn(cx, "graph-wip-staged-0"));
    assert!(drawn(cx, "graph-wip-unstaged-0"));
    assert!(!drawn(cx, "graph-wip-untracked-0"));
    // Nodes sit on their lanes, in the middle of their rows.
    let graph = bounds(cx, "graph-graph-3");
    let node = bounds(cx, "graph-node-3");
    let lane = view.read_with(cx, |view, _| view.history().unwrap().layout.nodes[3].lane);
    assert_eq!(node.center().x - graph.origin.x, px(lane_x(lane)));
    assert_eq!(node.center().y - graph.origin.y, px(ROW_H / 2.));
}

#[gpui::test]
fn scrolling_near_the_end_loads_more(cx: &mut TestAppContext) {
    let checkout = GraphCheckout::new("/nonexistent/graph-test".into()).unwrap();
    let local = Host::new(&ConnectTarget::Local).unwrap();
    let (view, cx) = cx.add_window_view(|_, cx| GitGraphView::new(Ok(local), checkout, look(), cx));
    cx.simulate_resize(size(px(1000.), px(600.)));
    let first = view.read_with(cx, |view, _| view.generation());
    // The test answers the first load itself and never polls the worker.
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let history = History::new(&line(200), &[], None, None, true);
            assert!(view.receive(first, Reply::History(Arc::new(history)), cx));
        })
    });
    draw(cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.generation()),
        first,
        "the top asks for nothing"
    );
    cx.update(|_, cx| view.update(cx, |view, _| view.scroll_to_row(190)));
    draw(cx);
    let (asked, request) = view.read_with(cx, |view, _| view.last_request());
    assert_eq!((asked, request), (first + 1, Request::More));
    // No second request while the first is out.
    draw(cx);
    cx.update(|_, cx| view.update(cx, |view, _| view.scroll_to_row(195)));
    draw(cx);
    assert_eq!(view.read_with(cx, |view, _| view.generation()), asked);
    // Without more history, the end asks for nothing.
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let history = History::new(&line(400), &[], None, None, false);
            assert!(view.receive(asked, Reply::History(Arc::new(history)), cx));
            view.scroll_to_row(395);
        })
    });
    draw(cx);
    assert_eq!(view.read_with(cx, |view, _| view.generation()), asked);
}

#[test]
fn current_branch_chip_has_its_check_and_laptop() {
    let chip = |kind, current, remotes: &[&str]| Chip {
        label: "x".into(),
        kind,
        current,
        remotes: remotes.iter().map(|remote| (*remote).into()).collect(),
        names: Vec::new(),
    };
    assert_eq!(
        (chip_icons(&chip(ChipKind::Branch, true, &["origin"]))),
        (
            vec!["icons/graph-check.svg"],
            vec!["icons/graph-laptop.svg", "icons/graph-remote.svg"]
        )
    );
    assert_eq!(
        (chip_icons(&chip(ChipKind::Branch, false, &[]))),
        (vec![], vec!["icons/graph-laptop.svg"])
    );
    assert_eq!(
        (chip_icons(&chip(ChipKind::Tag, false, &[]))),
        (vec!["icons/graph-tag.svg"], vec![])
    );
    assert_eq!(
        (chip_icons(&chip(ChipKind::Remote, false, &["origin"]))),
        (vec![], vec!["icons/graph-remote.svg"])
    );
    assert_eq!(
        (chip_icons(&chip(ChipKind::Stash, false, &[]))),
        (vec!["icons/graph-stash.svg"], vec![])
    );
}

#[gpui::test]
fn long_branch_names_fold_into_the_badge(cx: &mut TestAppContext) {
    let long = format!("refs/heads/{}", "x".repeat(300));
    let refs = [
        reference(&long, "c0", RefKind::Head),
        reference(&"x".repeat(300), "c0", RefKind::Current),
        reference("refs/heads/dev", "c0", RefKind::Head),
        reference("refs/tags/v1", "c0", RefKind::Tag),
    ];
    let history = History::new(&line(3), &refs, Some("c0".into()), None, false);
    let (view, cx) = shown(cx, history, 600.);
    let fit = cx.update(|window, cx| view.update(cx, |view, _| view.chip_fit(0, window)).unwrap());
    assert_eq!(
        (fit.shown.as_slice(), fit.hidden),
        ([0].as_slice(), 2),
        "{fit:?}"
    );
    let column = bounds(cx, "graph-col-refs");
    let first = bounds(cx, "graph-chip-0-0");
    assert!(
        first.right() <= column.right(),
        "{first:?} inside {column:?}"
    );
    let more = bounds(cx, "graph-chip-more-0");
    assert!(more.right() <= column.right(), "{more:?} inside {column:?}");
    assert!(!drawn(cx, "graph-chip-0-1"));
}

#[test]
fn lane_colours_come_from_the_theme() {
    let theme = Theme::builtin("Catppuccin Latte").unwrap();
    assert_eq!(lane_colour(&theme, 0), theme.ink(theme.palette[4]));
    assert_eq!(lane_colour(&theme, 1), theme.ink(theme.palette[5]));
    assert_eq!(lane_colour(&theme, 5), theme.ink(theme.palette[1]));
    assert_eq!(lane_colour(&theme, 9), theme.ink(theme.palette[10]));
    assert_eq!(lane_colour(&theme, 10), lane_colour(&theme, 0));
}

#[gpui::test]
fn row_text_is_subject_then_description(cx: &mut TestAppContext) {
    let mut described = commit("a", &["b"]);
    described.subject = "feat: x".into();
    described.description = "first line | second line".into();
    let history = History::new(&[described, commit("b", &[])], &[], None, None, false);
    let (_view, cx) = shown(cx, history, 600.);
    let subject = bounds(cx, "graph-subject-0");
    let description = bounds(cx, "graph-desc-0");
    assert!(
        subject.right() <= description.origin.x,
        "{subject:?} then {description:?}"
    );
    assert!(bounds(cx, "graph-msg-0").contains(&subject.center()));
    assert!(
        !drawn(cx, "graph-desc-1"),
        "a subject alone has no description"
    );
}

#[test]
fn lanes_turn_on_the_node_row_with_rounded_corners() {
    let straight = Segment {
        from: Point { lane: 1, row: 2 },
        to: Point { lane: 1, row: 3 },
        colour: 0,
        dangling: false,
        anchor: Some(Anchor::To),
    };
    assert_eq!(
        segment_steps(&straight, 0),
        [
            Step::Move(lane_x(1), row_y(2, 0)),
            Step::Line(lane_x(1), row_y(3, 0))
        ]
    );
    // Leaving a merge: sideways on the merge's row, a corner, down the lane.
    let out = Segment {
        from: Point { lane: 0, row: 4 },
        to: Point { lane: 2, row: 5 },
        anchor: Some(Anchor::From),
        ..straight
    };
    let radius = (LANE_W / 2.).min(ROW_H / 2.);
    assert_eq!(
        segment_steps(&out, 4),
        [
            Step::Move(lane_x(0), row_y(4, 4)),
            Step::Line(lane_x(2) - radius, row_y(4, 4)),
            Step::Arc {
                radius,
                sweep: true,
                x: lane_x(2),
                y: row_y(4, 4) + radius
            },
            Step::Line(lane_x(2), row_y(5, 4)),
        ]
    );
    // Arriving at the commit that absorbs a lane: down, a corner, sideways.
    let late = Segment {
        from: Point { lane: 2, row: 1 },
        to: Point { lane: 0, row: 6 },
        anchor: Some(Anchor::To),
        ..straight
    };
    assert_eq!(
        segment_steps(&late, 0),
        [
            Step::Move(lane_x(2), row_y(1, 0)),
            Step::Line(lane_x(2), row_y(6, 0) - radius),
            Step::Arc {
                radius,
                sweep: true,
                x: lane_x(2) - radius,
                y: row_y(6, 0)
            },
            Step::Line(lane_x(0), row_y(6, 0)),
        ]
    );
}
