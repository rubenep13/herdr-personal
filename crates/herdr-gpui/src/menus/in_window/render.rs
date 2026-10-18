//! The application menu's button and its bounded, scrollable dropdown.

use super::*;
use crate::{
    design::{ItemState, Leading},
    fonts::StyledFont,
    menus::icons::{action_icon, category_icon},
};
use gpui::prelude::*;

/// The menu tree's widths at `[ui] size = 12`: the categories, each
/// submenu, and the gap between a level and the next.
const CATEGORIES_WIDTH: f32 = 240.;
const SUBMENU_WIDTH: f32 = 340.;
const SUBMENU_GAP: f32 = 4.;
/// Keeps rows' element IDs apart across levels.
const ITEMS_PER_LEVEL: usize = 1000;

impl HerdrWindow {
    /// The button that opens the application menu where Linux has no global
    /// menu bar: the menus' names drop down under it, each opening in place.
    /// It records where it was painted, so the list drops from it wherever
    /// the window's chrome puts it.
    pub(crate) fn application_menu_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let bar = self.menu.application_bar.bar.clone();
        let buttons = self.menu.application_bar.buttons.clone();
        let theme = &self.theme;
        let active = theme.active;
        div()
            .id("app-menu-button")
            .debug_selector(|| "app-menu-button".into())
            .relative()
            .size(px(crate::design::ICON_BUTTON))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(crate::design::ICON_BUTTON_RADIUS))
            .cursor_pointer()
            .when(self.menu.application.is_some(), |button| {
                button.bg(rgb(active))
            })
            .hover(move |style| style.bg(rgb(active)))
            // A press must never start a window move or open the
            // compositor's title-bar menu.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(
                svg()
                    .path("icons/menu.svg")
                    .size(px(16.))
                    .text_color(rgb(theme.muted)),
            )
            .child(
                canvas(
                    move |bounds, _, _| {
                        bar.set(bounds);
                        let mut buttons = buttons.borrow_mut();
                        buttons.clear();
                        buttons.push(bounds);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                if this.menu.application.is_some() {
                    this.dismiss_menu(window, cx);
                } else {
                    this.open_application_menu(None, window, cx);
                }
            }))
    }

    fn application_shortcut(&self, action: &dyn Action, cx: &App) -> String {
        if let Some(command) = action.as_any().downcast_ref::<crate::RunCommand>() {
            return self
                .keymap()
                .shortcuts(command.command)
                .next()
                .unwrap_or_default()
                .replace("cmd-", "super-");
        }
        cx.key_bindings()
            .borrow()
            .bindings_for_action(action)
            .last()
            .map(|binding| {
                binding
                    .keystrokes()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    }

    pub(crate) fn render_application_menu(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut overlay = div()
            .id("application-menu-overlay")
            .debug_selector(|| "application-menu-overlay".into())
            .absolute()
            .inset_0()
            .occlude()
            .track_focus(&self.menu.focus)
            .on_key_down(cx.listener(Self::application_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.dismiss_menu(window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.dismiss_menu(window, cx);
                }),
            )
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
        let Some(menu) = &self.menu.application else {
            return overlay;
        };
        let bar = self.menu.application_bar.bar.get();
        let content = self.menu.application_bar.content.get();
        let button = self
            .menu
            .application_bar
            .buttons
            .borrow()
            .first()
            .copied()
            .unwrap_or(bar);
        let look = self.menu_look();
        let room = (content.size.width - px(16.)).max(px(0.));
        // The menus as a tree: the categories, then each submenu the path
        // opens, beside the row that opened it.
        let mut levels = vec![
            menu.menus
                .iter()
                .cloned()
                .map(OwnedMenuItem::Submenu)
                .collect::<Vec<_>>(),
        ];
        for index in &menu.path {
            let Some(OwnedMenuItem::Submenu(next)) =
                levels.last().and_then(|items| items.get(*index))
            else {
                break;
            };
            levels.push(next.items.clone());
        }
        let depth = levels.len() - 1;
        let mut width = look.at(CATEGORIES_WIDTH).min(room);
        let mut origin = point(
            button
                .left()
                .min((content.right() - width - px(8.)).max(content.left() + px(8.)))
                .max(content.left() + px(8.)),
            bar.bottom(),
        );
        for (level, items) in levels.into_iter().enumerate() {
            let heights: Vec<Pixels> = items
                .iter()
                .map(|item| match item {
                    OwnedMenuItem::Separator => px(look.separator_height()),
                    _ => px(look.row_height()),
                })
                .collect();
            let max_height = if level == 0 {
                (content.bottom() - origin.y - px(8.)).max(px(0.))
            } else {
                (content.size.height - px(16.)).max(px(0.))
            };
            overlay = overlay.child(
                anchored()
                    .position(origin)
                    .snap_to_window_with_margin(Edges::all(px(8.)))
                    .child(self.application_panel((level, depth), items, (width, max_height), cx)),
            );
            // The next level opens beside this one, its first row level with
            // the row that opened it; without room on the right it opens on
            // the left, and failing that it overlaps.
            let Some(&opened) = menu.path.get(level) else {
                break;
            };
            let next_width = look.at(SUBMENU_WIDTH).min(room);
            let right = origin.x + width + look.at(SUBMENU_GAP);
            let left = origin.x - next_width - look.at(SUBMENU_GAP);
            let x = if right + next_width <= content.right() - px(8.) {
                right
            } else if left >= content.left() + px(8.) {
                left
            } else {
                (content.right() - next_width - px(8.)).max(content.left() + px(8.))
            };
            let above: Pixels = heights.iter().take(opened).copied().sum();
            origin = point(x, origin.y + above);
            width = next_width;
        }
        overlay
    }

    /// One level of the menu tree. The row the path continues through stays
    /// highlighted; on the deepest level, the selection is.
    fn application_panel(
        &self,
        (level, depth): (usize, usize),
        items: Vec<OwnedMenuItem>,
        (width, max_height): (Pixels, Pixels),
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let font = &self.config.ui;
        let look = self.menu_look();
        let Some(menu) = &self.menu.application else {
            return div().id(("application-menu-panel", level));
        };
        let highlighted = if level < depth {
            menu.path.get(level).copied()
        } else {
            menu.selected
        };
        let mut rows = div()
            .id(("application-menu-rows", level))
            .flex()
            .flex_col()
            .min_h_0()
            .overflow_y_scroll()
            .when(level == depth, |rows| rows.track_scroll(&menu.scroll));
        for (index, item) in items.into_iter().enumerate() {
            // A submenu ends in a chevron; an action, in its shortcut.
            let (name, disabled, checked, icon, shortcut) = match item {
                OwnedMenuItem::Separator => {
                    rows = rows.child(
                        look.separator()
                            .debug_selector(move || format!("application-separator-{index}")),
                    );
                    continue;
                }
                OwnedMenuItem::Submenu(menu) => (
                    menu.name.to_string(),
                    menu.disabled,
                    false,
                    category_icon(&menu.name),
                    None,
                ),
                OwnedMenuItem::Action {
                    name,
                    action,
                    disabled,
                    checked,
                    ..
                } => {
                    let shortcut = self.application_shortcut(action.as_ref(), cx);
                    (
                        name,
                        disabled,
                        checked,
                        action_icon(action.as_ref()),
                        Some(shortcut),
                    )
                }
                OwnedMenuItem::SystemMenu(menu) => (
                    menu.name.to_string(),
                    true,
                    false,
                    None,
                    Some(String::new()),
                ),
            };
            let submenu = shortcut.is_none();
            let selector = format!("application-item-{name}");
            let leading = match icon {
                _ if checked => Leading::Check,
                Some(icon) => Leading::Icon(icon, format!("{selector}-icon")),
                None => Leading::Empty,
            };
            let state = ItemState {
                selected: highlighted == Some(index),
                disabled,
                danger: false,
            };
            let end = match shortcut {
                Some(shortcut) => look
                    .keys(&format!("{selector}-detail"), &shortcut)
                    .into_any_element(),
                None => look
                    .submenu(format!("application-submenu-{name}"))
                    .into_any_element(),
            };
            rows = rows.child(
                look.row(
                    ("application-item", level * ITEMS_PER_LEVEL + index),
                    selector.clone(),
                    leading,
                    state,
                )
                .child(look.label(format!("{selector}-label"), name))
                .child(end)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    if disabled {
                        return;
                    }
                    if submenu {
                        if *hovered {
                            this.application_open(level, index, cx);
                        }
                    } else {
                        this.application_hover(level, index, *hovered, cx);
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.application_click(level, index, window, cx);
                })),
            );
        }
        let selector = if level == 0 {
            "application-menu-panel".to_owned()
        } else {
            format!("application-menu-panel-{level}")
        };
        let cover = self.menu.cover.clone();
        div()
            .id(("application-menu-panel", level))
            .debug_selector(move || selector)
            .relative()
            .w(width)
            .max_h(max_height)
            .flex()
            .flex_col()
            .overflow_hidden()
            .p(px(6.))
            .map(|panel| look.panel(panel))
            .text_color(look.ink(ItemState::default()))
            .text_font(font)
            .text_size(px(font.size))
            .line_height(px(font.line_height()))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(rows)
            .child(
                canvas(
                    // The categories lay out first each frame and reset what
                    // the menu covers; each open submenu adds its own panel.
                    move |bounds, _, _| {
                        let bounds = bounds.dilate(px(8.));
                        cover.set(match cover.get() {
                            crate::menu::Cover::Panel(covered) if level > 0 => {
                                crate::menu::Cover::Panel(covered.union(&bounds))
                            }
                            _ => crate::menu::Cover::Panel(bounds),
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
    }
}
