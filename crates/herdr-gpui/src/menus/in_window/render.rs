//! The application menu's button and its bounded, scrollable dropdown.

use super::*;
use crate::{
    design::{ItemState, Leading},
    fonts::StyledFont,
    menus::icons::{action_icon, category_icon},
};
use gpui::prelude::*;

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
        let overlay = div()
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
        let font = &self.config.ui;
        let look = self.menu_look();
        let width = px(380. * font.size / 12.).min((content.size.width - px(16.)).max(px(0.)));
        let top = bar.bottom();
        let current = menu.current();
        let items = current.map(|menu| menu.items.clone()).unwrap_or_else(|| {
            menu.menus
                .iter()
                .cloned()
                .map(OwnedMenuItem::Submenu)
                .collect()
        });
        let mut rows = div()
            .id("application-menu-rows")
            .flex()
            .flex_col()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&menu.scroll);
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
            let selector = format!("application-item-{name}");
            let leading = match icon {
                _ if checked => Leading::Check,
                Some(icon) => Leading::Icon(icon, format!("{selector}-icon")),
                None => Leading::Empty,
            };
            let state = ItemState {
                selected: menu.selected == Some(index),
                disabled,
                danger: false,
            };
            let end = match shortcut {
                Some(shortcut) => look
                    .detail(format!("{selector}-detail"), shortcut)
                    .into_any_element(),
                None => look
                    .submenu(format!("application-submenu-{name}"))
                    .into_any_element(),
            };
            rows = rows.child(
                look.row(
                    ("application-item", index),
                    selector.clone(),
                    leading,
                    state,
                )
                .child(look.label(format!("{selector}-label"), name))
                .child(end)
                .on_hover(cx.listener(move |this, hovered, _, cx| {
                    if *hovered
                        && !disabled
                        && let Some(menu) = &mut this.menu.application
                    {
                        menu.selected = Some(index);
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.application_choose(index, window, cx);
                })),
            );
        }
        let cover = self.menu.cover.clone();
        let panel = div()
            .id("application-menu-panel")
            .debug_selector(|| "application-menu-panel".into())
            .relative()
            .w(width)
            .max_h((content.bottom() - top - px(8.)).max(px(0.)))
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
            .when_some(current, |panel, current| {
                panel
                    .child(
                        look.row(
                            "application-menu-back",
                            "application-menu-back".into(),
                            Leading::Icon(
                                "icons/chevron-left.svg",
                                "application-menu-back-icon".into(),
                            ),
                            ItemState::default(),
                        )
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(look.ink(ItemState {
                            selected: true,
                            ..ItemState::default()
                        }))
                        .child(
                            look.label("application-menu-back-label".into(), current.name.clone()),
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.application_back(cx);
                        })),
                    )
                    .child(look.separator())
            })
            .child(rows)
            .child(
                canvas(
                    move |bounds, _, _| {
                        cover.set(crate::menu::Cover::Panel(bounds.dilate(px(8.))));
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            );
        overlay.child(
            anchored()
                .position(point(
                    button
                        .left()
                        .min((content.right() - width - px(8.)).max(content.left() + px(8.)))
                        .max(content.left() + px(8.)),
                    top,
                ))
                .snap_to_window_with_margin(Edges::all(px(8.)))
                .child(panel),
        )
    }
}
