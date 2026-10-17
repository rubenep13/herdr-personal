//! A deterministic, compact lane layout for a history ordered child before
//! parent. Ported from `@web-git-graph/web` 1.0.7 `src/layout.ts`
//! (MIT, © kian-zh; see NOTICE), without its incremental state: the whole
//! loaded history is laid out again on every page, and since a row depends
//! only on the rows above it, loading more never moves a row already shown.

use super::model::Commit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Point {
    pub(crate) lane: usize,
    pub(crate) row: usize,
}

/// Which end a lane change belongs to: a line leaving a merge hugs its
/// `From` node, a line a commit absorbs hugs its `To` node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Anchor {
    From,
    To,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Node {
    pub(crate) lane: usize,
    pub(crate) row: usize,
    pub(crate) colour: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Segment {
    pub(crate) from: Point,
    pub(crate) to: Point,
    pub(crate) colour: usize,
    /// A line to a parent that is not loaded, drawn to the end of the rows.
    pub(crate) dangling: bool,
    pub(crate) anchor: Option<Anchor>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Layout {
    /// One per commit, in row order.
    pub(crate) nodes: Vec<Node>,
    pub(crate) segments: Vec<Segment>,
    pub(crate) lane_count: usize,
}

/// A lane waiting for the commit it leads to.
struct Lane<'a> {
    lane: usize,
    target: &'a str,
    colour: usize,
    from_row: usize,
}

fn first_free_lane(active: &[Lane], reserved: &[usize]) -> usize {
    (0..)
        .find(|lane| !reserved.contains(lane) && !active.iter().any(|open| open.lane == *lane))
        .unwrap_or_default()
}

pub(crate) fn layout(commits: &[Commit]) -> Layout {
    let mut active: Vec<Lane> = Vec::new();
    let mut next_colour = 0;
    let mut nodes = Vec::with_capacity(commits.len());
    let mut segments = Vec::new();
    let mut lane_count = 1;

    for (row, commit) in commits.iter().enumerate() {
        let mut matching: Vec<(usize, usize, usize)> = active
            .iter()
            .filter(|lane| lane.target == commit.oid)
            .map(|lane| (lane.lane, lane.colour, lane.from_row))
            .collect();
        matching.sort_by_key(|(lane, ..)| *lane);
        let mut reserved = Vec::new();
        let (lane, colour) = match matching.first() {
            Some((lane, colour, _)) => (*lane, *colour),
            None => {
                next_colour += 1;
                (first_free_lane(&active, &reserved), next_colour - 1)
            }
        };
        lane_count = lane_count.max(lane + 1);
        nodes.push(Node { lane, row, colour });

        // Lanes passing by continue straight down to this row; the original
        // emits a zero-length piece when one has just opened here.
        for open in active.iter_mut().filter(|open| open.target != commit.oid) {
            segments.push(Segment {
                from: Point {
                    lane: open.lane,
                    row: open.from_row,
                },
                to: Point {
                    lane: open.lane,
                    row,
                },
                colour: open.colour,
                dangling: false,
                anchor: None,
            });
            open.from_row = row;
        }
        for (from_lane, from_colour, from_row) in matching {
            segments.push(Segment {
                from: Point {
                    lane: from_lane,
                    row: from_row,
                },
                to: Point { lane, row },
                colour: from_colour,
                dangling: false,
                anchor: Some(Anchor::To),
            });
        }
        active.retain(|open| open.target != commit.oid);

        let mut parents = commit.parents.iter().filter(|parent| !parent.is_empty());
        if let Some(first) = parents.next() {
            active.push(Lane {
                lane,
                target: first,
                colour,
                from_row: row,
            });
            reserved.push(lane);
        }
        for parent in parents {
            if let Some(existing) = active.iter().find(|open| open.target == parent) {
                segments.push(Segment {
                    from: Point { lane, row },
                    to: Point {
                        lane: existing.lane,
                        row: row + 1,
                    },
                    colour: existing.colour,
                    dangling: false,
                    anchor: Some(Anchor::From),
                });
                continue;
            }
            let opened = first_free_lane(&active, &reserved);
            reserved.push(opened);
            lane_count = lane_count.max(opened + 1);
            let opened_colour = next_colour;
            next_colour += 1;
            // The curve lands on the new lane at the next row, so the lane's
            // own line starts there rather than beside the merge node.
            active.push(Lane {
                lane: opened,
                target: parent,
                colour: opened_colour,
                from_row: row + 1,
            });
            segments.push(Segment {
                from: Point { lane, row },
                to: Point {
                    lane: opened,
                    row: row + 1,
                },
                colour: opened_colour,
                dangling: false,
                anchor: Some(Anchor::From),
            });
        }
    }

    let end = commits.len();
    for open in active.iter().filter(|open| open.from_row < end) {
        segments.push(Segment {
            from: Point {
                lane: open.lane,
                row: open.from_row,
            },
            to: Point {
                lane: open.lane,
                row: end,
            },
            colour: open.colour,
            dangling: true,
            anchor: None,
        });
    }

    Layout {
        nodes,
        segments,
        lane_count,
    }
}
