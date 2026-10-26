//! A commit's detail beside the graph: its files with how they changed, a
//! chosen file's diff (cut at 4000 lines), parents that jump to their row,
//! and a panel that resizes within bounds and remembers its width.

use super::*;
use crate::git_graph::{
    detail::{CommitDetail, FileChange, file_counts, file_label},
    history::History,
    prefs::GraphPrefsStore,
    view::{Connection, GitGraphView, diff_tint},
    worker::{Pending, Reply, Request},
};
use crate::review::{Kind, Lines};
use crate::teleport::Host;
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size};
use herdr_client::ConnectTarget;
use std::sync::Arc;

fn change(path: &str, kind: char) -> FileChange {
    FileChange {
        path: path.into(),
        old_path: None,
        kind,
        additions: Some(3),
        deletions: Some(1),
        binary: false,
        git_path: path.into(),
        git_old_path: None,
    }
}

fn detail_of(commit: Commit, files: Vec<FileChange>) -> Arc<CommitDetail> {
    Arc::new(CommitDetail {
        commit,
        body: "first line\nsecond line".into(),
        files,
    })
}

pub(super) fn focus(view: &gpui::Entity<GitGraphView>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let handle = view.read(cx).focus_handle.clone();
        window.focus(&handle, cx);
    });
}

#[gpui::test]
fn enter_opens_the_detail_and_escape_closes_it(cx: &mut TestAppContext) {
    let (view, cx) = shown(cx, History::new(&line(5), &[], None, None, false), 600.);
    focus(&view, cx);
    cx.simulate_keystrokes("j enter");
    draw(cx);
    assert_eq!(bounds(cx, "graph-detail").size.width, px(520.));
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(!drawn(cx, "graph-detail"));
    cx.simulate_keystrokes("enter");
    draw(cx);
    let close = bounds(cx, "graph-detail-close").center();
    cx.simulate_click(close, Modifiers::none());
    draw(cx);
    assert!(!drawn(cx, "graph-detail"));
    // A click on a row opens it too.
    let row = bounds(cx, "graph-row-2").center();
    cx.simulate_click(row, Modifiers::none());
    draw(cx);
    assert!(drawn(cx, "graph-detail"));
    assert_eq!(view.read_with(cx, |view, _| view.selected_row()), Some(2));
}

#[cfg(unix)]
#[test]
fn detail_lists_files_with_kind_and_counts() {
    let fixture = Fixture::new();
    fixture.write("a.txt", "one\ntwo\nthree\n");
    fixture.write("b.txt", "gone\n");
    fixture.write(
        "c.txt",
        "a file long enough\nthat moving it\nstill reads\nas a rename\n",
    );
    std::fs::write(fixture.path().join("bin.dat"), [0u8, 1, 2, 3, 0, 255]).unwrap();
    let base = fixture.commit("base");
    fixture.write("a.txt", "one\nTWO\nthree\nfour\n");
    std::fs::remove_file(fixture.path().join("b.txt")).unwrap();
    std::fs::rename(fixture.path().join("c.txt"), fixture.path().join("d.txt")).unwrap();
    fixture.write("e.txt", "new\n");
    std::fs::write(fixture.path().join("bin.dat"), [0u8, 9, 9, 9, 0, 255]).unwrap();
    let head = fixture.commit("change everything\n\nWhy it changed.\nAnd how.");

    let repo = fixture.open();
    let cancelled = AtomicBool::new(false);
    let detail = repo.commit_detail(&head, &cancelled).unwrap();
    assert_eq!(detail.commit.oid, head);
    assert_eq!(detail.commit.subject, "change everything");
    assert_eq!(detail.body, "Why it changed.\nAnd how.");
    let found = |path: &str| {
        detail
            .files
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| panic!("{path} in {:?}", detail.files))
    };
    assert_eq!(found("a.txt").kind, 'M');
    assert_eq!(
        (found("a.txt").additions, found("a.txt").deletions),
        (Some(2), Some(1))
    );
    assert_eq!(found("b.txt").kind, 'D');
    assert_eq!(found("e.txt").kind, 'A');
    let renamed = found("d.txt");
    assert_eq!(renamed.kind, 'R');
    assert_eq!(renamed.old_path.as_deref(), Some("c.txt"));
    assert_eq!(file_label(renamed), "c.txt \u{2192} d.txt");
    let binary = found("bin.dat");
    assert!(binary.binary);
    assert_eq!(binary.kind, 'B');
    assert_eq!(file_counts(binary), "bin");
    assert_eq!(file_counts(found("a.txt")), "+2 \u{2212}1");

    // A root commit is compared with the empty tree.
    let root = repo.commit_detail(&base, &cancelled).unwrap();
    assert!(
        root.files.iter().all(|file| file.kind == 'A'),
        "{:?}",
        root.files
    );
    assert_eq!(root.files.len(), 4);
}

