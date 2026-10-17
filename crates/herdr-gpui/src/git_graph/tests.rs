#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::model::{Commit, CommitKind};

mod layout;

/// A commit with only the fields the lane engine reads.
fn commit(oid: &str, parents: &[&str]) -> Commit {
    Commit {
        oid: oid.into(),
        parents: parents.iter().map(|parent| (*parent).into()).collect(),
        author: "Ada".into(),
        email: "ada@example.com".into(),
        authored: 0,
        subject: oid.into(),
        description: String::new(),
        kind: CommitKind::Commit,
    }
}

/// hgg's own fixture: two branches, a merge, an octopus merge and a
/// disconnected root.
fn hgg_history() -> Vec<Commit> {
    vec![
        commit("m", &["a2", "b1"]),
        commit("b1", &["a1"]),
        commit("a2", &["o"]),
        commit("o", &["a1", "c1", "d1"]),
        commit("d1", &["a1"]),
        commit("c1", &["a1"]),
        commit("a1", &["root"]),
        commit("root", &[]),
        commit("orphan2", &["orphan1"]),
        commit("orphan1", &[]),
    ]
}

/// Forty commits, child before parent, whose branches open, merge and cross.
fn tangled_history() -> Vec<Commit> {
    const COUNT: usize = 40;
    let oid = |row: usize| format!("c{row:02}");
    (0..COUNT)
        .map(|row| {
            let mut parents = Vec::new();
            if row % 7 == 0 && row + 4 < COUNT {
                parents.extend([oid(row + 2), oid(row + 4)]);
            } else if row % 3 == 0 && row + 3 < COUNT {
                parents.push(oid(row + 3));
            } else if row + 1 < COUNT {
                parents.push(oid(row + 1));
            }
            if row % 11 == 5 && row + 6 < COUNT {
                parents.push(oid(row + 6));
            }
            let parents: Vec<&str> = parents.iter().map(String::as_str).collect();
            commit(&oid(row), &parents)
        })
        .collect()
}
