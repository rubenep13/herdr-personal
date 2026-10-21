//! Add Device in the redesigned look: a heading with its cross, a rail of
//! Settings' navigation rows, and one size whichever source is open.

use super::*;
use gpui::{Modifiers, VisualTestContext, px, size};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

fn open(
    width: f32,
    cx: &mut gpui::TestAppContext,
) -> (gpui::Entity<HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(width), px(760.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            #[cfg(feature = "coder")]
            {
                view.config.coder.url = None;
            }
            view.open_add_device(window, cx);
        });
    });
    draw(cx);
    (view, cx)
}

#[gpui::test]
fn add_device_keeps_its_size_across_sources(cx: &mut gpui::TestAppContext) {
    let (_, cx) = open(900., cx);
    let before = cx.debug_bounds("menu-panel").unwrap();
    let host = cx.debug_bounds("add-device-source-host").unwrap();
    assert_eq!(host.size.height, px(32.));
    let coder = cx.debug_bounds("add-device-source-coder").unwrap();
    cx.simulate_click(coder.center(), Modifiers::default());
    draw(cx);
    let after = cx.debug_bounds("menu-panel").unwrap();
    assert!(
        (after.size.height - before.size.height).abs() <= px(1.),
        "{before:?} {after:?}"
    );
    assert!(
        (after.size.width - before.size.width).abs() <= px(1.),
        "{before:?} {after:?}"
    );
}

#[gpui::test]
fn add_device_has_a_cross_and_a_compact_rail(cx: &mut gpui::TestAppContext) {
    let (view, cx) = open(900., cx);
    let header = cx.debug_bounds("device-setup-header").unwrap();
    let close = cx.debug_bounds("device-setup-close").unwrap();
    assert!(header.contains(&close.center()), "{close:?} in {header:?}");
    assert_eq!(close.size.width, px(28.));
    cx.simulate_click(close.center(), Modifiers::default());
    assert!(view.read_with(cx, |view, _| view.menu.page.is_none()));
    let (_, cx) = open(420., cx);
    let rail = cx.debug_bounds("add-device-rail").unwrap();
    assert!(rail.size.width <= px(52.), "{rail:?}");
}
