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

/// The categories stay listed and a category's entries open beside it, its
/// first entry level with the category's row; separators keep their inset.
#[gpui::test]
fn a_submenu_opens_beside_its_category(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1200.), px(800.)));
    draw(cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_application_menu(Some(1), window, cx)
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("application-menu-back").is_none());
    let categories = cx.debug_bounds("application-menu-panel").unwrap();
    let entries = cx.debug_bounds("application-menu-panel-1").unwrap();
    assert!(
        entries.left() >= categories.right(),
        "{entries:?} {categories:?}"
    );
    let file = cx.debug_bounds("application-item-File").unwrap();
    let first = cx.debug_bounds("application-item-New Workspace").unwrap();
    assert!(
        (first.top() - file.top()).abs() <= px(1.),
        "{first:?} {file:?}"
    );
    assert_eq!(first.size.height, px(32.));
    // File keeps closing apart from creating and going to.
    let separator = cx.debug_bounds("application-separator-6").unwrap();
    assert_eq!(separator.left(), first.left() + px(10.));
    assert_eq!(separator.right(), first.right() - px(10.));
    assert_eq!(separator.size.height, px(1.));
}

/// Resting the pointer on a category opens it, without a click, and moving
/// to another category opens that one instead.
#[gpui::test]
fn hovering_a_category_opens_its_submenu(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1200.), px(800.)));
    draw(cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| view.open_application_menu(None, window, cx))
    });
    draw(cx);
    let path = |cx: &mut VisualTestContext| {
        view.read_with(cx, |view, _| {
            view.menu.application.as_ref().unwrap().path.clone()
        })
    };
    let file = cx.debug_bounds("application-item-File").unwrap();
    cx.simulate_mouse_move(file.center(), None, Modifiers::none());
    draw(cx);
    assert_eq!(path(cx), [1]);
    assert!(cx.debug_bounds("application-item-New Workspace").is_some());
    let edit = cx.debug_bounds("application-item-Edit").unwrap();
    cx.simulate_mouse_move(edit.center(), None, Modifiers::none());
    draw(cx);
    assert_eq!(path(cx), [2]);
    assert!(cx.debug_bounds("application-item-New Workspace").is_none());
    assert!(cx.debug_bounds("application-item-Copy").is_some());
}

/// Resting on an entry closes a submenu opened deeper than it.
#[gpui::test]
fn hovering_an_entry_closes_a_deeper_submenu(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1200.), px(800.)));
    draw(cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_application_menu(Some(3), window, cx)
        })
    });
    draw(cx);
    let path = |cx: &mut VisualTestContext| {
        view.read_with(cx, |view, _| {
            view.menu.application.as_ref().unwrap().path.clone()
        })
    };
    let layout = cx.debug_bounds("application-item-Layout").unwrap();
    cx.simulate_mouse_move(layout.center(), None, Modifiers::none());
    draw(cx);
    assert_eq!(path(cx).len(), 2);
    assert!(cx.debug_bounds("application-menu-panel-2").is_some());
    let bigger = cx
        .debug_bounds("application-item-Increase Font Size")
        .unwrap();
    cx.simulate_mouse_move(bigger.center(), None, Modifiers::none());
    draw(cx);
    assert_eq!(path(cx), [3]);
    assert!(cx.debug_bounds("application-menu-panel-2").is_none());
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
    // Out past the submenu, the last level open.
    let panel = cx.debug_bounds("application-menu-panel-1").unwrap();
    cx.simulate_mouse_move(
        point(panel.right() + px(20.), panel.bottom() + px(20.)),
        None,
        Modifiers::none(),
    );
    draw(cx);
    assert_eq!(selected(cx), None);
}
