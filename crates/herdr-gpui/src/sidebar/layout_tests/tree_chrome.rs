use super::tree_layout::{draw, tree_window};
use super::*;
use crate::config::LayoutMode;

#[gpui::test]
fn search_button_opens_and_escape_closes_the_field(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    let header = cx.debug_bounds("sidebar-titlebar").unwrap();
    let button = cx.debug_bounds("sidebar-search-button").unwrap();
    assert!(header.contains(&button.center()));
    assert!(cx.debug_bounds("tree-search").is_none());
    cx.simulate_click(button.center(), Modifiers::none());
    draw(cx);
    let field = cx.debug_bounds("tree-search").unwrap();
    assert!(header.contains(&field.center()), "{field:?} {header:?}");
    assert!(cx.debug_bounds("sidebar-search-button").is_none());
    cx.simulate_input("review");
    draw(cx);
    assert!(cx.debug_bounds("row-agent-p0").is_some());
    assert!(cx.debug_bounds("row-agent-p1").is_none());
    // Typed text keeps the field open after the keyboard goes elsewhere.
    cx.update(|window, cx| {
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
    });
    draw(cx);
    assert!(cx.debug_bounds("tree-search").is_some());
    let field = cx.debug_bounds("tree-search").unwrap();
    cx.simulate_click(field.center(), Modifiers::none());
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.debug_bounds("tree-search").is_none());
    assert!(cx.debug_bounds("sidebar-search-button").is_some());
    assert!(cx.debug_bounds("row-agent-p1").is_some());
}

#[cfg(target_os = "linux")]
#[gpui::test]
fn menu_button_opens_the_compact_menu_anchored_to_it(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    let header = cx.debug_bounds("sidebar-titlebar").unwrap();
    let button = cx.debug_bounds("app-menu-button").unwrap();
    assert!(header.contains(&button.center()));
    // The menu bar's headings live behind the button now.
    assert!(cx.debug_bounds("application-menu-bar").is_none());
    cx.simulate_click(button.center(), Modifiers::none());
    draw(cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.menu.page),
        Some(crate::menu::Page::Application)
    );
    let panel = cx.debug_bounds("application-menu-panel").unwrap();
    assert!(
        (panel.top() - button.bottom()).abs() <= px(1.),
        "{panel:?} {button:?}"
    );
    assert!(panel.left() <= button.left() + px(1.) && panel.right() > button.right());
    assert!(cx.debug_bounds("application-item-File").is_some());
}

#[gpui::test]
fn tree_has_no_heading_row_or_new_menu_footer(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    for gone in ["header-tree", "new-workspace", "sidebar-menu"] {
        assert!(cx.debug_bounds(gone).is_none(), "{gone}");
    }
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.config.layout.mode = LayoutMode::default();
            cx.notify();
        })
    });
    draw(cx);
    for kept in ["header-spaces", "new-workspace", "sidebar-menu"] {
        assert!(cx.debug_bounds(kept).is_some(), "{kept}");
    }
    // Only the tree takes its search into the header.
    assert!(cx.debug_bounds("sidebar-search-button").is_none());
}

#[gpui::test]
fn device_footer_keeps_picker_sessions_and_settings(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = tree_window(cx, false);
    for selector in ["device-picker", "device-sessions", "device-settings"] {
        assert!(cx.debug_bounds(selector).is_some(), "{selector}");
    }
    // Sessions and Settings are buttons of one icon, with no words beside it.
    for button in ["device-sessions", "device-settings"] {
        let bounds = cx.debug_bounds(button).unwrap();
        assert!(bounds.size.width <= px(28.), "{button}: {bounds:?}");
    }
    let picker = cx.debug_bounds("device-picker").unwrap();
    let footer = cx.debug_bounds("device-footer").unwrap();
    assert!(picker.left() >= footer.left() && picker.right() < footer.right());
}
