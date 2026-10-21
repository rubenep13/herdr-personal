//! hgg's measures (`geometry.ts`) and the shape of a lane's line: a lane
//! runs straight down; where it meets another it turns at a right angle with
//! a rounded corner, and the horizontal run lies on the row of the node it
//! belongs to.

use super::layout::{Anchor, Segment};
use crate::config::Theme;

/// GitKraken's proportions: tall rows, avatar-sized nodes, wide lanes.
pub(crate) const ROW_H: f32 = 30.;
pub(crate) const LANE_W: f32 = 22.;
pub(crate) const GRAPH_PAD: f32 = 12.;
/// A commit's author bubble.
pub(crate) const NODE_R: f32 = 10.;
/// A merge's dot.
pub(crate) const MERGE_R: f32 = 5.;
/// The uncommitted changes' ring.
pub(crate) const WIP_R: f32 = 9.;
/// A stash's square.
pub(crate) const STASH_SIDE: f32 = 14.;
/// Never narrower than its column header.
pub(crate) const GRAPH_MIN_W: f32 = 64.;
pub(crate) const STROKE: f32 = 2.6;

pub(crate) const REFS_DEFAULT: f32 = 190.;
pub(crate) const MESSAGE_MIN: f32 = 220.;
pub(crate) const AUTHOR_W: f32 = 150.;
pub(crate) const DATE_W: f32 = 118.;
pub(crate) const SHA_W: f32 = 78.;

/// Branch labels: height, padding each side, text size, icon size and the
/// room an icon takes.
pub(crate) const CHIP_H: f32 = 22.;
pub(crate) const CHIP_PAD: f32 = 7.;
pub(crate) const CHIP_TEXT: f32 = 12.;
pub(crate) const ICON: f32 = 13.;
pub(crate) const ICON_STEP: f32 = 16.;
pub(crate) const CHIP_GAP: f32 = 4.;
/// The shortest line left between the labels and the graph.
pub(crate) const MIN_LEADER: f32 = 14.;
/// The label column's left padding.
pub(crate) const REFS_INSET: f32 = 14.;

/// Lanes cycle through the theme's blue, magenta, cyan, green, yellow and
/// red, then their bright variants.
const LANE_ORDER: [usize; 10] = [4, 5, 6, 2, 3, 1, 12, 13, 14, 10];

pub(crate) fn lane_colour(theme: &Theme, colour: usize) -> u32 {
    theme.ink(theme.palette[LANE_ORDER[colour % LANE_ORDER.len()]])
}

pub(crate) fn lane_x(lane: usize) -> f32 {
    GRAPH_PAD + lane as f32 * LANE_W + LANE_W / 2.
}

/// The middle of `row`, measured from the top of `origin`.
pub(crate) fn row_y(row: usize, origin: usize) -> f32 {
    (row as f32 - origin as f32) * ROW_H + ROW_H / 2.
}

pub(crate) fn graph_width(lane_count: usize) -> f32 {
    (GRAPH_PAD * 2. + lane_count.max(1) as f32 * LANE_W).max(GRAPH_MIN_W)
}

/// One step of a lane's line, in pixels from the left of the graph and the
/// top of the origin row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Step {
    Move(f32, f32),
    Line(f32, f32),
    /// A quarter circle to `(x, y)`, clockwise when `sweep` is set.
    Arc {
        radius: f32,
        sweep: bool,
        x: f32,
        y: f32,
    },
}

/// The steps of `segment` relative to row `origin`, as hgg's `segmentPath`
/// draws them: leaving a merge (`From`) it goes out sideways then down;
/// arriving at the commit that absorbs it (`To`) it comes down then sideways.
pub(crate) fn segment_steps(segment: &Segment, origin: usize) -> Vec<Step> {
    let (x1, x2) = (lane_x(segment.from.lane), lane_x(segment.to.lane));
    let (y1, y2) = (
        row_y(segment.from.row, origin),
        row_y(segment.to.row, origin),
    );
    if segment.from.lane == segment.to.lane {
        return vec![Step::Move(x1, y1), Step::Line(x2, y2)];
    }
    let direction = if x2 > x1 { 1. } else { -1. };
    let radius = (LANE_W / 2.).min((y2 - y1) / 2.).min((x2 - x1).abs() / 2.);
    if segment.anchor == Some(Anchor::From) {
        vec![
            Step::Move(x1, y1),
            Step::Line(x2 - direction * radius, y1),
            Step::Arc {
                radius,
                sweep: direction > 0.,
                x: x2,
                y: y1 + radius,
            },
            Step::Line(x2, y2),
        ]
    } else {
        vec![
            Step::Move(x1, y1),
            Step::Line(x1, y2 - radius),
            Step::Arc {
                radius,
                sweep: direction < 0.,
                x: x1 + direction * radius,
                y: y2,
            },
            Step::Line(x2, y2),
        ]
    }
}
