//! The inbox in a narrow tab: the preview goes under the list rather than
//! squeezing it.
use super::look::{bounds, inbox, issues};
use crate::orchestrator::view::{Look, OrchestratorView};
use gpui::{Bounds, Pixels, TestAppContext, VisualTestContext, px, size};

/// The view at `[ui]` `size`.
pub(super) fn ui_size(
    view: &mut OrchestratorView,
    size: f32,
    cx: &mut gpui::Context<OrchestratorView>,
) {
    let mut ui = view.look.ui.clone();
    ui.size = size;
    let look = Look::new(
        view.look.indicators,
        view.look.theme.clone(),
        ui.clone(),
        view.look.mono.clone(),
        false,
        &ui,
    );
    view.set_look(look, cx);
}

/// Two frames: the first measures the inbox, the second follows it.
pub(super) fn settle(cx: &mut VisualTestContext, width: f32, height: f32) {
    cx.simulate_resize(size(px(width), px(height)));
    for _ in 0..2 {
        cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    }
}

fn stacked(list: Bounds<Pixels>, preview: Bounds<Pixels>) -> bool {
    preview.top() >= list.bottom()
}

#[gpui::test]
fn a_narrow_inbox_puts_the_preview_under_the_list(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    view.update(cx, |view, cx| ui_size(view, 18., cx));
    settle(cx, 680., 560.);
    let list = bounds(cx, "orchestrator-list");
    let preview = bounds(cx, "orchestrator-preview-pane");
    assert!(stacked(list, preview), "{list:?} over {preview:?}");
    assert!((list.size.width - preview.size.width).abs() <= px(1.));
    assert_eq!(list.left(), preview.left());
    let body = preview.bottom() - list.top();
    assert!(
        (preview.size.height - body * 0.45).abs() <= px(1.),
        "{preview:?} of {body:?}"
    );
}

#[gpui::test]
fn a_wide_inbox_keeps_the_preview_beside_the_list(cx: &mut TestAppContext) {
    let (_dir, _view, cx) = inbox(cx, issues(3));
    settle(cx, 1400., 900.);
    let list = bounds(cx, "orchestrator-list");
    let preview = bounds(cx, "orchestrator-preview-pane");
    assert!(
        preview.left() >= list.right(),
        "{list:?} beside {preview:?}"
    );
    assert!(list.size.width >= px(400.));
}

#[gpui::test]
fn the_list_never_gets_under_400_beside_the_preview(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    settle(cx, 1400., 900.);
    let list = bounds(cx, "orchestrator-list");
    let preview = bounds(cx, "orchestrator-preview-pane");
    let chrome = px(1400.) - (preview.right() - list.left());
    let preview_width = view.read_with(cx, |view, _| view.preview_width);
    let room = f32::from(chrome) + 400. + preview_width;
    settle(cx, room - 1., 900.);
    let list = bounds(cx, "orchestrator-list");
    let preview = bounds(cx, "orchestrator-preview-pane");
    assert!(stacked(list, preview), "{list:?} over {preview:?}");
    settle(cx, room, 900.);
    let list = bounds(cx, "orchestrator-list");
    let preview = bounds(cx, "orchestrator-preview-pane");
    assert!(
        preview.left() >= list.right(),
        "{list:?} beside {preview:?}"
    );
    assert!(list.size.width >= px(400.) - px(0.5), "{list:?}");
}

#[gpui::test]
fn the_dragged_width_comes_back_when_there_is_room(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    view.update(cx, |view, cx| {
        view.preview_width = 333.;
        cx.notify();
    });
    settle(cx, 680., 560.);
    let list = bounds(cx, "orchestrator-list");
    let preview = bounds(cx, "orchestrator-preview-pane");
    assert!(stacked(list, preview));
    settle(cx, 1400., 900.);
    let preview = bounds(cx, "orchestrator-preview-pane");
    assert!(
        (preview.size.width - px(333.)).abs() <= px(1.),
        "{preview:?}"
    );
    view.read_with(cx, |view, _| assert_eq!(view.preview_width, 333.));
}

#[gpui::test]
fn keys_still_move_the_selection_under_the_list(cx: &mut TestAppContext) {
    let items = issues(3);
    let third = items[2].key.canonical();
    let (_dir, view, cx) = inbox(cx, items);
    settle(cx, 680., 560.);
    let down = gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("down").unwrap(),
        is_held: false,
        prefer_character_input: false,
    };
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.on_key(&down, window, cx);
            view.on_key(&down, window, cx);
        })
    });
    settle(cx, 680., 560.);
    view.read_with(cx, |view, _| {
        assert_eq!(view.selected.as_deref(), Some(third.as_str()));
        assert!(view.preview_stacked());
    });
    let list = bounds(cx, "orchestrator-list");
    let preview = bounds(cx, "orchestrator-preview-pane");
    assert!(stacked(list, preview));
}

#[gpui::test]
fn the_empty_list_reads_on_one_line(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, Vec::new());
    view.update(cx, |view, cx| {
        ui_size(view, 18., cx);
        view.query = "zzz".into();
    });
    settle(cx, 680., 560.);
    let empty = bounds(cx, "orchestrator-empty");
    let name = view.read_with(cx, |view, _| view.look.view().scale().name);
    assert!(empty.size.height <= px(name * 2.), "{empty:?}");
}
