//! The tab strip's menu takes its longest row's width, so New Browser Tab
//! reads whole beside its shortcut.
use super::*;
use gpui::{TextRun, black, px, size};

#[gpui::test]
fn the_tab_menu_fits_its_longest_row(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1200.), px(850.)));
    tunnels::remote_menu(&view, cx);
    draw(cx);
    let label = cx.debug_bounds("group-menu-NewBrowserTab-label").unwrap();
    let keys = cx.debug_bounds("group-menu-NewBrowserTab-detail").unwrap();
    let whole = cx.update(|window, cx| {
        let ui = view.read(cx).config.ui.clone();
        let text = "New Browser Tab";
        let run = TextRun {
            len: text.len(),
            font: ui.font(),
            color: black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let size = px(crate::design::TypeScale::of(&ui).name);
        window
            .text_system()
            .shape_line(text.into(), size, &[run], None)
            .width
    });
    assert!(label.size.width >= whole - px(0.5), "{label:?} vs {whole:?}");
    assert!(label.right() <= keys.left(), "{label:?} before {keys:?}");
    let panel = cx.debug_bounds("menu-panel").unwrap();
    assert!(panel.right() <= px(1200.), "{panel:?}");
}
