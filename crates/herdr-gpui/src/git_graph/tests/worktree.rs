//! The uncommitted changes' detail: the WIP row opens their sections and
//! files, the upstream reads with ahead and behind, each section's diff
//! reads the right side, and `w` goes to the WIP row.

use super::*;
use crate::git_graph::{
    detail::FileChange,
    git::WorktreeCounts,
    history::History,
    view::GitGraphView,
    worker::Reply,
    worktree::{Section, WorktreeStatus, upstream_label},
};
use crate::review::Kind;
use gpui::{TestAppContext, VisualTestContext};
use std::sync::Arc;

fn file(path: &str, kind: char) -> FileChange {
    FileChange {
        path: path.into(),
        old_path: None,
        kind,
        additions: None,
        deletions: None,
        binary: false,
        git_path: path.into(),
        git_old_path: None,
    }
}

fn wip_history() -> History {
    let counts = WorktreeCounts {
        staged: 1,
        unstaged: 1,
        untracked: 1,
        conflicted: 1,
    };
    History::new(&line(4), &[], Some("c0".into()), Some(counts), false)
}

fn focus(view: &gpui::Entity<GitGraphView>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let handle = view.read(cx).focus_handle.clone();
        window.focus(&handle, cx);
    });
}

#[gpui::test]
fn wip_row_opens_the_worktree_detail(cx: &mut TestAppContext) {
    let (view, cx) = shown(cx, wip_history(), 700.);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.open_detail(0, cx);
            let status = WorktreeStatus {
                upstream: Some("origin/main".into()),
                ahead: 0,
                behind: 0,
                files: vec![
                    (Section::Conflicted, file("both.txt", 'U')),
                    (Section::Staged, file("a.txt", 'M')),
                    (Section::Unstaged, file("b.txt", 'M')),
                    (Section::Untracked, file("new.txt", '?')),
                ],
            };
            let asked = view.last_detail();
            assert!(view.receive(asked, Reply::Worktree(Arc::new(status)), cx));
        })
    });
    draw(cx);
    for section in ["conflicted", "staged", "unstaged", "untracked"] {
        assert!(drawn(cx, &format!("graph-section-{section}")), "{section}");
    }
    for index in 0..4 {
        assert!(drawn(cx, &format!("graph-file-{index}")), "file {index}");
    }
    assert!(drawn(cx, "graph-upstream"));
}

#[test]
fn upstream_shows_ahead_and_behind() {
    let status = |upstream: Option<&str>, ahead, behind| WorktreeStatus {
        upstream: upstream.map(str::to_owned),
        ahead,
        behind,
        files: Vec::new(),
    };
    assert_eq!(
        upstream_label(&status(Some("origin/main"), 2, 1)),
        "origin/main \u{00b7} \u{2191}2 \u{2193}1"
    );
    assert_eq!(
        upstream_label(&status(Some("origin/main"), 0, 0)),
        "origin/main"
    );
    assert_eq!(upstream_label(&status(None, 0, 0)), "No upstream");
}

#[cfg(unix)]
#[test]
fn upstream_is_read_with_its_counts() {
    let fixture = Fixture::with_commits(1);
    let base = fixture.head();
    fixture.git(&["switch", "-qc", "theirs"]);
    fixture.write("theirs.txt", "theirs\n");
    let theirs = fixture.commit("theirs");
    fixture.git(&["switch", "-q", "main"]);
    fixture.write("one.txt", "one\n");
    fixture.commit("one");
    fixture.write("two.txt", "two\n");
    fixture.commit("two");
    fixture.git(&["update-ref", "refs/remotes/origin/main", &theirs]);
    fixture.git(&["remote", "add", "origin", "/nonexistent/origin"]);
    fixture.git(&["config", "branch.main.remote", "origin"]);
    fixture.git(&["config", "branch.main.merge", "refs/heads/main"]);
    let _ = base;
    let status = fixture
        .open()
        .worktree_status(&AtomicBool::new(false))
        .unwrap();
    assert_eq!(status.upstream.as_deref(), Some("origin/main"));
    assert_eq!((status.ahead, status.behind), (2, 1));
}

#[cfg(unix)]
#[test]
fn section_diffs_read_the_right_side() {
    let fixture = Fixture::new();
    fixture.write("a.txt", "v1\n");
    fixture.commit("base");
    fixture.write("a.txt", "v2\n");
    fixture.git(&["add", "a.txt"]);
    fixture.write("a.txt", "v3\n");
    fixture.write("new.txt", "fresh\nlines\n");
    let repo = fixture.open();
    let cancelled = AtomicBool::new(false);

    let status = repo.worktree_status(&cancelled).unwrap();
    let sections: Vec<(Section, &str, char)> = status
        .files
        .iter()
        .map(|(section, file)| (*section, file.path.as_str(), file.kind))
        .collect();
    assert!(
        sections.contains(&(Section::Staged, "a.txt", 'M')),
        "{sections:?}"
    );
    assert!(
        sections.contains(&(Section::Unstaged, "a.txt", 'M')),
        "{sections:?}"
    );
    assert!(
        sections.contains(&(Section::Untracked, "new.txt", '?')),
        "{sections:?}"
    );

    let text = |section: Section, path: &str, kind: Kind| -> Vec<String> {
        let diff = repo.worktree_diff(section, path, None, &cancelled).unwrap();
        diff.lines
            .iter()
            .filter(|line| line.kind == kind)
            .map(|line| diff.lines.text_of(line).to_owned())
            .collect()
    };
    assert_eq!(text(Section::Staged, "a.txt", Kind::Removed), ["v1"]);
    assert_eq!(text(Section::Staged, "a.txt", Kind::Added), ["v2"]);
    assert_eq!(text(Section::Unstaged, "a.txt", Kind::Removed), ["v2"]);
    assert_eq!(text(Section::Unstaged, "a.txt", Kind::Added), ["v3"]);
    assert_eq!(
        text(Section::Untracked, "new.txt", Kind::Added),
        ["fresh", "lines"]
    );

    // An untracked file is cut like any diff.
    let big: String = (0..5000).map(|line| format!("{line}\n")).collect();
    fixture.write("big.txt", &big);
    let diff = repo
        .worktree_diff(Section::Untracked, "big.txt", None, &cancelled)
        .unwrap();
    assert!(diff.truncated);
    assert!(diff.lines.len() <= crate::git_graph::detail::MAX_DIFF_LINES);
}

#[gpui::test]
fn w_selects_the_worktree_row(cx: &mut TestAppContext) {
    let (view, cx) = shown(cx, wip_history(), 600.);
    focus(&view, cx);
    cx.simulate_keystrokes("j j w");
    assert_eq!(view.read_with(cx, |view, _| view.selected_row()), Some(0));
    // Without uncommitted changes there is no row to go to.
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let clean = History::new(&line(4), &[], Some("c0".into()), None, false);
            assert!(view.receive(0, Reply::History(Arc::new(clean)), cx));
        })
    });
    // The WIP row went, so the selection fell back to HEAD's row (0).
    assert_eq!(view.read_with(cx, |view, _| view.selected_row()), Some(0));
    cx.simulate_keystrokes("j j w");
    assert_eq!(view.read_with(cx, |view, _| view.selected_row()), Some(2));
}
