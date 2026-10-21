//! The browser tab in the redesign's pieces: its toolbar as a view bar and a
//! blank tab as an empty state. A blank tab needs no native page, so this
//! runs on every platform.
use super::*;
use crate::browser::groups_view::{address_host, address_parts};
use gpui::{px, size};

/// Opens a blank tab in the window's group, as the new tab menu does.
fn blank(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            let (scope, workspace) = view.browser_key().unwrap();
            let id = Store::update(cx, |store| store.open(scope, &workspace, None, None)).unwrap();
            view.show_browser_tab(id, window, cx);
        })
    });
    draw(cx);
}

#[gpui::test]
fn the_toolbar_is_a_view_bar(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx);
    blank(&view, cx);
    let toolbar = cx.debug_bounds("browser-toolbar").unwrap();
    assert_eq!(toolbar.size.height, px(40.));
    let back = cx.debug_bounds("browser-back").unwrap();
    assert_eq!(back.size.height, px(28.));
    assert_eq!(back.size.width, px(28.));
    let field = cx.debug_bounds("browser-address-field").unwrap();
    assert_eq!(field.size.height, px(28.));
    assert!(field.contains(&cx.debug_bounds("browser-address").unwrap().center()));
}

#[gpui::test]
fn a_blank_tab_is_an_empty_state_with_its_ports(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1400.), px(900.)));
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.listening_ports.seed(
                crate::usage::Host::Local,
                crate::listening_ports::parse("L 1 *:3000 node\nL 1 127.0.0.1:5173 vite\nE 1 w0\n")
                    .unwrap(),
            );
            cx.notify();
        })
    });
    blank(&view, cx);
    let area = cx.debug_bounds("browser-placeholder-area").unwrap();
    let prompt = cx.debug_bounds("browser-placeholder").unwrap();
    assert!(
        (prompt.center().x - area.center().x).abs() <= px(2.),
        "{prompt:?} centred in {area:?}"
    );
    assert!(cx.debug_bounds("browser-placeholder-tile").is_some());
    for selector in ["blank-port-3000", "blank-port-5173"] {
        let port = cx.debug_bounds(selector).unwrap();
        assert_eq!(port.size.height, px(26.), "{selector}");
        assert!(port.top() > prompt.bottom());
    }
}

#[test]
fn address_parts_split_the_host() {
    for (address, parts) in [
        ("https://github.com/a/b", ("github.com", "/a/b")),
        ("http://localhost:5173", ("localhost:5173", "")),
        ("http://localhost:5173/?q=1", ("localhost:5173", "/?q=1")),
        ("example.com/docs", ("example.com", "/docs")),
        ("", ("", "")),
    ] {
        assert_eq!(address_parts(address), parts, "{address}");
    }
    assert_eq!(address_host("https://github.com/a/b"), 8..18);
    assert_eq!(address_host("localhost:5173"), 0..14);
}
