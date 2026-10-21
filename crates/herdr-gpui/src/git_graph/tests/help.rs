//! `?` lists every shortcut the graph knows, as the design's table does.
use super::*;
use crate::git_graph::{history::History, view::SHORTCUTS};
use gpui::TestAppContext;

#[gpui::test]
fn question_mark_shows_every_shortcut(cx: &mut TestAppContext) {
    let keys: Vec<&str> = SHORTCUTS.iter().map(|(keys, _)| *keys).collect();
    assert_eq!(
        keys,
        [
            "j / \u{2193}, k / \u{2191}",
            "Enter",
            "Esc",
            "/",
            "n / N",
            "w",
            "h",
            "c",
            "r",
            "?",
        ]
    );
    let (view, cx) = shown(cx, History::new(&line(3), &[], None, None, false), 600.);
    cx.update(|window, cx| {
        let handle = view.read(cx).focus_handle.clone();
        window.focus(&handle, cx);
    });
    assert!(!drawn(cx, "graph-help"));
    cx.simulate_keystrokes("?");
    draw(cx);
    assert!(drawn(cx, "graph-help"));
    for row in 0..SHORTCUTS.len() {
        assert!(drawn(cx, &format!("graph-help-row-{row}")), "row {row}");
        // Each key is drawn as a key.
        assert!(
            drawn(cx, &format!("graph-help-row-{row}-key-0")),
            "row {row}"
        );
    }
    assert!(drawn(cx, "graph-help-header"));
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(!drawn(cx, "graph-help"));
}

#[gpui::test]
fn shift_slash_is_help_too(cx: &mut TestAppContext) {
    let (view, cx) = shown(cx, History::new(&line(3), &[], None, None, false), 600.);
    cx.update(|window, cx| {
        let handle = view.read(cx).focus_handle.clone();
        window.focus(&handle, cx);
    });
    cx.simulate_keystrokes("shift-/");
    draw(cx);
    assert!(drawn(cx, "graph-help"));
    assert!(!drawn(cx, "graph-search-input"), "it is not the search");
}
