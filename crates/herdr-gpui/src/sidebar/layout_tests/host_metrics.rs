//! A Tree host's CPU and memory on a line of their own under its name, so
//! the name never gives way to them.
use super::tree_layout::{draw, tree_window};
use super::*;
use crate::{design::ROW_HEIGHT, usage::Host};
use gpui::{Bounds, FontWeight, Pixels, TextRun, VisualTestContext, black};

fn seed(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let target = view.endpoints[1].connection.target.clone();
            view.system_load.seed(Host::Local, 12., (8, 16));
            view.system_load
                .seed(Host::of(&target).unwrap(), 87., (15, 16));
            cx.notify();
        })
    });
    draw(cx);
    draw(cx);
}

fn bounds(cx: &mut VisualTestContext, selector: &str) -> Bounds<Pixels> {
    cx.debug_bounds(Box::leak(selector.to_owned().into_boxed_str()))
        .unwrap_or_else(|| panic!("{selector}"))
}

/// How wide `text` shapes in the host names' face.
fn natural(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext, text: &str) -> Pixels {
    cx.update(|window, cx| {
        let ui = view.read(cx).config.ui.clone();
        let mut font = ui.font();
        font.weight = FontWeight::SEMIBOLD;
        let run = TextRun {
            len: text.len(),
            font,
            color: black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let size = px(crate::design::TypeScale::of(&ui).name);
        window
            .text_system()
            .shape_line(text.to_owned().into(), size, &[run], None)
            .width
    })
}

fn large(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext) {
    view.update(cx, |view, _| view.config.ui.size = 18.);
}

#[gpui::test]
fn the_host_name_is_whole_beside_its_metrics(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, true);
    large(&view, cx);
    seed(&view, cx);
    let label = bounds(cx, "host-label-ssh:test");
    let whole = natural(&view, cx, "Build box");
    assert!(
        label.size.width >= whole - px(0.5),
        "{label:?} vs {whole:?}"
    );
    let line = bounds(cx, "host-load-line-ssh:test");
    assert!(line.top() >= label.bottom(), "{line:?} under {label:?}");
    assert!(
        (line.left() - label.left()).abs() < px(1.),
        "aligned with the name"
    );
    let row = bounds(cx, "host-ssh:test");
    assert!(line.bottom() <= row.bottom() && line.right() <= row.right());
}

#[gpui::test]
fn a_host_without_a_reading_keeps_one_line(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = tree_window(cx, true);
    draw(cx);
    assert_eq!(bounds(cx, "host-ssh:test").size.height, px(ROW_HEIGHT));
    assert!(cx.debug_bounds("host-load-line-ssh:test").is_none());
}

#[gpui::test]
fn host_buttons_stay_on_the_first_line(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, true);
    seed(&view, cx);
    let row = bounds(cx, "host-ssh:test");
    cx.simulate_mouse_move(row.center(), None, Modifiers::none());
    draw(cx);
    let label = bounds(cx, "host-label-ssh:test");
    for button in ["host-fold-ssh:test", "host-new-space-ssh:test"] {
        let button = bounds(cx, button);
        assert!(button.size.width > px(0.), "{button:?} shows on hover");
        assert!(
            (button.center().y - label.center().y).abs() < px(1.5),
            "{button:?} on {label:?}'s line"
        );
    }
    // Pointing at the row leaves the metrics where they are.
    assert!(bounds(cx, "host-load-line-ssh:test").size.width > px(0.));
}

#[gpui::test]
fn the_pinned_host_matches_its_row(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, true);
    seed(&view, cx);
    cx.simulate_resize(size(px(800.), px(260.)));
    draw(cx);
    draw(cx);
    let header = bounds(cx, "host-local").size.height;
    cx.update(|window, cx| {
        let scroll = view.read(cx).sidebar_scroll[0].clone();
        scroll.set_offset(point(px(0.), -(header + px(5.))));
        full_draw(window, cx).clear(cx);
    });
    let pinned = bounds(cx, "sticky-host-local");
    assert!(
        (pinned.size.height - header).abs() < px(0.5),
        "{pinned:?} vs {header:?}"
    );
    assert!(cx.debug_bounds("sticky-host-load-line-local").is_some());
}
