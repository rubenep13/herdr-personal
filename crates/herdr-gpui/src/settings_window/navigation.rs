//! The section list and the footer: grouped sections under a search box, and
//! the save status line.
use super::*;
use crate::design::{ButtonKind, ButtonSize};

impl SettingsWindow {
    pub(super) fn navigation(&self, search_focused: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        use crate::panel_resize::{PanelDrag, Side, handle};
        let look = self.look();
        div()
            .id("settings-navigation")
            .debug_selector(|| "settings-navigation".into())
            .relative()
            .w(px(self.navigation_width.width(self.viewport_width)))
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<PanelDrag>, _, cx| {
                    if *event.drag(cx) == PanelDrag::SettingsNavigation
                        && this.navigation_width.drag(
                            Side::Left,
                            event.bounds,
                            event.event.position.x,
                        )
                    {
                        cx.notify();
                    }
                }),
            )
            .child(handle(
                "settings-navigation-resize",
                Side::Left,
                PanelDrag::SettingsNavigation,
                cx.listener(|this, _, _, cx| {
                    this.navigation_width.reset();
                    cx.notify();
                }),
            ))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .py(look.at(10.))
            .px(look.at(8.))
            .bg(look.nav())
            .border_r_1()
            .border_color(look.line())
            .child(
                look.field("settings-nav-search".into(), search_focused)
                    .mb(look.at(6.))
                    .text_color(look.muted())
                    .child(
                        svg()
                            .path("icons/search.svg")
                            .size(look.at(15.))
                            .flex_none()
                            .text_color(look.muted()),
                    )
                    .child(div().flex_1().min_w_0().child(self.nav_search.clone())),
            )
            .child(
                self.scroll_area(
                    scrollbar::Pane::Sections,
                    div()
                        .id("settings-sections")
                        .debug_selector(|| "settings-sections".into())
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .overflow_y_scroll()
                        .track_scroll(&self.navigation_scroll)
                        // Flat, so `Section::nav_item` can scroll to a page.
                        .children(Group::ALL.into_iter().flat_map(|group| {
                            std::iter::once(self.group_label(group).into_any_element()).chain(
                                Section::ALL
                                    .iter()
                                    .copied()
                                    .enumerate()
                                    .filter(move |(_, section)| section.group() == group)
                                    .map(|(index, section)| {
                                        self.navigation_row(index, section, cx).into_any_element()
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        })),
                    cx,
                ),
            )
    }

    fn group_label(&self, group: Group) -> Div {
        let look = self.look();
        div()
            .debug_selector(move || format!("settings-group-{}", group.label().to_lowercase()))
            .flex_none()
            .px(look.at(10.))
            .pt(look.at(12.))
            .pb(look.at(4.))
            .text_size(px(look.scale().detail))
            .font_weight(FontWeight::MEDIUM)
            .text_color(look.muted())
            .child(group.label())
    }

    fn navigation_row(
        &self,
        index: usize,
        section: Section,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let look = self.look();
        let selected = self.section == section;
        let hover = look.nav_hover();
        div()
            .id(("settings-section", index))
            .relative()
            .map(|row| {
                #[cfg(all(feature = "integration-test", target_os = "macos"))]
                let row = row.child(native::probe(index));
                row
            })
            .debug_selector(move || section.selector())
            .flex_none()
            .h(px(look.nav_row_height()))
            .flex()
            .items_center()
            .gap(look.at(10.))
            .px(look.at(10.))
            .rounded(look.at(8.))
            .cursor_pointer()
            .text_size(px(look.scale().name))
            .text_color(if selected { look.title() } else { look.text() })
            .when(selected, |row| row.bg(look.selection()))
            .when(!selected, |row| row.hover(move |row| row.bg(hover)))
            .child(
                svg()
                    .path(section.icon())
                    .size(look.at(16.))
                    .flex_none()
                    .text_color(if selected { look.title() } else { look.muted() })
                    .debug_selector(move || format!("{}-icon", section.selector())),
            )
            .child(div().min_w_0().truncate().child(section.label()))
            .on_click(
                cx.listener(move |this, _, window, cx| this.select_section(section, window, cx)),
            )
    }

    fn pending_drafts(&self) -> Vec<&'static str> {
        [
            (self.theme_dirty(), "theme"),
            (self.layout_intent.is_some(), "layout"),
            (self.sidebar_dirty(), "sidebar"),
        ]
        .into_iter()
        .filter_map(|(dirty, name)| dirty.then_some(name))
        .collect()
    }

    pub(super) fn footer_text(&self) -> String {
        let pending = self.pending_drafts();
        if let Some(error) = &self.error {
            error.clone()
        } else if self.loading {
            "Loading preferences...".into()
        } else if let Some(status) = &self.status {
            status.clone()
        } else if pending.is_empty() {
            match self.section {
                // Its account fields are a form with their own Save button.
                #[cfg(feature = "cloud")]
                Section::CloudDevices => {
                    "Account fields save with Save; sign-in and removal apply at once.".into()
                }
                _ => "All changes saved.".into(),
            }
        } else {
            format!(
                "Unsaved: {} — saved when this window closes",
                pending.join(", ")
            )
        }
    }

    pub(super) fn footer(&self, cx: &mut Context<Self>) -> Div {
        let look = self.look();
        let pending = self.pending_drafts();
        let text = self.footer_text();
        div()
            .debug_selector(|| "settings-footer".into())
            .flex_none()
            .min_h(look.at(36.))
            .px(look.at(16.))
            .flex()
            .items_center()
            .gap(look.at(12.))
            .bg(look.well())
            .border_t_1()
            .border_color(look.line())
            .text_size(px(look.scale().secondary))
            .text_color(look.muted())
            .when(!pending.is_empty() && self.error.is_none(), |footer| {
                footer.child(div().size(look.at(7.)).rounded_full().bg(look.warn()))
            })
            .child(
                div()
                    .debug_selector(|| "settings-footer-status".into())
                    .flex_1()
                    .min_w_0()
                    .child(text),
            )
            .when(!pending.is_empty(), |footer| {
                footer.child(
                    look.button(
                        "settings-footer-save",
                        "settings-footer-save".into(),
                        "Save now",
                        ButtonKind::Primary,
                        ButtonSize::Small,
                        !self.busy(),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.save_pending_drafts(cx))),
                )
            })
            .child(
                look.button(
                    "settings-footer-reload",
                    "settings-footer-reload".into(),
                    "Reload",
                    ButtonKind::Normal,
                    ButtonSize::Small,
                    true,
                )
                .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
            )
    }
}