#[cfg(unix)]
#[test]
fn choosing_a_file_shows_its_diff() {
    let fixture = Fixture::new();
    fixture.write("a.rs", "fn one() {}\nfn two() {}\n");
    fixture.commit("base");
    fixture.write("a.rs", "fn one() {}\nfn three() {}\n");
    let head = fixture.commit("edit");
    let cancelled = AtomicBool::new(false);
    let diff = fixture
        .open()
        .file_diff(&head, "a.rs", None, &cancelled)
        .unwrap();
    assert!(!diff.truncated);
    let kinds: Vec<Kind> = diff.lines.iter().map(|line| line.kind).collect();
    assert!(
        kinds.contains(&Kind::Added) && kinds.contains(&Kind::Removed),
        "{kinds:?}"
    );
    let added = diff
        .lines
        .iter()
        .position(|line| line.kind == Kind::Added)
        .unwrap();
    assert_eq!(diff.lines.text(added), "fn three() {}");
    assert_eq!(
        diff.spans.len(),
        diff.lines.len(),
        "every line has its colours"
    );

    let theme = crate::config::Theme::builtin("Catppuccin Latte").unwrap();
    assert_eq!(
        diff_tint(&theme, Kind::Added),
        Some(theme.ink(theme.palette[2]))
    );
    assert_eq!(
        diff_tint(&theme, Kind::Removed),
        Some(theme.ink(theme.palette[1]))
    );
    assert_eq!(diff_tint(&theme, Kind::Context), None);
}

#[cfg(unix)]
#[test]
fn diffs_stop_at_four_thousand_lines() {
    let fixture = Fixture::new();
    fixture.write("big.txt", "");
    fixture.commit("empty");
    let big: String = (0..10_000).map(|line| format!("line {line}\n")).collect();
    fixture.write("big.txt", &big);
    let head = fixture.commit("big");
    let cancelled = AtomicBool::new(false);
    let diff = fixture
        .open()
        .file_diff(&head, "big.txt", None, &cancelled)
        .unwrap();
    assert!(diff.truncated);
    assert_eq!(diff.lines.len(), 4000);
}

#[gpui::test]
fn the_truncated_notice_shows(cx: &mut TestAppContext) {
    let commits = line(3);
    let (view, cx) = shown(cx, History::new(&commits, &[], None, None, false), 600.);
    focus(&view, cx);
    cx.simulate_keystrokes("j enter");
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let detail = detail_of(commits[0].clone(), vec![change("big.txt", 'M')]);
            assert!(view.receive(0, Reply::Detail(detail), cx));
        })
    });
    draw(cx);
    let file = bounds(cx, "graph-file-0").center();
    cx.simulate_click(file, Modifiers::none());
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let mut lines = Lines::default();
            lines.push(Kind::Hunk, None, None, "@@ -1 +1 @@");
            lines.push(Kind::Added, None, Some(1), "added");
            let reply = Reply::Diff {
                oid: "c0".into(),
                path: "big.txt".into(),
                lines: Arc::new(lines.finish()),
                spans: Arc::new(vec![Vec::new(); 2]),
                truncated: true,
                binary: false,
            };
            assert!(view.receive(0, reply, cx));
        })
    });
    draw(cx);
    assert!(drawn(cx, "graph-diff-line-1"));
    assert!(drawn(cx, "graph-diff-truncated"));
}

