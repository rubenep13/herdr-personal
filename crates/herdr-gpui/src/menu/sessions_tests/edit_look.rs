//! A session's create and delete forms are dialogs in the redesigned look:
//! a heading with its cross, and the action last in the footer, red when
//! it deletes.

use super::*;
use crate::design::ButtonKind;

#[test]
fn deleting_a_session_submits_in_danger() {
    use super::super::sessions::{Edit, EditTarget, Row, edit_kind};
    let delete = Edit::Delete {
        row: Row::Local("agents".into()),
        target: EditTarget::Local,
    };
    assert_eq!(edit_kind(&delete), ButtonKind::Danger);
}

#[cfg(not(windows))]
#[gpui::test]
fn session_forms_are_dialogs(cx: &mut TestAppContext) {
    let (view, cx) =
        on_the_first_device(cx, Some(reported(&[("default", true), ("agents", false)])));
    view.update(cx, |view, cx| {
        view.endpoints[1].live.status = crate::state::ConnectionStatus::Connected;
        cx.notify();
    });
    draw(cx);
    // The device's two sessions come first, then its Add row.
    click(cx, "sessions-row-2");
    draw(cx);
    let header = bounds(cx, "session-edit-header");
    let cross = bounds(cx, "sessions-dismiss");
    assert!(header.contains(&cross.center()), "{cross:?} in {header:?}");
    let cancel = bounds(cx, "session-edit-cancel");
    let submit = bounds(cx, "session-edit-submit");
    assert!(submit.left() > cancel.right(), "{cancel:?} {submit:?}");
    assert_eq!(submit.size.height, px(30.));
    click(cx, "sessions-dismiss");
    cx.update(|_, cx| assert!(view.read(cx).menu.session_edit.is_none()));
}
