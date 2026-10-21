//! The new worktree dialog in the redesigned look: its search in a query
//! row, its tabs underlined in the accent, and its listings in menu rows.

use super::*;
use gpui::{TestAppContext, px, size};

fn window(cx: &mut TestAppContext) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(1000.), px(800.)));
    (view, cx)
}

#[gpui::test]
fn tabs_are_underlined_and_keep_the_height(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    open_dialog(&view, cx, true, Tab::New);
    let height = cx.debug_bounds("menu-panel").unwrap().size.height;
    for tab in [
        Tab::New,
        Tab::Branches,
        Tab::Items(Kind::Issue),
        Tab::Items(Kind::PullRequest),
    ] {
        cx.update(|window, cx| {
            view.update(cx, |view, cx| view.select_worktree_tab(tab, window, cx))
        });
        draw(cx);
        let panel = cx.debug_bounds("menu-panel").unwrap();
        assert!(
            (panel.size.height - height).abs() <= px(1.),
            "{tab:?}: {panel:?}"
        );
        let search = cx.debug_bounds("worktree-search").unwrap();
        let tabs = cx.debug_bounds("worktree-tabs").unwrap();
        assert!(
            tabs.top() >= search.bottom() - px(1.),
            "{search:?} {tabs:?}"
        );
        // Debug bounds are looked up by a static name.
        let selector = format!("worktree-tab-{}", tab.label()).leak();
        let chosen = cx.debug_bounds(selector).unwrap();
        let line = cx.debug_bounds("worktree-tab-underline").unwrap();
        assert!(
            chosen.left() <= line.left() && line.right() <= chosen.right(),
            "{tab:?}: {line:?} under {chosen:?}"
        );
    }
}

#[gpui::test]
fn listing_rows_are_menu_rows(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    open_dialog(&view, cx, true, Tab::Items(Kind::PullRequest));
    let panel = cx.debug_bounds("menu-panel").unwrap();
    let row = cx.debug_bounds("worktree-row-0").unwrap();
    assert_eq!(row.size.height, px(44.));
    // Inset like a menu's rows, so the selection's corners show.
    assert!(row.left() - panel.left() >= px(6.), "{row:?} in {panel:?}");
}
