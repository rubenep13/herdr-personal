//! Lanes as hgg lays them out: one node per row, a lane per open branch, and
//! the same places whichever page the history arrived in.

use super::*;
use crate::git_graph::layout::{Anchor, Node, Point, Segment, layout};
use crate::git_graph::model::split_message;
use std::collections::HashSet;

fn node(lane: usize, row: usize, colour: usize) -> Node {
    Node { lane, row, colour }
}

fn point(lane: usize, row: usize) -> Point {
    Point { lane, row }
}

#[test]
fn linear_history_stays_on_one_lane() {
    let commits = [
        commit("c3", &["c2"]),
        commit("c2", &["c1"]),
        commit("c1", &[]),
    ];
    let layout = layout(&commits);
    assert_eq!(layout.nodes, [node(0, 0, 0), node(0, 1, 0), node(0, 2, 0)]);
    assert_eq!(
        layout.segments,
        [
            Segment {
                from: point(0, 0),
                to: point(0, 1),
                colour: 0,
                dangling: false,
                anchor: Some(Anchor::To),
            },
            Segment {
                from: point(0, 1),
                to: point(0, 2),
                colour: 0,
                dangling: false,
                anchor: Some(Anchor::To),
            },
        ]
    );
    assert_eq!(layout.lane_count, 1);
}

#[test]
fn merge_opens_a_lane_and_absorbs_it() {
    let commits = [
        commit("m", &["a", "b"]),
        commit("a", &["base"]),
        commit("b", &["base"]),
        commit("base", &[]),
    ];
    let layout = layout(&commits);
    assert_eq!(
        layout.nodes,
        [node(0, 0, 0), node(0, 1, 0), node(1, 2, 1), node(0, 3, 0)]
    );
    assert!(layout.segments.contains(&Segment {
        from: point(0, 0),
        to: point(1, 1),
        colour: 1,
        dangling: false,
        anchor: Some(Anchor::From),
    }));
    assert!(layout.segments.contains(&Segment {
        from: point(1, 2),
        to: point(0, 3),
        colour: 1,
        dangling: false,
        anchor: Some(Anchor::To),
    }));
    assert_eq!(layout.lane_count, 2);
}

#[test]
fn octopus_merge_opens_one_lane_per_extra_parent() {
    let commits = [
        commit("m", &["a", "b", "c"]),
        commit("a", &["base"]),
        commit("b", &["base"]),
        commit("c", &["base"]),
        commit("base", &[]),
    ];
    let layout = layout(&commits);
    assert_eq!(layout.lane_count, 3);
    let opened = layout
        .segments
        .iter()
        .filter(|segment| segment.from == point(0, 0) && segment.anchor == Some(Anchor::From))
        .count();
    assert_eq!(opened, 2);
}

#[test]
fn missing_parents_dangle_at_the_end() {
    let commits = [commit("tip", &["mid"]), commit("mid", &["unloaded"])];
    let layout = layout(&commits);
    let dangling: Vec<&Segment> = layout.segments.iter().filter(|s| s.dangling).collect();
    assert_eq!(dangling.len(), 1);
    assert_eq!(dangling[0].from, point(0, 1));
    assert_eq!(dangling[0].to, point(0, commits.len()));
}

#[test]
fn paging_never_moves_loaded_rows() {
    let all = tangled_history();
    let full = layout(&all);
    assert!(full.lane_count > 2, "the fixture must cross lanes");
    for loaded in 1..=all.len() {
        let partial = layout(&all[..loaded]);
        assert_eq!(partial.nodes, full.nodes[..loaded], "{loaded} rows loaded");
        for segment in partial.segments.iter().filter(|s| s.dangling) {
            assert_eq!(segment.to.row, loaded);
        }
    }
}

#[test]
fn no_two_nodes_share_a_place() {
    for commits in [hgg_history(), tangled_history()] {
        let layout = layout(&commits);
        let places: HashSet<(usize, usize)> = layout
            .nodes
            .iter()
            .map(|node| (node.lane, node.row))
            .collect();
        assert_eq!(places.len(), commits.len());
        // Every parent edge reaches the parent's node.
        let row_of = |oid: &str| commits.iter().position(|c| c.oid == oid).unwrap();
        for commit in &commits {
            for parent in &commit.parents {
                let parent = &layout.nodes[row_of(parent)];
                assert!(
                    layout
                        .segments
                        .iter()
                        .any(|s| s.to == point(parent.lane, parent.row)),
                    "{} reaches {}",
                    commit.oid,
                    parent.row
                );
            }
        }
    }
}

#[test]
fn split_message_joins_the_body() {
    assert_eq!(
        split_message("feat: x\n\nfirst line\n  second line  \n"),
        ("feat: x".to_owned(), "first line | second line".to_owned())
    );
    assert_eq!(
        split_message("only a subject"),
        ("only a subject".to_owned(), String::new())
    );
}
