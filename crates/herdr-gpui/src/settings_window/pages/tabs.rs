//! Tabs & panes: tab bar, closing confirmations, pane focus and session restore.
use super::{SettingsWindow, on_value, ui};
use crate::{
    config::{TabContent, TabPlacement, preferences::Preference},
    design::SettingsLook,
    herdr_settings::{Edit, TabBarPosition},
    settings_window::remote_history::HostState,
};
use gpui::{prelude::*, *};

/// Why Herdr's own position is off while the tab strips do not follow it.
const FOLLOWS_HERDR: &str = "Used when Position follows Herdr.";

impl SettingsWindow {
    /// Where the tab strips sit and what they show, both this client's, then
    /// Herdr's shared tab bar keys. Herdr's position is only used while
    /// Position follows Herdr, so it depends on that row.
    fn tab_bar_group(
        &self,
        look: &SettingsLook,
        ready: bool,
        position: Option<TabBarPosition>,
        hide: bool,
        cx: &mut Context<Self>,
    ) -> ui::Group {
        let style = self.config.tabs_style;
        let following = style.placement == TabPlacement::Herdr;
        let open = !self.quitting && self.closing.is_none();
        let herdr_row = ui::row(look, "tabs-position", "Herdr's position")
            .sub()
            .tag(ui::Source::Shared);
        let herdr_row = if following {
            herdr_row
        } else {
            herdr_row.disabled(FOLLOWS_HERDR)
        };
        ui::group(look, "tab-bar", "Tab bar", None)
            .row(
                ui::row(look, "tabs-placement", "Position")
                    .tag(ui::Source::Local)
                    .description("Where the tab strips sit.")
                    .control(ui::segmented(
                        look,
                        "tabs-placement-control",
                        &[
                            (TabPlacement::Subheader, "Subheader"),
                            (TabPlacement::Bottom, "Bottom"),
                            (TabPlacement::Herdr, "Follow Herdr"),
                        ],
                        Some(style.placement),
                        open,
                        on_value(cx, |this, placement, _, cx| {
                            this.save_preference(Preference::TabsPosition(placement), cx)
                        }),
                    )),
            )
            .row(self.gate_shared(herdr_row).control(ui::segmented(
                look,
                "tabs-position-control",
                &[
                    (TabBarPosition::Top, "Top"),
                    (TabBarPosition::Bottom, "Bottom"),
                ],
                position,
                ready && following,
                on_value(cx, |this, choice, _, cx| {
                    this.save_shared(Edit::TabBarPosition(choice), cx)
                }),
            )))
            .row(
                ui::row(look, "tabs-content", "Content")
                    .tag(ui::Source::Local)
                    .description("What each tab shows besides its name.")
                    .control(ui::segmented(
                        look,
                        "tabs-content-control",
                        &[
                            (TabContent::Name, "Name"),
                            (TabContent::Agent, "Agent"),
                            (TabContent::Status, "Agent and status"),
                        ],
                        Some(style.content),
                        open,
                        on_value(cx, |this, content, _, cx| {
                            this.save_preference(Preference::TabsContent(content), cx)
                        }),
                    )),
            )
            .row(
                self.gate_shared(
                    ui::row(look, "tabs-hide-single", "Hide with a single tab")
                        .tag(ui::Source::Shared),
                )
                .control(
                    ui::toggle(look, "tabs-hide-single-toggle", hide, ready).when(
                        ready,
                        |toggle| {
                            toggle.on_click(cx.listener(move |this, _, _, cx| {
                                this.save_shared(Edit::HideSingleTabBar(!hide), cx)
                            }))
                        },
                    ),
                ),
            )
    }

