//! The workspace dialogs in the redesigned look: a heading with its cross, a
//! body built from the shared pieces, and a footer that names its keys and
//! leads with its action at the right.

use super::*;
use crate::design::ButtonKind;
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, point, px, size};

/// Opens `action`'s dialog for workspace `id` in a `width` × `height` window
/// with the interface face at `ui_size`.
fn open_dialog<'a>(
    id: &str,
    action: WorkspaceAction,
    (width, height, ui_size): (f32, f32, f32),
    cx: &'a mut TestAppContext,
) -> (Entity<HerdrWindow>, &'a mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
        let mut view = sidebar::layout_tests::fixture_window(window, cx);
        view.config.ui.size = ui_size;
        view
    });
    cx.simulate_resize(size(px(width), px(height)));
    let id = id.to_owned();
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.live.status = crate::state::ConnectionStatus::Connected;
            view.open_workspace_menu(&id, point(px(120.), px(100.)), window, cx);
            view.open_workspace_dialog(action, window, cx);
        });
        window.draw(cx).clear(cx);
    });
    (view, cx)
}

fn redraw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn a_workspace_dialog_has_a_header_with_a_cross(cx: &mut TestAppContext) {
    let (view, cx) = open_dialog("w3", WorkspaceAction::Rename, (800., 600., 12.), cx);
    let panel = cx.debug_bounds("menu-panel").unwrap();
    let header = cx.debug_bounds("dialog-header").unwrap();
    assert!(header.size.height >= px(52.), "{header:?}");
    let cross = cx.debug_bounds("dialog-dismiss").unwrap();
    assert!(header.contains(&cross.center()), "{cross:?} in {header:?}");
    assert!(
        cross.right() >= panel.right() - px(12.),
        "{cross:?} in {panel:?}"
    );
    cx.simulate_click(cross.center(), Modifiers::default());
    assert!(view.read_with(cx, |view, _| view.menu.page.is_none()));
    assert!(cx.update(|window, cx| view.read(cx).focus.is_focused(window)));
}

#[gpui::test]
fn the_footer_puts_the_action_last_and_hints_first(cx: &mut TestAppContext) {
    let (_, cx) = open_dialog("w3", WorkspaceAction::Rename, (800., 600., 12.), cx);
    let hint = cx.debug_bounds("dialog-footer-hint-0").unwrap();
    let cancel = cx.debug_bounds("dialog-cancel").unwrap();
    let submit = cx.debug_bounds("dialog-submit").unwrap();
    assert!(hint.right() < cancel.left(), "{hint:?} {cancel:?}");
    assert!(cancel.right() < submit.left(), "{cancel:?} {submit:?}");
    assert_eq!(cancel.size.height, px(30.));
    assert_eq!(submit.size.height, px(30.));
}

#[gpui::test]
fn the_text_field_is_thirty_tall_and_keeps_ime_inside_it(cx: &mut TestAppContext) {
    let (view, cx) = open_dialog("w3", WorkspaceAction::Rename, (800., 600., 12.), cx);
    let field = cx.debug_bounds("dialog-input").unwrap();
    assert_eq!(field.size.height, px(30.));
    let input = view.read_with(cx, |view, _| view.menu.input.as_ref().unwrap().bounds);
    assert!(
        field.contains(&input.origin) && field.contains(&input.bottom_right()),
        "{input:?} in {field:?}"
    );
}

#[test]
fn close_and_delete_submit_in_danger() {
    for action in [WorkspaceAction::Close, WorkspaceAction::DeleteWorktree] {
        assert_eq!(
            super::super::workspace::submit_kind(action),
            ButtonKind::Danger
        );
    }
    for action in [
        WorkspaceAction::Rename,
        WorkspaceAction::NewWorktree,
        WorkspaceAction::OpenWorktree,
        WorkspaceAction::NewTab,
        WorkspaceAction::NewWorkspace,
        WorkspaceAction::Note,
    ] {
        assert_eq!(
            super::super::workspace::submit_kind(action),
            ButtonKind::Primary
        );
    }
}

#[gpui::test]
fn a_long_label_truncates_and_keeps_the_cross(cx: &mut TestAppContext) {
    let (view, cx) = open_dialog("w3", WorkspaceAction::Rename, (800., 600., 12.), cx);
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.menu.target.as_mut().unwrap().label = "a-workspace-label-".repeat(5);
        })
    });
    redraw(cx);
    let cross = cx.debug_bounds("dialog-dismiss").unwrap();
    let sub = cx.debug_bounds("dialog-target").unwrap();
    assert!(sub.right() <= cross.left(), "{sub:?} {cross:?}");
    assert_eq!(cross.size.width, px(28.));
}

#[gpui::test]
fn an_error_wraps_without_widening_the_dialog(cx: &mut TestAppContext) {
    let (view, cx) = open_dialog("w3", WorkspaceAction::Rename, (800., 600., 12.), cx);
    let before = cx.debug_bounds("menu-panel").unwrap();
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.menu.error = Some("the daemon refused the rename ".repeat(10));
        })
    });
    redraw(cx);
    let after = cx.debug_bounds("menu-panel").unwrap();
    assert_eq!(after.size.width, before.size.width);
    let error = cx.debug_bounds("dialog-error").unwrap();
    assert!(error.size.height > px(40.), "{error:?}");
    assert!(
        after.contains(&error.bottom_right()),
        "{error:?} in {after:?}"
    );
}

#[gpui::test]
fn buttons_stay_inside_a_small_window(cx: &mut TestAppContext) {
    for window in [(480., 400., 12.), (960., 780., 18.)] {
        let (_, cx) = open_dialog("w3", WorkspaceAction::Rename, window, cx);
        let panel = cx.debug_bounds("menu-panel").unwrap();
        let submit = cx.debug_bounds("dialog-submit").unwrap();
        assert!(
            submit.right() <= panel.right(),
            "{window:?}: {submit:?} in {panel:?}"
        );
        let cross = cx.debug_bounds("dialog-dismiss").unwrap();
        assert!(
            panel.contains(&cross.origin) && cross.right() <= panel.right(),
            "{window:?}: {cross:?} in {panel:?}"
        );
    }
}

#[gpui::test]
fn a_new_worktree_error_stays_in_view_in_a_small_window(cx: &mut TestAppContext) {
    let (view, cx) = open_dialog("w3", WorkspaceAction::NewWorktree, (480., 400., 12.), cx);
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.menu.error = Some("The daemon refused to create the worktree. ".repeat(3));
        })
    });
    redraw(cx);
    let form = cx.debug_bounds("new-worktree-form").unwrap();
    let error = cx.debug_bounds("dialog-error").unwrap();
    let footer = cx.debug_bounds("dialog-footer").unwrap();
    // Outside the scrolling form, so no scroll is needed to read it.
    assert!(
        error.top() >= form.bottom() - px(1.),
        "{error:?} under {form:?}"
    );
    assert!(
        error.bottom() <= footer.top() + px(1.),
        "{error:?} over {footer:?}"
    );
}

#[gpui::test]
fn the_archive_script_check_is_a_note(cx: &mut TestAppContext) {
    let (view, cx) = open_dialog("w4", WorkspaceAction::DeleteWorktree, (800., 600., 12.), cx);
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.menu.deletion = Some(Deletion {
                path: Some("/endpoint/.herdr/worktrees/agent-launcher/child".into()),
                archive: ArchiveCheck::Reading,
                ..Deletion::new(None, true)
            });
        })
    });
    redraw(cx);
    assert!(cx.debug_bounds("dialog-archive-script").is_some());
    assert!(cx.debug_bounds("dialog-archive-script-icon").is_some());
}
