//! A binary file's diff says the file is binary rather than that nothing
//! changed: an untracked one read whole, and one in a commit.

use super::*;
use crate::git_graph::{
    detail::FileChange,
    git::WorktreeCounts,
    history::History,
    worker::Reply,
    worktree::{Section, WorktreeStatus},
};
use crate::review::{BINARY_NOTE, Lines};
use gpui::{Modifiers, TestAppContext};
use std::sync::Arc;

fn untracked(path: &str) -> FileChange {
    FileChange {
        path: path.into(),
        old_path: None,
        kind: '?',
        additions: None,
        deletions: None,
        binary: false,
        git_path: path.into(),
        git_old_path: None,
    }
}

#[cfg(unix)]
#[test]
fn an_untracked_binary_file_reads_as_binary() {
    let fixture = Fixture::new();
    fixture.write("a.txt", "text\n");
    fixture.commit("base");
    fixture.write("logo.png", "PNG\r\n\0\0\0IHDR\n");
    fixture.write("new.txt", "fresh\n");
    let repo = fixture.open();
    let cancelled = AtomicBool::new(false);

    let status = repo.worktree_status(&cancelled).unwrap();
    assert!(
        status
            .files
            .iter()
            .any(|(section, file)| *section == Section::Untracked && file.path == "logo.png"),
        "{:?}",
        status.files
    );
    let diff = repo
        .worktree_diff(Section::Untracked, "logo.png", None, &cancelled)
        .unwrap();
    assert!(diff.binary);
    assert!(diff.lines.is_empty());
    let text = repo
        .worktree_diff(Section::Untracked, "new.txt", None, &cancelled)
        .unwrap();
    assert!(!text.binary);
    assert!(!text.lines.is_empty());
}

#[cfg(unix)]
#[test]
fn a_committed_binary_file_reads_as_binary() {
    let fixture = Fixture::new();
    fixture.write("a.txt", "text\n");
    fixture.commit("base");
    fixture.write("bin.dat", "\0\x01\x02");
    let head = fixture.commit("binary");
    let diff = fixture
        .open()
        .file_diff(&head, "bin.dat", None, &AtomicBool::new(false))
        .unwrap();
    assert!(diff.binary);
    assert!(diff.lines.is_empty());
}

#[gpui::test]
fn an_untracked_binary_file_says_it_is_binary(cx: &mut TestAppContext) {
    let counts = WorktreeCounts {
        untracked: 2,
        ..WorktreeCounts::default()
    };
    let history = History::new(&line(2), &[], Some("c0".into()), Some(counts), false);
    let (view, cx) = shown(cx, history, 700.);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.open_detail(0, cx);
            let status = WorktreeStatus {
                upstream: None,
                ahead: 0,
                behind: 0,
                files: vec![
                    (Section::Untracked, untracked("logo.png")),
                    (Section::Untracked, untracked("empty.txt")),
                ],
            };
            assert!(view.receive(0, Reply::Worktree(Arc::new(status)), cx));
        })
    });
    draw(cx);
    let diff = |path: &str, binary| Reply::WorktreeDiff {
        section: Section::Untracked,
        path: path.into(),
        lines: Arc::new(Lines::default().finish()),
        spans: Arc::default(),
        truncated: false,
        binary,
    };

    let file = bounds(cx, "graph-file-0").center();
    cx.simulate_click(file, Modifiers::none());
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            assert!(view.receive(0, diff("logo.png", true), cx))
        })
    });
    draw(cx);
    assert!(drawn(cx, "graph-diff-binary"));
    assert!(!drawn(cx, "graph-diff-line-0"));
    // The note is the review's own.
    assert_eq!(BINARY_NOTE, "Binary file not shown");

    // A text file with nothing in it is not called binary.
    let file = bounds(cx, "graph-file-1").center();
    cx.simulate_click(file, Modifiers::none());
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            assert!(view.receive(0, diff("empty.txt", false), cx))
        })
    });
    draw(cx);
    assert!(!drawn(cx, "graph-diff-binary"));
}
