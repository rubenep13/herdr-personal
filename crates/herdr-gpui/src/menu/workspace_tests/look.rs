use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext, point, px, size};

fn open<'a>(
    id: &str,
    cx: &'a mut TestAppContext,
) -> (Entity<HerdrWindow>, &'a mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
        sidebar::layout_tests::fixture_window(window, cx)
    });
    cx.simulate_resize(size(px(1000.), px(800.)));
    let id = id.to_owned();
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.live.status = crate::state::ConnectionStatus::Connected;
            view.open_workspace_menu(&id, point(px(300.), px(100.)), window, cx);
        });
        window.draw(cx).clear(cx);
    });
    (view, cx)
}

#[gpui::test]
fn the_header_names_the_folder_and_the_branch(cx: &mut TestAppContext) {
    let (_, cx) = open("w3", cx);
    assert!(cx.debug_bounds("workspace-menu-header").is_some());
    assert!(cx.debug_bounds("workspace-menu-name").is_some());
    assert!(cx.debug_bounds("workspace-menu-branch").is_some());
    let icon = cx.debug_bounds("workspace-menu-header-icon").unwrap();
    assert_eq!(icon.size, size(px(16.), px(16.)));
    let branch = cx.debug_bounds("workspace-menu-header-sub-icon").unwrap();
    assert_eq!(branch.size, size(px(11.), px(11.)));
}

#[gpui::test]
fn tiles_carry_an_eighteen_px_icon(cx: &mut TestAppContext) {
    let (_, cx) = open("w3", cx);
    let icon = cx.debug_bounds("workspace-menu-icon-Rename").unwrap();
    assert_eq!(icon.size, size(px(18.), px(18.)));
}

#[gpui::test]
fn rows_and_the_danger_row_are_menu_rows(cx: &mut TestAppContext) {
    let (_, cx) = open("w3", cx);
    let row = cx.debug_bounds("workspace-menu-Open worktree...").unwrap();
    assert_eq!(row.size.height, px(32.));
    let (_, cx) = open("w4", cx);
    let danger = cx
        .debug_bounds("workspace-menu-Delete worktree checkout")
        .unwrap();
    assert_eq!(danger.size.height, px(32.));
}

/// Walking down with the arrows reaches the destructive row, which then
/// fills with the danger wash; at rest it has no fill.
#[gpui::test]
fn arrowing_onto_the_danger_row_fills_it_red(cx: &mut TestAppContext) {
    let (view, cx) = open("w4", cx);
    let delete = WorkspaceMenuAction::Dialog(WorkspaceAction::DeleteWorktree);
    let count = view.read_with(cx, |view, _| view.workspace_items().len());
    for _ in 0..count {
        if view.read_with(cx, |view, _| view.menu.workspace_selected) == Some(delete) {
            break;
        }
        cx.simulate_keystrokes("down");
    }
    assert_eq!(
        view.read_with(cx, |view, _| view.menu.workspace_selected),
        Some(delete)
    );
    let (look, red) = view.read_with(cx, |view, _| {
        (view.menu_look(), crate::menu::danger(&view.theme))
    });
    let state = crate::design::ItemState {
        selected: true,
        danger: true,
        ..Default::default()
    };
    assert_eq!(look.fill(state), Some(gpui::Rgba { a: 0.14, ..red }));
    let resting = crate::design::ItemState {
        danger: true,
        ..Default::default()
    };
    assert_eq!(look.fill(resting), None);
}

#[gpui::test]
fn the_pull_request_sits_under_a_section_heading(cx: &mut TestAppContext) {
    let (view, cx) = open("w3", cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.menu.github = crate::github::Auth::connected_fixture();
            view.menu.pr.clear();
            view.menu.pr.value = Some(crate::pull_request::fixture().unwrap());
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    let heading = cx.debug_bounds("workspace-menu-section-pr").unwrap();
    let card = cx.debug_bounds("workspace-pr-card").unwrap();
    assert!(heading.bottom() <= card.top());
}
