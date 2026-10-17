//! A loaded history as the graph draws it: its rows, their lanes and labels,
//! and an index of the lines that cross any window of rows, as hgg's
//! `HistoryModel` keeps one, so drawing touches only what is visible.

use super::{
    chips::{Chip, build_chips},
    git::WorktreeCounts,
    layout::{Layout, Segment, layout},
    model::{Commit, CommitKind, Ref, WORKTREE_OID},
};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

/// A line spanning more rows than this is checked against every window
/// rather than filed under each row it crosses.
const LONG: usize = 64;

#[derive(Debug, Default)]
pub(crate) struct History {
    /// The rows, child before parent, with the uncommitted changes first.
    pub(crate) commits: Vec<Commit>,
    pub(crate) layout: Layout,
    pub(crate) chips: HashMap<String, Vec<Chip>>,
    pub(crate) head: Option<String>,
    pub(crate) worktree: Option<WorktreeCounts>,
    pub(crate) has_more: bool,
    /// The repository's top-level folder on its host; empty until read.
    pub(crate) root: String,
    rows: HashMap<String, usize>,
    by_row: Vec<Vec<usize>>,
    long: Vec<usize>,
}

impl WorktreeCounts {
    pub(crate) fn total(&self) -> usize {
        self.staged + self.unstaged + self.untracked + self.conflicted
    }
}

/// The row for uncommitted changes, drawn above the commit it sits on.
fn worktree_row(head: Option<&str>, counts: WorktreeCounts) -> Commit {
    let parts: Vec<String> = [
        (counts.conflicted, "conflicted"),
        (counts.staged, "staged"),
        (counts.unstaged, "unstaged"),
        (counts.untracked, "untracked"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, what)| format!("{count} {what}"))
    .collect();
    Commit {
        oid: WORKTREE_OID.to_owned(),
        parents: head.map(str::to_owned).into_iter().collect(),
        author: String::new(),
        email: String::new(),
        authored: 0,
        subject: "// WIP".to_owned(),
        description: parts.join(", "),
        kind: CommitKind::Worktree,
    }
}

impl History {
    pub(crate) fn new(
        loaded: &[Commit],
        refs: &[Ref],
        head: Option<String>,
        worktree: Option<WorktreeCounts>,
        has_more: bool,
    ) -> Self {
        let mut commits = Vec::with_capacity(loaded.len() + 1);
        if let Some(counts) = worktree.filter(|counts| counts.total() > 0) {
            commits.push(worktree_row(head.as_deref(), counts));
        }
        commits.extend_from_slice(loaded);
        let layout = layout(&commits);
        let rows = commits
            .iter()
            .enumerate()
            .map(|(row, commit)| (commit.oid.clone(), row))
            .collect();
        let mut by_row: Vec<Vec<usize>> = vec![Vec::new(); commits.len() + 1];
        let mut long = Vec::new();
        for (index, segment) in layout.segments.iter().enumerate() {
            let from = segment.from.row.min(segment.to.row);
            let to = segment.from.row.max(segment.to.row);
            if to - from > LONG {
                long.push(index);
                continue;
            }
            for row in from..=from.max(to.saturating_sub(1)) {
                if let Some(list) = by_row.get_mut(row) {
                    list.push(index);
                }
            }
        }
        Self {
            chips: build_chips(refs),
            commits,
            layout,
            head,
            worktree,
            has_more,
            root: String::new(),
            rows,
            by_row,
            long,
        }
    }

    pub(crate) fn row_of(&self, oid: &str) -> Option<usize> {
        self.rows.get(oid).copied()
    }

    /// The lines that cross `rows`, each once.
    pub(crate) fn segments_in(&self, rows: Range<usize>) -> impl Iterator<Item = &Segment> {
        let mut seen = HashSet::new();
        let start = rows.start.saturating_sub(1);
        let end = rows.end.min(self.by_row.len());
        let near = self
            .by_row
            .get(start..end)
            .unwrap_or_default()
            .iter()
            .flatten();
        let long = self.long.iter().filter(move |index| {
            let segment = &self.layout.segments[**index];
            let from = segment.from.row.min(segment.to.row);
            let to = segment.from.row.max(segment.to.row);
            to >= rows.start && from < rows.end
        });
        near.chain(long)
            .filter(move |index| seen.insert(**index))
            .map(|index| &self.layout.segments[*index])
    }
}
