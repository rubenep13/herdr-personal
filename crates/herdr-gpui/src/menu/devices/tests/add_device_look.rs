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

#[gpui::test]
fn add_device_fields_wear_the_dialogs_frame(cx: &mut gpui::TestAppContext) {
    let (view, cx) = open(900., cx);
    view.read_with(cx, |view, cx| {
        for field in &view.menu.device_setup.as_ref().unwrap().fields {
            assert_eq!(field.read(cx).frame(), crate::search_input::Frame::Field);
        }
    });
}

#[cfg(feature = "boat")]
#[gpui::test]
fn a_cloud_source_ends_in_the_shared_footer(cx: &mut gpui::TestAppContext) {
    let (_, cx) = open(900., cx);
    let boat = cx.debug_bounds("add-device-source-boat").unwrap();
    cx.simulate_click(boat.center(), Modifiers::default());
    draw(cx);
    let footer = cx.debug_bounds("cloud-setup-footer").unwrap();
    let settings = cx.debug_bounds("cloud-setup-settings").unwrap();
    assert!(
        footer.contains(&settings.center()),
        "{settings:?} in {footer:?}"
    );
    assert_eq!(settings.size.height, px(30.));
}

#[cfg(feature = "coder")]
#[gpui::test]
fn coder_signs_out_and_errs_in_the_shared_pieces(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(900.), px(760.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_coder_fixture(window, cx);
            view.menu.coder.as_mut().unwrap().error = Some("Coder refused the token".into());
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("coder-setup-error").is_some());
    let sign_out = cx.debug_bounds("coder-sign-out").unwrap();
    assert_eq!(sign_out.size.height, px(26.));
}
