//! Config diagnostics drawn like notification cards over the top-right of the
//! terminal area, where Herdr draws its own: this app's GUI config warning,
//! the missing icon font, then the selected endpoint's daemon `config.toml`
//! diagnostic.
use super::{Flash, HerdrWindow};
use crate::{fonts::StyledFont, notifications::safe_text};
use gpui::{prelude::*, *};
use std::sync::Arc;

const MAX_WIDTH: f32 = 420.;

impl HerdrWindow {
    /// This app's GUI config warning and icon font notice, then the selected
    /// daemon's diagnostic.
    pub(super) fn config_diagnostic_cards(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let gui = self.gui_config_diagnostic.visible().map(|lines| {
            let drawn = lines.clone();
            self.render_diagnostic_card(
                "gui-config-diagnostic",
                "Herdr UI".into(),
                lines,
                None,
                move |this, cx| {
                    if this.gui_config_diagnostic.dismiss(&drawn) {
                        cx.notify();
                    }
                },
                cx,
            )
        });
        let icon_font = self.icon_font_notice.visible().map(|lines| {
            let drawn = lines.clone();
            let command =
                crate::icon_font_notice::COMMAND.map(|command| self.command_chip(command, cx));
            self.render_diagnostic_card(
                "icon-font-notice",
                "Herdr UI".into(),
                lines,
                command,
                move |this, cx| {
                    if this.icon_font_notice.dismiss(&drawn) {
                        cx.notify();
                    }
                },
                cx,
            )
        });
        let endpoint = self
            .endpoints
            .get(self.selected_endpoint)
            .and_then(|endpoint| {
                let lines = endpoint.config_diagnostic.visible()?;
                let drawn = lines.clone();
                let endpoint_id = endpoint.id.clone();
                let generation = endpoint.generation;
                let inbox = endpoint.connection.inbox.clone();
                Some(self.render_diagnostic_card(
                    "config-diagnostic",
                    safe_text(&endpoint.label, 80),
                    lines,
                    None,
                    move |this, cx| {
                        this.dismiss_config_diagnostic(
                            &endpoint_id,
                            generation,
                            &inbox,
                            &drawn,
                            cx,
                        );
                    },
                    cx,
                ))
            });
        gui.into_iter()
            .chain(icon_font)
            .chain(endpoint)
            .map(IntoElement::into_any_element)
            .collect()
    }

    /// A command the card names, in the terminal face on its own chip so it
    /// reads as something to run. Card text cannot be selected, so a click
    /// anywhere on the chip copies the command, as its icon says; it is never
    /// run. A narrow card truncates the text and still copies all of it.
    fn command_chip(&self, command: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let look = self.dialog_look();
        let raised = look.settings().raised();
        let chip = div()
            .id("diagnostic-command")
            .debug_selector(|| "diagnostic-command".into())
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(look.at(10.))
            .py(look.at(7.))
            .rounded(look.at(8.))
            .bg(look.well_fill())
            .border_1()
            .border_color(look.line())
            .hover(move |chip| chip.border_color(raised))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                cx.write_to_clipboard(ClipboardItem::new_string(command.into()));
                this.show_flash(Flash::success("Copied the command"), cx);
            }))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_font(&self.config.terminal)
                    .text_size(look.at(12.))
                    .text_color(rgb(self.theme.foreground))
                    .child(command),
            )
            .child(
                div()
                    .debug_selector(|| "diagnostic-command-copy".into())
                    .flex_none()
                    .child(
                        svg()
                            .path("icons/copy.svg")
                            .size(look.at(14.))
                            .text_color(rgb(self.theme.muted)),
                    ),
            );
        // A row, so the chip keeps to its text and shrinks with the card. It
        // starts where the text does, past the tile and its gap.
        div()
            .min_w_0()
            .pl(look.at(39.))
            .flex()
            .child(chip)
            .into_any_element()
    }

    /// `footer` spans the card under its text, for a control the lines cannot
    /// carry.
    fn render_diagnostic_card(
        &self,
        selector: &'static str,
        label: String,
        lines: &Arc<[String]>,
        footer: Option<AnyElement>,
        dismiss: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let look = self.dialog_look();
        let accent = rgb(self.theme.ink(self.theme.palette[3]));
        let header = div()
            .min_w_0()
            .flex()
            .gap(look.at(11.))
            .child(look.tile(format!("{selector}-tile"), "icons/alert.svg", accent))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.at(3.))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(look.settings().scale().secondary))
                            .text_color(rgb(self.theme.muted))
                            .child(label),
                    )
                    .children(lines.iter().enumerate().map(|(index, line)| {
                        div()
                            .debug_selector(move || format!("{selector}-line-{index}"))
                            .truncate()
                            .text_size(px(look.settings().scale().name - 0.5))
                            .text_color(rgb(self.theme.subtext()))
                            .child(line.clone())
                    })),
            )
            .child(
                notice_cross(&look, &self.theme)
                    .id("dismiss")
                    .debug_selector(move || format!("{selector}-dismiss"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        dismiss(this, cx);
                    })),
            );
        div()
            .id(selector)
            .debug_selector(move || selector.into())
            .occlude()
            .min_w_0()
            .max_w(px(MAX_WIDTH))
            .flex()
            .flex_col()
            .gap(look.at(8.))
            .pt(look.at(11.))
            .pb(look.at(11.))
            .pl(look.at(12.))
            .pr(look.at(8.))
            .map(|card| look.menu().panel(card))
            .text_color(rgb(self.theme.foreground))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(header)
            .children(footer)
    }

    /// Dismisses the banner only for the connection and text it was drawn
    /// from: a click delivered after either changed does nothing.
    fn dismiss_config_diagnostic(
        &mut self,
        endpoint_id: &str,
        generation: u64,
        inbox: &Arc<std::sync::Mutex<crate::state::LiveState>>,
        lines: &Arc<[String]>,
        cx: &mut Context<Self>,
    ) {
        let Some(endpoint) = self.endpoints.iter_mut().find(|e| {
            e.id == endpoint_id
                && e.generation == generation
                && Arc::ptr_eq(&e.connection.inbox, inbox)
        }) else {
            return;
        };
        if endpoint.config_diagnostic.dismiss(lines) {
            cx.notify();
        }
    }
}

/// A notice's cross: the pickers' cross at 24, quiet in the grey. The
/// caller adds its id, selector and click.
pub(super) fn notice_cross(look: &crate::design::DialogLook, theme: &crate::config::Theme) -> Div {
    let hover = rgb(theme.active);
    div()
        .flex_none()
        .size(look.at(24.))
        .rounded(look.at(crate::design::ICON_BUTTON_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(
            svg()
                .path("icons/close.svg")
                .size(look.at(12.))
                .text_color(rgb(theme.muted)),
        )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
