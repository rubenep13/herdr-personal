//! Reading a real repository: its refs, its history in date-ordered pages and
//! its uncommitted changes, whatever odd state it is in.

use super::*;
use crate::git_graph::Error;
use crate::git_graph::git::{PAGE, Snapshot};
use crate::git_graph::layout::layout;
use crate::git_graph::model::{Ref, RefKind};
use std::collections::{HashMap, HashSet};

fn snapshot(repo: &Repo) -> Snapshot {
    repo.snapshot(&AtomicBool::new(false)).unwrap()
}

/// Every commit the snapshot reaches, page after page.
fn history(repo: &Repo) -> Vec<Commit> {
    let snapshot = snapshot(repo);
    let mut commits = Vec::new();
    let mut skip = 0;
    loop {
        let page = repo.page(&snapshot, skip, &AtomicBool::new(false)).unwrap();
        commits.extend(page.commits);
        if !page.has_more {
            return commits;
        }
        skip += PAGE;
    }
}

fn reference(name: &str, target: &str, kind: RefKind) -> Ref {
    Ref {
        name: name.into(),
        target: target.into(),
        kind,
    }
}

#[test]
fn snapshot_lists_heads_remotes_tags_and_head() {
    let fixture = Fixture::with_commits(2);
    let first = fixture.git(&["rev-parse", "HEAD~1"]).trim().to_owned();
    let head = fixture.head();
    fixture.git(&["branch", "feature", "HEAD~1"]);
    fixture.git(&["tag", "-a", "v1", "-m", "release"]);
    fixture.git(&["update-ref", "refs/remotes/origin/main", &first]);
    fixture.git(&[
        "symbolic-ref",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
    ]);

    let snapshot = snapshot(&fixture.open());
    let mut refs = snapshot.refs.clone();
    refs.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(
        refs,
        [
            reference("main", &head, RefKind::Current),
            reference("refs/heads/feature", &first, RefKind::Head),
            reference("refs/heads/main", &head, RefKind::Head),
            reference("refs/remotes/origin/main", &first, RefKind::Remote),
            reference("refs/tags/v1", &head, RefKind::Tag),
        ]
    );
    assert_eq!(snapshot.head.as_deref(), Some(head.as_str()));
    let unique: HashSet<&String> = snapshot.tips.iter().collect();
    assert_eq!(unique.len(), snapshot.tips.len(), "{:?}", snapshot.tips);
    assert_eq!(unique, HashSet::from([&head, &first]));
}

#[test]
fn checkpoint_refs_are_never_tips() {
    let fixture = Fixture::with_commits(1);
    let hidden = fixture
        .git(&[
            "commit-tree",
            "HEAD^{tree}",
            "-p",
            "HEAD",
            "-m",
            "checkpoint",
        ])
        .trim()
        .to_owned();
    fixture.git(&["update-ref", "refs/herdr-gpui/checkpoints/x", &hidden]);

    let repo = fixture.open();
    assert!(!snapshot(&repo).tips.contains(&hidden));
    assert!(history(&repo).iter().all(|commit| commit.oid != hidden));
}

#[test]
fn pages_are_date_ordered_and_bounded() {
    const COUNT: usize = 1203;
    let fixture = Fixture::new();
    let mut stream = String::new();
    for index in 1..=COUNT {
        let at = 1_700_000_000 + index;
        stream.push_str(&format!(
            "commit refs/heads/main\nmark :{index}\n\
             author Test <test@example.invalid> {at} +0000\n\
             committer Test <test@example.invalid> {at} +0000\n\
             data <<END\ncommit {index}\nEND\n"
        ));
        if index > 1 {
            stream.push_str(&format!("from :{}\n", index - 1));
        }
        stream.push('\n');
    }
    fixture.import(&stream);

    let repo = fixture.open();
    let snapshot = snapshot(&repo);
    let first = repo.page(&snapshot, 0, &AtomicBool::new(false)).unwrap();
    assert_eq!(first.commits.len(), PAGE);
    assert!(first.has_more);
    assert_eq!(first.commits[0].subject, format!("commit {COUNT}"));
    let last = repo.page(&snapshot, 1000, &AtomicBool::new(false)).unwrap();
    assert_eq!(last.commits.len(), 203);
    assert!(!last.has_more);

    let all = history(&repo);
    assert_eq!(all.len(), COUNT);
    let row: HashMap<&str, usize> = all
        .iter()
        .enumerate()
        .map(|(row, commit)| (commit.oid.as_str(), row))
        .collect();
    for (child, commit) in all.iter().enumerate() {
        for parent in &commit.parents {
            assert!(
                row[parent.as_str()] > child,
                "{} before its parent",
                commit.oid
            );
        }
    }
}

