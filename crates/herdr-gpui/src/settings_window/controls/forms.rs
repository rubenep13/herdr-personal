//! Form pieces for the pages that hold forms rather than setting rows (Code,
//! Plugins, Cloud devices): a titled card, and a labelled switch row. Both
//! follow the row system's look.
use super::*;
use crate::settings_window::ui;

impl SettingsWindow {
    /// A card of form content under its section's title.
    pub(in crate::settings_window) fn control_card(&self, title: &'static str) -> ui::FormCard {
        ui::form_card(&self.look(), title)
    }

    /// A clickable row with `label` and a switch, inside a form's card; the
    /// caller adds `.on_click` when `enabled`.
    pub(in crate::settings_window) fn control_switch(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        checked: bool,
        enabled: bool,
    ) -> Stateful<Div> {
        let look = self.look();
        let id = id.into();
        let toggle = format!("{id}-toggle");
        div()
            .id(ElementId::Name(id.clone()))
            .debug_selector(move || id.to_string())
            .flex()
            .items_center()
            .justify_between()
            .gap(look.at(16.))
            .min_h(px(look.control_height()))
            .when(enabled, |row| row.cursor_pointer())
            .when(!enabled, |row| row.opacity(0.5))
            .child(
                div()
                    .min_w_0()
                    .text_size(px(look.scale().name))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(look.title())
                    .child(label.into()),
            )
            .child(ui::toggle(&look, toggle, checked, true))
    }
}
