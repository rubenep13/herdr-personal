//! Copy mode's badge and search prompt stay inside a short pane, at its
//! bottom right, at a large interface size.
use super::*;
// `super::*` brings in gpui's `test`, which `#[gpui::test]` expands to.
use core::prelude::v1::test;
use gpui::{Bounds, Pixels, px};

/// Copy mode over a pane three rows tall at the top, at `[ui]` 18, and the
/// pane's rectangle on screen.
fn short_pane(
    cx: &mut TestAppContext,
) -> (Entity<HerdrWindow>, &mut VisualTestContext, Bounds<Pixels>) {
    let peer = MockPeer::advertising(&[
        "pane.copy_motion",
        "pane.copy_search",
        "pane.selection.read",
        "pane.scroll",
    ]);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        peer.prepare(&mut view, cx);
        view.live.supports_copy_motion = true;
        view.live.supports_copy_search = true;
        view.config.ui.size = 18.;
        let surface = Arc::make_mut(view.live.surface.as_mut().unwrap());
        surface.panes[0].rect.y = 0;
        surface.panes[0].rect.height = 3;
        view
    });
    let redraw = |cx: &mut VisualTestContext| {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
    };
    redraw(cx);
    cx.update(|window, cx| view.update(cx, |view, cx| view.command(Command::CopyMode, window, cx)));
    redraw(cx);
    let pane = view.read_with(cx, |view, _| {
        let rect = view.live.surface.as_ref().unwrap().panes[0].rect;
        let height = view.config.terminal.line_height();
        Bounds::from_corners(
            point(
                view.bounds.left() + px(f32::from(rect.x) * view.cell_width),
                view.bounds.top() + px(f32::from(rect.y) * height),
            ),
            point(
                view.bounds.left() + px(f32::from(rect.x + rect.width) * view.cell_width),
                view.bounds.top() + px(f32::from(rect.y + rect.height) * height),
            ),
        )
    });
    (view, cx, pane)
}

fn inside(part: Bounds<Pixels>, pane: Bounds<Pixels>) {
    assert!(
        part.left() >= pane.left() && part.right() <= pane.right(),
        "{part:?} in {pane:?}"
    );
    assert!(
        part.top() >= pane.top() && part.bottom() <= pane.bottom(),
        "{part:?} in {pane:?}"
    );
}

#[gpui::test]
fn the_copy_badge_stays_inside_its_pane(cx: &mut TestAppContext) {
    let (_view, cx, pane) = short_pane(cx);
    inside(cx.debug_bounds("copy-mode").unwrap(), pane);
}

#[gpui::test]
fn the_copy_search_stays_inside_its_pane(cx: &mut TestAppContext) {
    let (_view, cx, pane) = short_pane(cx);
    cx.simulate_keystrokes("/");
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    inside(cx.debug_bounds("copy-search").unwrap(), pane);
}
