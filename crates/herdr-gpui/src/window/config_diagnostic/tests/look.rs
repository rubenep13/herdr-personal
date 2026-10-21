//! The window's notices in the redesigned look: a tile in the notice's
//! colour leads each card, and the flash keeps its tone's dot.

use super::*;

#[gpui::test]
fn notices_lead_with_a_tile(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(1000.), px(600.)));
    view.update(cx, |view, cx| {
        view.gui_config_diagnostic.sync(Some(
            "config-gpui.local.toml: ignoring unknown keys a, b, c",
        ));
        cx.notify();
    });
    draw(cx);
    let card = cx.debug_bounds("gui-config-diagnostic").unwrap();
    let tile = cx.debug_bounds("gui-config-diagnostic-tile").unwrap();
    assert_eq!(tile.size, size(px(28.), px(28.)));
    assert!(card.contains(&tile.origin), "{tile:?} in {card:?}");
    let line = cx.debug_bounds("gui-config-diagnostic-line-0").unwrap();
    assert!(tile.right() < line.left(), "{tile:?} before {line:?}");
}

#[gpui::test]
fn the_flash_keeps_its_tone_dot(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(1000.), px(600.)));
    for flash in [
        crate::window::Flash::success("Copied to clipboard"),
        crate::window::Flash::warning("No pane in that direction"),
    ] {
        view.update(cx, |view, cx| view.show_flash(flash, cx));
        draw(cx);
        assert!(cx.debug_bounds("flash").is_some());
        let dot = cx.debug_bounds("flash-dot").unwrap();
        assert_eq!(dot.size, size(px(7.), px(7.)));
    }
}