    pub(in crate::settings_window) fn render_tabs(&self, cx: &mut Context<Self>) -> Div {
        let look = &self.look();
        let ready = self.controls_shared_ready();
        let shared = self.shared.as_ref();
        let position = shared.map(|shared| shared.tab_bar_position);
        let hide = shared.is_some_and(|shared| shared.hide_tab_bar_when_single_tab);
        let history = shared.is_some_and(|shared| shared.pane_history);
        let page = div()
            .flex()
            .flex_col()
            .child(self.tab_bar_group(look, ready, position, hide, cx))
            .child(
                ui::group(look, "closing", "Closing", None)
                    .row(
                        ui::row(look, "tabs-confirm-tab", "Confirm before closing a tab").control(
                            self.preference_toggle(
                                "tabs-confirm-tab-toggle",
                                self.config.confirm_close_tab,
                                Preference::ConfirmCloseTab(!self.config.confirm_close_tab),
                                cx,
                            ),
                        ),
                    )
                    .row(
                        ui::row(look, "tabs-confirm-pane", "Confirm before closing a pane")
                            .control(self.preference_toggle(
                                "tabs-confirm-pane-toggle",
                                self.config.confirm_close_pane,
                                Preference::ConfirmClosePane(!self.config.confirm_close_pane),
                                cx,
                            )),
                    )
                    .row(
                        ui::row(
                            look,
                            "tabs-confirm-workspace",
                            "Confirm before closing a workspace",
                        )
                        .control(self.preference_toggle(
                            "tabs-confirm-workspace-toggle",
                            self.config.confirm_close_workspace,
                            Preference::ConfirmCloseWorkspace(!self.config.confirm_close_workspace),
                            cx,
                        )),
                    ),
            )
            .child(
                ui::group(look, "panes", "Panes", None).row(
                    ui::row(look, "tabs-focus-follows-mouse", "Focus follows mouse")
                        .description(
                            "The pointer chooses the active pane within a split terminal tab.",
                        )
                        .control(self.preference_toggle(
                            "tabs-focus-follows-mouse-toggle",
                            self.config.focus_follows_mouse,
                            Preference::FocusFollowsMouse(!self.config.focus_follows_mouse),
                            cx,
                        )),
                ),
            );
        let mut restore = ui::group(look, "session-restore", "Session restore", None).row(
            self.gate_shared(
                ui::row(
                    look,
                    "tabs-pane-history",
                    "Restore scrollback after Herdr restarts",
                )
                .tag(ui::Source::Shared)
                .description(
                    "Experimental. Herdr saves pane output to session-history.json and \
                         replays it when the daemon restarts.",
                ),
            )
            .control(
                ui::toggle(look, "tabs-pane-history-toggle", history, ready).when(
                    ready,
                    |toggle| {
                        toggle.on_click(cx.listener(move |this, _, _, cx| {
                            this.save_shared(Edit::PaneHistory(!history), cx)
                        }))
                    },
                ),
            ),
        );
        let open = !self.quitting && self.closing.is_none();
        for (index, host) in self.remote_history.hosts().iter().enumerate() {
            let (checked, settled) = match &host.state {
                HostState::Ready(history) => (history.enabled, true),
                HostState::Saving(history) => (history.enabled, false),
                HostState::Loading | HostState::Failed(_) => (false, false),
            };
            let enabled = open && settled;
            let target = host.target.clone();
            let note = match &host.state {
                HostState::Loading => "Reading host config\u{2026}".to_owned(),
                HostState::Saving(_) => "Saving host config\u{2026}".to_owned(),
                HostState::Failed(error) => {
                    format!("{error}. Reload configuration to try again.")
                }
                HostState::Ready(_) => "Each SSH host keeps its own setting.".to_owned(),
            };
            let id = format!("tabs-pane-history-host-{index}");
            restore = restore.row(
                ui::row(look, id.clone(), host.label.clone())
                    .sub()
                    .description(note)
                    .control(
                        ui::toggle(look, format!("{id}-toggle"), checked, enabled).when(
                            enabled,
                            |toggle| {
                                toggle.on_click(cx.listener(move |this, _, _, cx| {
                                    this.save_remote_history(&target, !checked, cx);
                                }))
                            },
                        ),
                    ),
            );
        }
        page.child(restore)
    }
}
