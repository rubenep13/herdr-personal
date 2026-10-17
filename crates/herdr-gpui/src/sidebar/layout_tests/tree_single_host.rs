//! A tree with one host draws no host header, so what the header would carry
//! stays where the other layouts keep it.
use super::tree_layout::{draw, tree_window};
use crate::usage::Host;

#[gpui::test]
fn single_host_tree_keeps_new_and_menu_in_the_footer(cx: &mut gpui::TestAppContext) {
    let (_, cx) = tree_window(cx, false);
    assert!(cx.debug_bounds("host-local").is_none());
    assert!(cx.debug_bounds("new-workspace").is_some());
    assert!(cx.debug_bounds("sidebar-menu").is_some());
}

#[gpui::test]
fn host_headers_take_over_new_and_menu(cx: &mut gpui::TestAppContext) {
    let (_, cx) = tree_window(cx, true);
    assert!(cx.debug_bounds("host-new-space-local").is_some());
    assert!(cx.debug_bounds("new-workspace").is_none());
    assert!(cx.debug_bounds("sidebar-menu").is_none());
}

#[gpui::test]
fn single_host_tree_keeps_load_in_the_status_bar(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.system_load.seed(Host::Local, 12., (8, 16));
            cx.notify();
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("host-load-local").is_none());
    assert!(cx.debug_bounds("system-load").is_some());
}