#[test]
fn stash_rows_keep_only_their_first_parent() {
    let fixture = Fixture::with_commits(2);
    fixture.write("file.txt", "changed\n");
    fixture.write("untracked.txt", "new\n");
    fixture.git(&["stash", "push", "-u", "-q"]);
    let stash = fixture.git(&["rev-parse", "refs/stash"]).trim().to_owned();
    let helpers: Vec<String> = fixture
        .git(&["rev-list", "--no-walk", "--parents", "refs/stash"])
        .split_whitespace()
        .skip(2)
        .map(str::to_owned)
        .collect();
    assert_eq!(helpers.len(), 2, "an index and an untracked commit");

    let repo = fixture.open();
    let snapshot = snapshot(&repo);
    assert_eq!(snapshot.stash_helpers, helpers.iter().cloned().collect());
    assert!(
        snapshot
            .refs
            .contains(&reference("stash@{0}", &stash, RefKind::Stash))
    );
    let rows = history(&repo);
    let row = rows.iter().find(|commit| commit.oid == stash).unwrap();
    assert_eq!(row.kind, CommitKind::Stash);
    assert_eq!(row.parents, [fixture.head()]);
    assert!(rows.iter().all(|commit| !helpers.contains(&commit.oid)));
}

#[test]
fn worktree_counts_staged_unstaged_untracked_and_conflicts() {
    let fixture = Fixture::new();
    for name in ["a.txt", "b.txt", "c.txt"] {
        fixture.write(name, "one\n");
    }
    fixture.commit("base");
    fixture.write("a.txt", "staged\n");
    fixture.git(&["add", "a.txt"]);
    fixture.write("b.txt", "unstaged\n");
    fixture.write("c.txt", "unstaged\n");
    fixture.write("d.txt", "untracked\n");

    let counts = fixture.open().worktree(&AtomicBool::new(false)).unwrap();
    assert_eq!(
        (
            counts.staged,
            counts.unstaged,
            counts.untracked,
            counts.conflicted
        ),
        (1, 2, 1, 0)
    );

    let conflict = Fixture::with_commits(1);
    conflict.git(&["switch", "-qc", "other"]);
    conflict.write("file.txt", "theirs\n");
    conflict.commit("theirs");
    conflict.git(&["switch", "-q", "main"]);
    conflict.write("file.txt", "ours\n");
    conflict.commit("ours");
    let merge = conflict
        .command(conflict.path())
        .args(["merge", "-q", "other"])
        .output()
        .unwrap();
    assert!(!merge.status.success(), "the merge must conflict");
    let counts = conflict.open().worktree(&AtomicBool::new(false)).unwrap();
    assert_eq!(counts.conflicted, 1);
}

#[test]
fn empty_repository_has_no_history() {
    let fixture = Fixture::new();
    let repo = fixture.open();
    let snapshot = snapshot(&repo);
    assert!(snapshot.tips.is_empty());
    assert_eq!(snapshot.head, None);
    let page = repo.page(&snapshot, 0, &AtomicBool::new(false)).unwrap();
    assert!(page.commits.is_empty());
    assert!(!page.has_more);
}

#[test]
fn folder_outside_a_repository_is_an_error() {
    let fixture = Fixture::new();
    let outside = fixture.temp.path().join("home");
    let opened = Repo::open(
        fixture.host(),
        outside.to_str().unwrap(),
        &AtomicBool::new(false),
    );
    assert!(matches!(opened, Err(Error::NotARepository)), "{opened:?}");
    let missing = fixture.temp.path().join("gone");
    let opened = Repo::open(
        fixture.host(),
        missing.to_str().unwrap(),
        &AtomicBool::new(false),
    );
    assert!(matches!(opened, Err(Error::NotARepository)), "{opened:?}");
}

#[test]
fn opening_a_subfolder_reads_the_whole_repository() {
    let fixture = Fixture::with_commits(1);
    std::fs::create_dir_all(fixture.path().join("src/deep")).unwrap();
    let repo = open_at(fixture.host(), &fixture.path().join("src/deep"));
    assert_eq!(
        Path::new(repo.root()).canonicalize().unwrap(),
        fixture.path().canonicalize().unwrap()
    );
}

#[test]
fn shallow_clone_parents_dangle() {
    let fixture = Fixture::with_commits(4);
    let shallow = fixture.temp.path().join("shallow");
    let url = format!("file://{}", fixture.path().display());
    fixture.git_in(
        fixture.temp.path(),
        &[
            "clone",
            "-q",
            "--depth",
            "2",
            &url,
            shallow.to_str().unwrap(),
        ],
    );
    let commits = history(&open_at(fixture.host(), &shallow));
    assert_eq!(commits.len(), 2);
    let layout = layout(&commits);
    let last = layout.segments.last().unwrap();
    assert!(last.dangling, "{:?}", layout.segments);
    assert_eq!(last.to.row, 2);
}

#[test]
fn commit_text_is_shown_without_control_characters() {
    let fixture = Fixture::new();
    fixture.write("file.txt", "x\n");
    fixture.commit("\u{1b}[31mred\u{1b}[0m\tx\n\nbody \u{202e}reversed\u{7}");
    let commits = history(&fixture.open());
    let commit = &commits[0];
    for text in [&commit.subject, &commit.description, &commit.author] {
        assert!(
            !text.chars().any(crate::notifications::unsafe_char),
            "{text:?}"
        );
    }
    assert!(commit.subject.contains("red"));
    assert!(commit.description.contains("reversed"));
}
