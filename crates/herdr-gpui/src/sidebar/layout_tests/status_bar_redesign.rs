use super::tree_layout::{draw, tree_window};
use super::*;
use crate::{
    config::status_bar::{Button, StatusBar},
    state::ConnectionStatus,
    usage::Host,
};
use herdr_client::protocol::ClientShellTabStatusSegment;

fn plain_window(
    cx: &mut gpui::TestAppContext,
) -> (Entity<HerdrWindow>, &mut gpui::VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.status = ConnectionStatus::Connected;
        view
    });
    draw(cx);
    (view, cx)
}

/// Herdr's `tab_bar_right` as Rubén's config fills it: the machine and the
/// time.
fn herdr_segments(view: &Entity<HerdrWindow>, cx: &mut gpui::VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
            snapshot.tab_bar_right = ["omarchy", "00:42"]
                .map(|text| ClientShellTabStatusSegment {
                    text: text.into(),
                    accent: false,
                })
                .into();
            cx.notify();
        })
    });
    draw(cx);
}

fn within(inner: Bounds<Pixels>, outer: Bounds<Pixels>) -> bool {
    inner.left() >= outer.left()
        && inner.right() <= outer.right()
        && inner.top() >= outer.top()
        && inner.bottom() <= outer.bottom()
}

#[gpui::test]
fn herdr_segments_move_to_the_status_bar(cx: &mut gpui::TestAppContext) {
    let (view, cx) = plain_window(cx);
    herdr_segments(&view, cx);
    let bar = cx.debug_bounds("connection-status").unwrap();
    let segments = cx.debug_bounds("status-segments").unwrap();
    assert!(segments.size.width > px(0.));
    assert!(within(segments, bar), "{segments:?} in {bar:?}");
    // The main header no longer carries them.
    assert!(cx.debug_bounds("titlebar-status").is_none());
}

#[gpui::test]
fn hidden_status_bar_keeps_herdr_segments_in_the_header(cx: &mut gpui::TestAppContext) {
    let (view, cx) = plain_window(cx);
    cx.update(|_, cx| view.update(cx, |view, _| view.status_bar_visible = false));
    herdr_segments(&view, cx);
    assert!(cx.debug_bounds("status-segments").is_none());
    let header = cx.debug_bounds("titlebar").unwrap();
    let status = cx.debug_bounds("titlebar-status").unwrap();
    assert!(within(status, header), "{status:?} in {header:?}");
}

#[gpui::test]
fn status_bar_is_thirty_pixels_tall(cx: &mut gpui::TestAppContext) {
    let (_, cx) = plain_window(cx);
    let bar = cx.debug_bounds("connection-status").unwrap();
    assert_eq!(bar.size.height, px(crate::design::STATUS_BAR_HEIGHT));
}

#[gpui::test]
fn tree_layout_leaves_system_load_to_host_rows(cx: &mut gpui::TestAppContext) {
    let seed = |view: &Entity<HerdrWindow>, cx: &mut gpui::VisualTestContext| {
        cx.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.system_load.seed(Host::Local, 12., (8, 16));
                cx.notify();
            })
        });
        draw(cx);
    };
    let (view, tree) = tree_window(cx, true);
    seed(&view, tree);
    assert!(tree.debug_bounds("host-load-local").is_some());
    assert!(tree.debug_bounds("system-load").is_none());
}

#[gpui::test]
fn other_layouts_keep_system_load_in_the_status_bar(cx: &mut gpui::TestAppContext) {
    let (view, cx) = plain_window(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.system_load.seed(Host::Local, 12., (8, 16));
            cx.notify();
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("system-load").is_some());
}

#[gpui::test]
fn buttons_default_to_icons(cx: &mut gpui::TestAppContext) {
    let defaults = StatusBar::default();
    assert_eq!(
        [defaults.theme, defaults.shortcuts, defaults.report_issue],
        [Button::Icon; 3]
    );
    let (view, cx) = plain_window(cx);
    assert!(cx.debug_bounds("status-theme").is_some());
    assert!(cx.debug_bounds("status-theme-label").is_none());
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.config.status_bar.theme = Button::Label;
            cx.notify();
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("status-theme-label").is_some());
}
