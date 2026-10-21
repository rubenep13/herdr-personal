//! The notice that a daemon refused this app's handshake and which side must
//! be updated. The update command is shown and copied, never run: updating a
//! daemon may stop its sessions, so the user decides when.

use super::Page;
use crate::{
    HerdrWindow,
    design::{ButtonKind, Header},
    notifications::safe_text,
    window::Flash,
};
use gpui::{prelude::*, *};
use herdr_client::{ConnectTarget, MIN_HERDR_VERSION, VersionMismatch};

pub(crate) const UPDATE_COMMAND: &str = "herdr update";

/// What the open notice describes, captured as it opens so a retry that
/// resets the connection cannot change the text under the reader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Notice {
    /// The SSH host's label, or `None` for this device.
    pub(crate) host: Option<String>,
    pub(crate) mismatch: VersionMismatch,
}

impl Notice {
    pub(crate) fn title(&self) -> &'static str {
        match self.mismatch {
            VersionMismatch::DaemonOutdated { .. } => "Herdr needs an update",
            VersionMismatch::ClientOutdated { .. } => "Herdr UI needs an update",
        }
    }

    pub(crate) fn body(&self) -> String {
        let place = match &self.host {
            Some(host) => safe_text(host, 64),
            None => "this device".into(),
        };
        let version = self.mismatch.server_version();
        let server = match version {
            Some(version) => format!("The Herdr server on {place} (version {version})"),
            None => format!("The Herdr server on {place}"),
        };
        // Releases before the endpoint protocol report no version to this
        // app, so only the requirement can be stated, not their age.
        let problem = match version {
            Some(_) => format!("{server} is too old for this app."),
            None => format!(
                "{server} did not answer this app's handshake. This app needs Herdr {MIN_HERDR_VERSION} or newer."
            ),
        };
        let wherever = match &self.host {
            Some(_) => format!("on {place}"),
            None => "in a terminal".into(),
        };
        match &self.mismatch {
            VersionMismatch::DaemonOutdated { .. } => format!(
                "{problem} Run this command {wherever}; the app reconnects once the updated server is running."
            ),
            VersionMismatch::ClientOutdated { .. } => {
                format!("{server} is newer than this app supports. Update this app to connect.")
            }
        }
    }
}

impl HerdrWindow {
    /// Announces the selected endpoint's refused handshake once, when no other
    /// popup is open; retries that are refused again stay quiet.
    pub(crate) fn announce_version_mismatch(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.endpoints[self.selected_endpoint]
            .version_mismatch
            .is_none()
        {
            self.version_notice_shown = false;
        } else if !self.version_notice_shown && self.menu.page.is_none() {
            self.version_notice_shown = true;
            self.show_version_mismatch(window, cx);
        }
    }

    /// Opens the notice for the selected endpoint's refused handshake.
    pub(crate) fn show_version_mismatch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let endpoint = &self.endpoints[self.selected_endpoint];
        let Some(mismatch) = endpoint.version_mismatch.clone() else {
            return;
        };
        let host = matches!(
            endpoint.connection.target,
            ConnectTarget::Ssh { .. } | ConnectTarget::Wsl { .. }
        )
        .then(|| endpoint.label.clone());
        if !self.open_menu(window, cx) {
            return;
        }
        self.menu.version_notice = Some(Notice { host, mismatch });
        self.menu.page = Some(Page::VersionMismatch);
    }

    /// The notice's primary action, also bound to Enter.
    pub(super) fn act_on_version_mismatch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(notice) = &self.menu.version_notice else {
            return;
        };
        match notice.mismatch {
            VersionMismatch::DaemonOutdated { .. } => {
                cx.write_to_clipboard(ClipboardItem::new_string(UPDATE_COMMAND.into()));
                self.show_flash(Flash::success(format!("Copied `{UPDATE_COMMAND}`")), cx);
                self.dismiss_menu(window, cx);
            }
            VersionMismatch::ClientOutdated { .. } => self.open_app_update(false, window, cx),
        }
    }

    pub(super) fn render_version_mismatch(&self, cx: &mut Context<Self>) -> Div {
        let Some(notice) = &self.menu.version_notice else {
            return div();
        };
        let look = self.dialog_look();
        let daemon = matches!(notice.mismatch, VersionMismatch::DaemonOutdated { .. });
        let title = notice.title();
        let close = |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            this.dismiss_menu(window, cx);
        };
        div()
            .debug_selector(|| "version-mismatch".into())
            .flex()
            .flex_col()
            .child(
                look.header(
                    Header {
                        selector: "version-mismatch-header",
                        icon: "icons/alert.svg",
                        title: ("version-mismatch-title", title),
                        sub: None,
                        danger: false,
                    },
                    look.menu()
                        .close_button("version-mismatch-cross")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            close(this, window, cx);
                        })),
                ),
            )
            .child(
                look.body("version-mismatch-sheet")
                    .child(look.lead("version-mismatch-body", notice.body()))
                    .when(daemon, |body| {
                        body.child(
                            look.well(
                                "version-mismatch-command".into(),
                                Some(&self.config.terminal),
                            )
                            .child(UPDATE_COMMAND),
                        )
                    }),
            )
            .child(
                look.footer("version-mismatch-footer")
                    .hint("Esc", "cancel")
                    .hint("↵", if daemon { "copy" } else { "check" })
                    .button(
                        look.button(
                            "version-mismatch-dismiss",
                            "Dismiss",
                            ButtonKind::Normal,
                            true,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                close(this, window, cx);
                            },
                        )),
                    )
                    .button(
                        look.button(
                            "version-mismatch-action",
                            if daemon {
                                "Copy Command"
                            } else {
                                "Check for Updates"
                            },
                            ButtonKind::Primary,
                            true,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            cx.stop_propagation();
                            this.act_on_version_mismatch(window, cx);
                        })),
                    ),
            )
    }
}
