//! Keyboard Shortcuts in the redesigned menus' look: sentence-case groups,
//! rows with their keys on the right, and the cross to close.
use super::*;
use gpui::{TestAppContext, VisualTestContext, px, size};

fn open(cx: &mut TestAppContext) -> &mut VisualTestContext {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(1000.), px(800.)));
    view.update_in(cx, |view, window, cx| view.open_keybinds(window, cx));
    cx.run_until_parked();
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    cx
}

#[test]
fn keybind_groups_use_sentence_case() {
    for group in settings::KEYBIND_GROUPS {
        assert_ne!(*group, group.to_uppercase(), "{group}");
    }
}

#[gpui::test]
fn shortcut_rows_put_keys_on_the_right(cx: &mut TestAppContext) {
    let cx = open(cx);
    let keys = cx.debug_bounds("keys-New Workspace").unwrap();
    let description = cx.debug_bounds("description-New Workspace").unwrap();
    let body = cx.debug_bounds("keybinds-body").unwrap();
    assert!(keys.left() > description.left(), "{keys:?} {description:?}");
    // The row's padding, the pencil's 26 px and the gap before it.
    assert!(
        body.right() - keys.right() < px(60.),
        "the keys end by the pencil: {keys:?} in {body:?}"
    );
}

#[gpui::test]
fn the_edit_button_stays_in_the_tree_while_hidden(cx: &mut TestAppContext) {
    let cx = open(cx);
    let edit = cx.debug_bounds("edit-shortcut-reload_config").unwrap();
    assert_eq!((edit.size.width, edit.size.height), (px(26.), px(26.)));
}

#[gpui::test]
fn keybinds_close_is_the_cross(cx: &mut TestAppContext) {
    let cx = open(cx);
    let close = cx.debug_bounds("keybinds-close").unwrap();
    assert_eq!((close.size.width, close.size.height), (px(28.), px(28.)));
}
