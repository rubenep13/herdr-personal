//! What the terminal area shows while the selected endpoint reconnects after a
//! drop: the last picture dimmed, and a card naming the host, why it dropped,
//! and a way to retry now. The endpoint keeps retrying on its own backoff and
//! the window stays on it; nothing typed meanwhile is sent.
use super::HerdrWindow;
use crate::notifications::safe_text;
use gpui::{prelude::*, *};

const MAX_WIDTH: f32 = 420.;
const MAX_REASON: usize = 240;
/// How much of the stale picture shows through.
const DIM_ALPHA: u32 = 0xa0;

impl HerdrWindow {
    /// Whether the card shows. A replacement connection's snapshot ends the
    /// outage before its first frame, so the card stays over the old picture
    /// until that frame lands.
    pub(crate) fn reconnecting(&self) -> bool {
        self.endpoints[self.selected_endpoint].outage().is_some() || self.presentation.stale()
    }

    pub(super) fn render_reconnecting(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.reconnecting() {
            return None;
        }
        let endpoint = &self.endpoints[self.selected_endpoint];
        let reason = endpoint.outage();
        let theme = &self.theme;
        let look = self.view_look();
        let (warn, _) = look.settings().tone(crate::design::Tone::Warn);
        let card = look
            .menu()
            .panel(div())
            .id("reconnecting")
            .debug_selector(|| "reconnecting".into())
            .min_w_0()
            .max_w(px(MAX_WIDTH))
            .occlude()
            .flex()
            .items_start()
            .gap(look.at(12.))
            .p(look.at(14.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(
                look.dialog()
                    .tile("reconnecting-tile".into(), "icons/wifi-off.svg", warn),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.at(6.))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(look.scale().name))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(look.title())
                            .child(format!(
                                "Reconnecting to {}",
                                safe_text(&endpoint.label, 80)
                            )),
                    )
                    .children(reason.map(|reason| {
                        div()
                            .debug_selector(|| "reconnecting-reason".into())
                            .text_size(px(look.scale().secondary))
                            .text_color(look.muted())
                            .child(safe_text(reason, MAX_REASON))
                    }))
                    .child(
                        div().flex().pt(look.at(4.)).child(
                            look.button(
                                "reconnect-now",
                                "reconnect-now".into(),
                                None,
                                "Reconnect now",
                                crate::design::ButtonKind::Normal,
                                true,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                this.reconnect(cx);
                                cx.notify();
                            })),
                        ),
                    ),
            );
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .p(px(16.))
                .when(self.presentation.stale(), |overlay| {
                    overlay.bg(rgba((theme.background << 8) | DIM_ALPHA))
                })
                .child(card)
                .into_any_element(),
        )
    }
}
