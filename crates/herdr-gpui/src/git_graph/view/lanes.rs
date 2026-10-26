//! The graph column of one row: its band, the line to its labels, the piece
//! of every lane that crosses it, and its node. Each row paints its own
//! band of the graph, clipped to it, so only visible rows cost anything.

use super::{GitGraphView, chips::LEADER_ALPHA};
use crate::git_graph::{
    geometry::{
        MERGE_R, NODE_R, ROW_H, STASH_SIDE, STROKE, Step, WIP_R, lane_colour, lane_x, segment_steps,
    },
    history::History,
    model::{Commit, CommitKind},
    text::{author_colour, initials},
};
use gpui::{prelude::*, *};

/// A row's tint, and a selected row's.
pub(super) const BAND: f32 = 0.16;
pub(super) const BAND_SELECTED: f32 = 0.42;
/// Lines to commits that are not loaded.
const DANGLING: f32 = 0.6;
const DASH: [f32; 2] = [3., 4.];

fn tint(colour: u32, alpha: f32) -> Hsla {
    let mut tinted: Hsla = rgb(colour).into();
    tinted.a = alpha;
    tinted
}

/// A lane's piece in one row: its steps, colour, and whether it leads to a
/// commit that is not loaded.
struct Piece {
    steps: Vec<Step>,
    colour: u32,
    dangling: bool,
}

/// What a row paints under its node, in the geometry's measures at the
/// face of 12; `scale` takes them to the graph's size.
struct Paint {
    scale: f32,
    node_x: f32,
    colour: u32,
    band: f32,
    leader: bool,
    pieces: Vec<Piece>,
}

fn paint(bounds: Bounds<Pixels>, paint: &Paint, window: &mut Window) {
    let scale = paint.scale;
    let length = |measure: f32| px(measure * scale);
    let at = |x: f32, y: f32| point(bounds.origin.x + length(x), bounds.origin.y + length(y));
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        let band = Bounds::new(
            at(paint.node_x, 1.),
            size(
                (bounds.size.width - length(paint.node_x)).max(px(0.)),
                length(ROW_H - 2.),
            ),
        );
        window.paint_quad(fill(band, tint(paint.colour, paint.band)));
        if paint.leader {
            let leader = Bounds::new(
                at(0., ROW_H / 2. - 0.75),
                size(length((paint.node_x - NODE_R).max(0.)), length(1.5)),
            );
            window.paint_quad(fill(leader, tint(paint.colour, LEADER_ALPHA)));
        }
        for piece in &paint.pieces {
            let mut path = PathBuilder::stroke(length(STROKE));
            if piece.dangling {
                path = path.dash_array(&DASH.map(length));
            }
            for step in &piece.steps {
                match *step {
                    Step::Move(x, y) => path.move_to(at(x, y)),
                    Step::Line(x, y) => path.line_to(at(x, y)),
                    Step::Arc {
                        radius,
                        sweep,
                        x,
                        y,
                    } => path.arc_to(
                        point(length(radius), length(radius)),
                        px(0.),
                        false,
                        sweep,
                        at(x, y),
                    ),
                }
            }
            if let Ok(path) = path.build() {
                let alpha = if piece.dangling { DANGLING } else { 1. };
                window.paint_path(path, tint(piece.colour, alpha));
            }
        }
    });
}

impl GitGraphView {
    /// `row`'s graph column, `width` wide, in its lane's `colour`.
    pub(super) fn render_graph_cell(
        &self,
        history: &History,
        commit: &Commit,
        row: usize,
        width: f32,
        colour: u32,
        leader: bool,
    ) -> AnyElement {
        let theme = &self.look.theme;
        let node_x = history
            .layout
            .nodes
            .get(row)
            .map_or(lane_x(0), |node| lane_x(node.lane));
        let pieces = history
            .segments_in(row..row + 1)
            .filter(|segment| segment.from != segment.to)
            .map(|segment| Piece {
                steps: segment_steps(segment, row),
                colour: lane_colour(theme, segment.colour),
                dangling: segment.dangling,
            })
            .collect();
        let painted = Paint {
            scale: self.look.scale(),
            node_x,
            colour,
            band: self.band(row),
            leader,
            pieces,
        };
        div()
            .id(("graph-graph", row))
            .debug_selector(move || format!("graph-graph-{row}"))
            .relative()
            .flex_none()
            .w(px(width))
            .h_full()
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, (), window, _| paint(bounds, &painted, window),
                )
                .absolute()
                .size_full(),
            )
            .child(self.render_node(commit, row, node_x, colour))
            .into_any_element()
    }

    /// A commit's author bubble, a merge's dot, the uncommitted changes'
    /// dashed ring, or a stash's dashed square.
    fn render_node(&self, commit: &Commit, row: usize, x: f32, colour: u32) -> AnyElement {
        let theme = &self.look.theme;
        let (side, node) = match commit.kind {
            CommitKind::Worktree => (
                WIP_R * 2.,
                div()
                    .rounded_full()
                    .bg(rgb(theme.background))
                    .border_2()
                    .border_dashed()
                    .border_color(rgb(theme.subtext())),
            ),
            CommitKind::Stash => (
                STASH_SIDE,
                div()
                    .rounded(px(self.look.at(3.)))
                    .bg(rgb(theme.background))
                    .border_2()
                    .border_dashed()
                    .border_color(rgb(colour)),
            ),
            CommitKind::Commit if commit.parents.len() > 1 => (
                MERGE_R * 2.,
                div()
                    .rounded_full()
                    .bg(rgb(colour))
                    .border_1()
                    .border_color(rgb(theme.background)),
            ),
            CommitKind::Commit => {
                let key = if commit.email.is_empty() {
                    &commit.author
                } else {
                    &commit.email
                };
                (
                    NODE_R * 2.,
                    div()
                        .rounded_full()
                        .bg(author_colour(key))
                        .border_2()
                        .border_color(rgb(colour))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .debug_selector(move || format!("graph-initials-{row}"))
                                .text_color(white())
                                .text_size(px(self.look.at(9.)))
                                .line_height(px(self.look.at(9.)))
                                .font_weight(FontWeight::BOLD)
                                .child(initials(&commit.author)),
                        ),
                )
            }
        };
        node.debug_selector(move || format!("graph-node-{row}"))
            .absolute()
            .left(px(self.look.at(x - side / 2.)))
            .top(px(self.look.at((ROW_H - side) / 2.)))
            .size(px(self.look.at(side)))
            .into_any_element()
    }
}