#[gpui::test]
fn parents_jump_to_their_commit(cx: &mut TestAppContext) {
    let checkout = crate::git_graph::GraphCheckout::new("/nonexistent/graph-test".into()).unwrap();
    let local = Host::new(&ConnectTarget::Local).unwrap();
    let (view, cx) = cx.add_window_view(|_, cx| {
        GitGraphView::new(Ok(local), checkout, look(), Connection::Online, cx)
    });
    cx.simulate_resize(size(px(1000.), px(600.)));
    let commits = line(30);
    let first = view.read_with(cx, |view, _| view.generation());
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let history = History::new(&commits[..10], &[], None, None, true);
            assert!(view.receive(first, Reply::History(Arc::new(history)), cx));
            view.open_detail(0, cx);
            let detail = detail_of(commits[0].clone(), Vec::new());
            let asked = view.last_detail();
            assert!(view.receive(asked, Reply::Detail(detail), cx));
        })
    });
    draw(cx);
    // A loaded parent: its row is selected.
    let parent = bounds(cx, "graph-parent-0").center();
    cx.simulate_click(parent, Modifiers::none());
    assert_eq!(view.read_with(cx, |view, _| view.selected_row()), Some(1));

    // One not loaded yet: pages load until it is found.
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.jump_to("c25", cx);
            let (more, request) = view.last_request();
            assert_eq!(request, Request::More);
            let history = History::new(&commits[..20], &[], None, None, true);
            assert!(view.receive(more, Reply::History(Arc::new(history)), cx));
            let (again, request) = view.last_request();
            assert_eq!(
                (again, request),
                (more + 1, Request::More),
                "still not found"
            );
            let history = History::new(&commits, &[], None, None, false);
            assert!(view.receive(again, Reply::History(Arc::new(history)), cx));
            assert_eq!(view.selected_row(), Some(25));
        })
    });

    // At most ten pages are read looking for one.
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let history = History::new(&commits, &[], None, None, true);
            assert!(view.receive(view.generation(), Reply::History(Arc::new(history)), cx));
            view.jump_to("missing", cx);
            let (first, request) = view.last_request();
            assert_eq!(request, Request::More);
            for _ in 0..12 {
                let (asked, _) = view.last_request();
                let history = History::new(&commits, &[], None, None, true);
                view.receive(asked, Reply::History(Arc::new(history)), cx);
            }
            // Nothing else is asked meanwhile, so pages run in sequence.
            assert_eq!(view.generation() - first + 1, 10);
        })
    });
}

#[gpui::test]
fn panel_resizes_within_bounds_and_is_remembered(cx: &mut TestAppContext) {
    let (view, cx) = shown(cx, History::new(&line(5), &[], None, None, false), 600.);
    focus(&view, cx);
    cx.simulate_keystrokes("j enter");
    // Wide enough that the share, not the graph's room, bounds the panel.
    cx.simulate_resize(size(px(2000.), px(600.)));
    draw(cx);
    draw(cx);
    let drag = |cx: &mut VisualTestContext, to: f32| {
        let edge = bounds(cx, "graph-detail-resize").center();
        cx.simulate_mouse_down(edge, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(
            point(edge.x - px(20.), edge.y),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_move(
            point(px(to), edge.y),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(to), edge.y),
            MouseButton::Left,
            Modifiers::default(),
        );
        draw(cx);
        bounds(cx, "graph-detail").size.width
    };
    assert_eq!(drag(cx, 10.), px(1500.), "at most 75 % of the tab");
    assert_eq!(drag(cx, 1990.), px(320.), "at least 320");
    let width = drag(cx, 1500.);
    assert_eq!(width, px(500.));
    assert_eq!(
        cx.update(|_, cx| GraphPrefsStore::get(cx).panel_width),
        500.
    );
}

#[test]
fn the_mailbox_keeps_the_newest_request_of_each_kind() {
    let mut pending = Pending::default();
    pending.put(1, Request::More);
    pending.put(2, Request::Detail("a".into()));
    pending.put(3, Request::Load);
    pending.put(4, Request::More);
    pending.put(5, Request::Detail("b".into()));
    pending.put(
        6,
        Request::Diff {
            oid: "b".into(),
            path: "x".into(),
            old_path: None,
        },
    );
    // A reload stands for a page asked for after it, under the newer number.
    assert_eq!(
        pending.take(),
        [
            (4, Request::Load),
            (5, Request::Detail("b".into())),
            (
                6,
                Request::Diff {
                    oid: "b".into(),
                    path: "x".into(),
                    old_path: None,
                }
            ),
        ]
    );
    assert!(pending.take().is_empty());
}
