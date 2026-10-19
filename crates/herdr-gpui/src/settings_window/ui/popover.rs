//! Dropdown: a button that opens a floating option list beneath itself. The
//! list renders inside the button's subtree via `deferred(anchored(..))`, so
//! no bounds are passed around and it paints above the page.
use crate::{
    design::{ItemState, Leading, MenuLook, SettingsLook},
    settings_window::inputs::DropdownField,
};
use gpui::{prelude::*, *};
use std::{cell::RefCell, collections::HashSet, rc::Rc};

thread_local! {
    /// Dropdowns whose list was open when the button went down; the click that
    /// follows closes instead of reopening.
    static CLOSING: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
}

/// The open popover the view holds in `self.dropdown: Option<Dropdown>`.
pub(in crate::settings_window) struct Dropdown {
    pub field: DropdownField,
    pub options: Vec<SharedString>,
    /// The option the keys or the pointer are on.
    pub selected: usize,
    /// The option in effect when the list opened; it keeps the check, and the
    /// highlight returns to it when the pointer leaves a row.
    pub current: usize,
    focus: FocusHandle,
}

impl Dropdown {
    pub(in crate::settings_window) fn open(
        field: DropdownField,
        options: Vec<SharedString>,
        selected: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            field,
            options,
            selected,
            current: selected,
            focus,
        }
    }

    /// The pointer entered or left option `index`.
    pub(in crate::settings_window) fn hover(&mut self, index: usize, hovered: bool) -> bool {
        let next = if hovered {
            index
        } else if self.selected == index {
            self.current
        } else {
            return false;
        };
        let changed = self.selected != next;
        self.selected = next;
        changed
    }

    pub(in crate::settings_window) fn focus(&self) -> &FocusHandle {
        &self.focus
    }

    /// Up/Down/Enter/Escape. Returns `Some(index)` on Enter, `Some(usize::MAX)`
    /// on Escape (the caller closes), else `None`.
    pub(in crate::settings_window) fn key(&mut self, event: &KeyDownEvent) -> Option<usize> {
        let count = self.options.len();
        match event.keystroke.key.as_str() {
            "down" if count > 0 => self.selected = (self.selected + 1) % count,
            "up" if count > 0 => self.selected = (self.selected + count - 1) % count,
            "enter" => return (self.selected < count).then_some(self.selected),
            "escape" => return Some(usize::MAX),
            _ => {}
        }
        None
    }
}

/// The button, plus its list when `open` is this dropdown.
#[allow(clippy::too_many_arguments)]
pub(in crate::settings_window) fn dropdown(
    look: &SettingsLook,
    menu: &MenuLook,
    id: &'static str,
    label: impl Into<SharedString>,
    shared_prefix: bool,
    enabled: bool,
    open: Option<&Dropdown>,
    on_open: impl Fn(&mut Window, &mut App) + 'static,
    on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
    on_hover: impl Fn(usize, bool, &mut Window, &mut App) + 'static,
    on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let label: SharedString = label.into();
    let open = open.filter(|_| enabled);
    let was_open = open.is_some();
    let on_dismiss = Rc::new(on_dismiss);
    let dismiss_button = on_dismiss.clone();
    let dismiss_list = on_dismiss;
    let prefix = "Use Herdr setting · ";
    let label_el = match label.strip_prefix(prefix).filter(|_| shared_prefix) {
        Some(rest) => div()
            .flex()
            .child(div().text_color(look.muted()).child(prefix))
            .child(rest.to_string()),
        None => div().child(label.clone()),
    };
    let ring = look.raised();
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .relative()
        .flex()
        .items_center()
        .justify_between()
        .gap(look.at(8.))
        .flex_none()
        .h(px(look.control_height()))
        .min_w(look.at(170.))
        .pl(look.at(12.))
        .pr(look.at(8.))
        .rounded(look.at(8.))
        .border_1()
        .border_color(look.line())
        .bg(look.well())
        .text_size(px(look.scale().name))
        .text_color(look.title())
        .whitespace_nowrap()
        .when(!enabled, |button| button.opacity(0.5))
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(move |button| button.border_color(ring))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    if was_open {
                        CLOSING.with(|closing| closing.borrow_mut().insert(id));
                        dismiss_button(window, cx);
                    } else {
                        // A press that never became a click leaves a stale flag.
                        CLOSING.with(|closing| closing.borrow_mut().remove(id));
                    }
                })
                .on_click(move |_, window, cx| {
                    if !CLOSING.with(|closing| closing.borrow_mut().remove(id)) {
                        on_open(window, cx);
                    }
                })
        })
        .child(label_el)
        .child(
            svg()
                .path("icons/chevron-down.svg")
                .size(look.at(14.))
                .flex_none()
                .text_color(look.muted()),
        )
        .children(open.map(|open| {
            // A point at the button's bottom right corner: the list hangs
            // from it, its right edge along the button's.
            div().absolute().top_full().right_0().mt(look.at(4.)).child(
                deferred(
                    anchored()
                        .anchor(Anchor::TopRight)
                        .snap_to_window_with_margin(px(8.))
                        .child(popover_list(
                            menu,
                            open,
                            on_pick,
                            on_hover,
                            move |window, cx| dismiss_list(window, cx),
                        )),
                )
                .with_priority(1),
            )
        }))
}

/// The floating list: the redesigned menus' panel and rows, the option in
/// effect checked.
pub(in crate::settings_window) fn popover_list(
    look: &MenuLook,
    open: &Dropdown,
    on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
    on_hover: impl Fn(usize, bool, &mut Window, &mut App) + 'static,
    on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id = open.field.selector();
    let on_pick = Rc::new(on_pick);
    let on_hover = Rc::new(on_hover);
    look.panel(
        div()
            .id(SharedString::from(format!("{id}-list")))
            .debug_selector(move || format!("{id}-list"))
            .min_w(look.at(LIST_WIDTH))
            .max_h(look.at(LIST_HEIGHT))
            .overflow_y_scroll()
            .p(look.at(6.))
            .flex()
            .flex_col(),
    )
    .occlude()
    .track_focus(&open.focus)
    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .on_click(|_, _, cx| cx.stop_propagation())
    .on_mouse_down_out(move |_, window, cx| on_dismiss(window, cx))
    .children(open.options.iter().enumerate().map(|(index, option)| {
        let (on_pick, on_hover) = (on_pick.clone(), on_hover.clone());
        let state = ItemState {
            selected: index == open.selected,
            ..ItemState::default()
        };
        look.row(
            (id, index),
            format!("{id}-option-{index}"),
            Leading::Empty,
            state,
        )
        .child(look.label(format!("{id}-option-{index}-label"), option.clone()))
        .when(index == open.current, |row| {
            row.child(
                svg()
                    .path("icons/check.svg")
                    .size(look.at(16.))
                    .flex_none()
                    .text_color(look.accent())
                    .debug_selector(move || format!("{id}-option-{index}-check")),
            )
        })
        .on_hover(move |hovered, window, cx| on_hover(index, *hovered, window, cx))
        .on_click(move |_, window, cx| on_pick(index, window, cx))
    }))
}

/// The list's least width and its height before it scrolls, at the default
/// `[ui]` size.
const LIST_WIDTH: f32 = 200.;
const LIST_HEIGHT: f32 = 300.;

#[cfg(test)]
mod tests;
