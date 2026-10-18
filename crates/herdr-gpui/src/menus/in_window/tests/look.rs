use super::*;
use crate::menus::icons::{action_icon, category_icon};
use gpui::{Menu, MenuItem};

/// Entries of the macOS app menu that Linux never shows.
const MAC_ONLY: [&str; 3] = ["Hide Herdr", "Hide Others", "Show All"];

fn assert_icons(menu: &Menu) {
    assert!(category_icon(&menu.name).is_some(), "{}", menu.name);
    for item in &menu.items {
        match item {
            MenuItem::Submenu(submenu) => assert_icons(submenu),
            // Layout's entries are choices: the check mark fills their slot.
            MenuItem::Action { name, .. } if menu.name.as_ref() == "Layout" => {
                assert!(!name.is_empty());
            }
            MenuItem::Action { name, action, .. } if !MAC_ONLY.contains(&name.as_ref()) => {
                assert!(action_icon(action.as_ref()).is_some(), "{name}");
            }
            _ => {}
        }
    }
}

#[test]
fn every_linux_menu_entry_has_an_icon() {
    for menu in crate::menus::menus(crate::config::Layout::default()) {
        if menu.name.as_ref() != "QA" {
            assert_icons(&menu);
        }
    }
}

#[gpui::test]
fn categories_show_an_icon_and_a_chevron(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    draw(cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| view.open_application_menu(None, window, cx))
    });
    draw(cx);
    let icon = cx.debug_bounds("application-item-File-icon").unwrap();
    assert_eq!(icon.size, size(px(16.), px(16.)));
    let label = cx.debug_bounds("application-item-File-label").unwrap();
    let chevron = cx.debug_bounds("application-submenu-File").unwrap();
    assert!(chevron.left() >= label.right());
}

#[gpui::test]
fn a_submenu_opens_under_a_back_row_and_an_inset_separator(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    draw(cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_application_menu(Some(1), window, cx)
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("application-menu-back").is_some());
    assert!(cx.debug_bounds("application-menu-back-icon").is_some());
    let row = cx.debug_bounds("application-item-New Workspace").unwrap();
    assert_eq!(row.size.height, px(32.));
    // File keeps closing apart from creating and going to.
    let separator = cx.debug_bounds("application-separator-6").unwrap();
    assert_eq!(separator.left(), row.left() + px(10.));
    assert_eq!(separator.right(), row.right() - px(10.));
    assert_eq!(separator.size.height, px(1.));
}

#[gpui::test]
fn shortcuts_are_drawn_as_chips(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    draw(cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_application_menu(Some(1), window, cx)
        })
    });
    draw(cx);
    let shortcut = view.read_with(cx, |view, _| {
        view.keymap()
            .primary(crate::controls::Command::Workspace)
            .to_owned()
    });
    let keys = crate::design::keycaps(&shortcut).count();
    assert!(keys > 0, "New Workspace has a default shortcut");
    for index in 0..=keys {
        let selector = format!("application-item-New Workspace-detail-key-{index}");
        assert_eq!(cx.debug_bounds(selector.leak()).is_some(), index < keys);
    }
}

/// The row under the pointer is selected; once the pointer leaves it, the
/// row reads as it did before.
#[gpui::test]
fn leaving_a_row_restores_it(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    draw(cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_application_menu(Some(1), window, cx)
        })
    });
    draw(cx);
    let selected = |cx: &mut VisualTestContext| {
        view.read_with(cx, |view, _| {
            view.menu.application.as_ref().unwrap().selected
        })
    };
    let row = cx.debug_bounds("application-item-New Tab").unwrap();
    cx.simulate_mouse_move(row.center(), None, Modifiers::none());
    draw(cx);
    assert!(selected(cx).is_some());
    let panel = cx.debug_bounds("application-menu-panel").unwrap();
    cx.simulate_mouse_move(
        point(panel.right() + px(20.), panel.bottom() + px(20.)),
        None,
        Modifiers::none(),
    );
    draw(cx);
    assert_eq!(selected(cx), None);
}
