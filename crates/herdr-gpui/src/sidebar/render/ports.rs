//! A space's listening ports as chips: inside its row in the Tree layout, on
//! a line under it in the others.

use crate::{HerdrWindow, config::FontConfig, listening_ports::Daemon};
use gpui::{prelude::*, *};

impl HerdrWindow {
    /// The chips for the ports `workspace` of `endpoint` listens on, sized
    /// from `font`, or none while it listens on none or the setting hides
    /// them.
    pub(super) fn space_port_chips(
        &self,
        daemon: Option<&Daemon>,
        (endpoint, workspace): (&str, &str),
        font: &FontConfig,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        if !self.config.show_listening_ports {
            return None;
        }
        let listed = self.listening_ports.get(daemon?, workspace)?;
        let size = (font.size * 0.85).round();
        Some(
            div()
                .debug_selector(|| format!("ports-{endpoint}-{workspace}"))
                .flex_none()
                .min_w_0()
                .text_size(px(size))
                .child(crate::listening_ports::chips(
                    listed,
                    (endpoint, workspace),
                    &self.tunnels,
                    &self.theme,
                    self.menu_look(),
                    size,
                    cx,
                )),
        )
    }
}
