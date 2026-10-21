//! Open worktree in the redesigned look: a query row for its search and
//! menu rows for its checkouts.

use super::*;

#[gpui::test]
fn open_worktree_searches_from_a_query_row(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(1000.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.live.status = ConnectionStatus::Connected;
            view.open_workspace_menu("w3", Default::default(), window, cx);
            view.open_workspace_dialog(WorkspaceAction::OpenWorktree, window, cx);
            pending(&mut view.menu, cx);
            view.menu
                .apply_worktree_list_response("list", Ok(listing()));
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    let search = cx.debug_bounds("open-worktree-search").unwrap();
    let icon = cx.debug_bounds("open-worktree-search-icon").unwrap();
    assert!(search.contains(&icon.center()), "{icon:?} in {search:?}");
    let row = cx.debug_bounds("open-worktree-row-0").unwrap();
    assert_eq!(row.size.height, px(44.));
    let cross = cx.debug_bounds("open-worktree-escape").unwrap();
    cx.simulate_click(cross.center(), Modifiers::default());
    assert!(view.read_with(cx, |view, _| view.menu.page.is_none()));
}

#[gpui::test]
fn open_worktree_status_shows_two_lines_of_an_error(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(1000.), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.live.status = ConnectionStatus::Connected;
            view.open_workspace_menu("w3", Default::default(), window, cx);
            view.open_workspace_dialog(WorkspaceAction::OpenWorktree, window, cx);
            pending(&mut view.menu, cx);
            view.menu
                .apply_worktree_list_response("list", Ok(listing()));
            view.menu.error = Some("Endpoint error. ".repeat(20));
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    let line = view.read_with(cx, |view, _| view.config.ui.line_height());
    let status = cx.debug_bounds("open-worktree-status").unwrap();
    let error = cx.debug_bounds("dialog-error").unwrap();
    // The error box's own padding, then at least two of its lines.
    assert!(
        status.bottom() - error.top() >= px(8. + 2. * line),
        "{error:?} in {status:?}"
    );
}
